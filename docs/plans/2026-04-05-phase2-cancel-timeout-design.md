# Phase 2 Design: Cancel Query (#4) + Per-Query Timeout (#3)

**Date:** 2026-04-05
**Status:** Approved
**Issues:** cntm-labs/sentinel-driver#4, cntm-labs/sentinel-driver#3
**Depends on:** Phase 1 (array types + health check)

## Motivation

Production safety requires two capabilities:

1. **Cancel query** — abort long-running queries from another task without killing the connection
2. **Per-query timeout** — prevent queries from hanging indefinitely, with automatic cleanup

Issue #4 must be implemented before #3 because timeout uses cancel to clean up server-side.

## Issue #4: Cancel Query Using BackendKeyData

### CancelToken Design

Standalone struct holding everything needed to cancel a query. No reference to Connection — can be cloned and sent across tasks freely.

```rust
#[derive(Clone, Debug)]
pub struct CancelToken {
    host: String,
    port: u16,
    process_id: i32,
    secret_key: i32,
}
```

Implements `Clone + Send + Sync`. Duplicates host/port from Config (trivial cost).

### Cancel Protocol

Per PG spec, cancel opens a **new plain TCP connection** (no TLS, no startup handshake):

```
i32  length      = 16
i32  magic       = 80877102
i32  process_id
i32  secret_key
```

After sending, close the TCP connection immediately. The server will attempt to cancel the running query on the connection identified by `(process_id, secret_key)`. Cancellation is best-effort — the server may or may not honor it.

### TLS Decision

Always use plain TCP for cancel, even if the main connection uses TLS. Rationale:
- PG accepts cancel on plain TCP regardless of main connection TLS status
- Cancel is fire-and-forget (16 bytes, no response)
- TLS handshake adds ~5ms latency to what should be instant
- `secret_key` already prevents unauthorized cancel
- Consistent with libpq, tokio-postgres, and all major drivers

### Public API

```rust
// New public type
pub struct CancelToken { ... }

impl CancelToken {
    /// Send cancel request. Opens new TCP connection, sends CancelRequest, closes.
    /// Best-effort — server may not cancel immediately.
    pub async fn cancel(&self) -> Result<()>;
}

// On Connection
impl Connection {
    /// Get a cancel token for this connection.
    pub fn cancel_token(&self) -> CancelToken;
}
```

### Files Changed

- Create: `crates/sentinel-driver/src/cancel.rs` — CancelToken struct + cancel() impl
- Modify: `crates/sentinel-driver/src/protocol/frontend.rs` — add `cancel_request()` encoder
- Modify: `crates/sentinel-driver/src/lib.rs` — remove `_` prefix from `secret_key`, add `cancel_token()` method, re-export CancelToken, add `pub mod cancel`

## Issue #3: Per-Query Timeout

### Timeout Strategy

Client-side timeout using `tokio::time::timeout` wrapping the query future. On timeout:

1. Send cancel request via `CancelToken::cancel()` (best-effort, don't block on result)
2. Mark connection as broken (cannot safely reuse — response stream is in unknown state)
3. Return `Error::Timeout`

### Config-Level Default

`Config` already has `_statement_timeout: Option<Duration>` (unused, with `_` prefix). Activate it:

- Remove `_` prefix → `statement_timeout`
- Store in `Connection` as `query_timeout: Option<Duration>`
- Apply as default client-side timeout for all queries
- Parse `statement_timeout` from connection string query params

### Per-Query Override

```rust
impl Connection {
    pub async fn query_with_timeout(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        timeout: Duration,
    ) -> Result<Vec<Row>>;

    pub async fn execute_with_timeout(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        timeout: Duration,
    ) -> Result<u64>;
}
```

### Internal Flow

```
query_with_timeout(sql, params, timeout)
  │
  ├─ create cancel_token (cheap, just copies fields)
  ├─ tokio::time::timeout(timeout, query_internal(sql, params))
  │    ├─ Ok(result) → return result
  │    └─ Err(elapsed) →
  │         ├─ tokio::spawn(cancel_token.cancel())  // fire-and-forget
  │         ├─ mark connection broken internally
  │         └─ return Error::Timeout("query timeout after {timeout}")
  │
query(sql, params)  // uses default timeout
  │
  ├─ if self.query_timeout.is_some() →
  │     query_with_timeout(sql, params, self.query_timeout)
  └─ else →
        query_internal(sql, params)  // no timeout, current behavior
```

### Connection Broken After Timeout

After a timeout, the connection is in an unknown protocol state (server may still be sending DataRow messages). The connection **must** be marked broken:

- Direct `Connection`: user gets `Error::Timeout`, subsequent calls on this connection will fail
- Pooled `Connection`: pool discards it on return, creates fresh one next acquire

### Files Changed

- Modify: `crates/sentinel-driver/src/config.rs` — activate `statement_timeout` field (remove `_` prefix), add accessor, add connection string parsing
- Modify: `crates/sentinel-driver/src/lib.rs` — add `query_timeout` field to Connection, implement `query_with_timeout()`, `execute_with_timeout()`, modify `query()`/`execute()` to use default timeout
- Modify: `crates/sentinel-driver/src/error.rs` — no changes needed (Error::Timeout already exists)

## Implementation Order

```
#4 Cancel Query
  ├── Task 1: cancel_request() encoder in frontend.rs
  ├── Task 2: CancelToken struct + cancel() in cancel.rs
  └── Task 3: Connection::cancel_token() in lib.rs

#3 Per-Query Timeout
  ├── Task 4: Activate statement_timeout in config.rs
  ├── Task 5: query_with_timeout() + execute_with_timeout() in lib.rs
  └── Task 6: Default timeout integration (query/execute use config default)
```
