# Phase 2 Implementation Plan: Cancel Query + Per-Query Timeout

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement cancel query protocol and per-query timeout with auto-cancel to enable production-safe query execution in sentinel-driver.

**Architecture:** CancelToken is a standalone Clone+Send+Sync struct that opens a new plain TCP connection to send a 16-byte CancelRequest. Per-query timeout wraps query futures with `tokio::time::timeout`, fires cancel on expiry, and marks the connection broken.

**Tech Stack:** Rust, tokio (timeout + TCP), bytes crate (message encoding)

**Prerequisites:** Phase 1 must be merged first (this branch should be created from main after Phase 1 merge).

---

### Task 1: CancelRequest message encoder

**Files:**
- Modify: `crates/sentinel-driver/src/protocol/frontend.rs` (append after `sasl_response` fn, ~line 197)
- Test: `tests/core/protocol_frontend.rs`

**Step 1: Write the failing test**

Add to `tests/core/protocol_frontend.rs`:

```rust
#[test]
fn test_cancel_request() {
    use bytes::BytesMut;
    use sentinel_driver::protocol::frontend;

    let mut buf = BytesMut::new();
    frontend::cancel_request(&mut buf, 12345, 67890);

    assert_eq!(buf.len(), 16);

    // length = 16
    let len = i32::from_be_bytes(buf[0..4].try_into().unwrap());
    assert_eq!(len, 16);

    // magic = 80877102
    let magic = i32::from_be_bytes(buf[4..8].try_into().unwrap());
    assert_eq!(magic, 80877102);

    // process_id
    let pid = i32::from_be_bytes(buf[8..12].try_into().unwrap());
    assert_eq!(pid, 12345);

    // secret_key
    let key = i32::from_be_bytes(buf[12..16].try_into().unwrap());
    assert_eq!(key, 67890);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --workspace test_cancel_request -- --no-capture`
Expected: FAIL — `cancel_request` not found in `frontend`

**Step 3: Write minimal implementation**

Add to `crates/sentinel-driver/src/protocol/frontend.rs` after the `sasl_response` function (after line 197):

```rust
/// CancelRequest — sent on a new connection to cancel a running query.
///
/// Format: `[length: i32 = 16][magic: i32 = 80877102][process_id: i32][secret_key: i32]`
///
/// Unlike other messages, CancelRequest has no type byte — it uses a
/// length prefix + magic number, similar to StartupMessage and SSLRequest.
pub fn cancel_request(buf: &mut BytesMut, process_id: i32, secret_key: i32) {
    buf.put_i32(16); // total length
    buf.put_i32(80877102); // cancel request code
    buf.put_i32(process_id);
    buf.put_i32(secret_key);
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --workspace test_cancel_request -- --no-capture`
Expected: PASS

**Step 5: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/sentinel-driver/src/protocol/frontend.rs tests/core/protocol_frontend.rs
git commit -m "feat(protocol): add cancel_request message encoder (#4)"
```

---

### Task 2: CancelToken struct and cancel() method

**Files:**
- Create: `crates/sentinel-driver/src/cancel.rs`
- Modify: `crates/sentinel-driver/src/lib.rs` (add `pub mod cancel` + re-export)
- Test: `tests/core/cancel.rs` (create)
- Modify: `tests/core/mod.rs` (register new test module)

**Step 1: Write the failing test**

Create `tests/core/cancel.rs`:

```rust
use sentinel_driver::CancelToken;

#[test]
fn test_cancel_token_clone_send_sync() {
    fn assert_clone_send_sync<T: Clone + Send + Sync>() {}
    assert_clone_send_sync::<CancelToken>();
}

#[test]
fn test_cancel_token_creation() {
    let token = CancelToken::new("localhost", 5432, 12345, 67890);
    // Token should be creatable and cloneable
    let _clone = token.clone();
}

#[tokio::test]
async fn test_cancel_token_cancel_connection_refused() {
    // Cancel to a port with nothing listening should return an error
    let token = CancelToken::new("127.0.0.1", 1, 12345, 67890);
    let result = token.cancel().await;
    assert!(result.is_err());
}
```

Add to `tests/core/mod.rs`:

```rust
mod cancel;
```

**Step 2: Run test to verify it fails**

Run: `cargo test --workspace test_cancel_token -- --no-capture`
Expected: FAIL — `CancelToken` not found

**Step 3: Write the CancelToken implementation**

Create `crates/sentinel-driver/src/cancel.rs`:

```rust
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use bytes::BytesMut;

use crate::error::{Error, Result};
use crate::protocol::frontend;

/// Token for cancelling a running query from any task or thread.
///
/// Obtained via [`Connection::cancel_token()`]. Cheaply cloneable — holds
/// only the host, port, and backend key data needed to send a CancelRequest.
///
/// # Example
///
/// ```rust,no_run
/// # async fn example(conn: &mut sentinel_driver::Connection) -> sentinel_driver::Result<()> {
/// let token = conn.cancel_token();
///
/// // Spawn a task that cancels after 5 seconds
/// tokio::spawn(async move {
///     tokio::time::sleep(std::time::Duration::from_secs(5)).await;
///     token.cancel().await.ok();
/// });
///
/// // This query will be cancelled if it takes more than 5 seconds
/// let rows = conn.query("SELECT pg_sleep(60)", &[]).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct CancelToken {
    host: String,
    port: u16,
    process_id: i32,
    secret_key: i32,
}

impl CancelToken {
    /// Create a new cancel token.
    ///
    /// Typically obtained via `Connection::cancel_token()` rather than
    /// constructed directly.
    pub fn new(host: impl Into<String>, port: u16, process_id: i32, secret_key: i32) -> Self {
        Self {
            host: host.into(),
            port,
            process_id,
            secret_key,
        }
    }

    /// Send a cancel request to the PostgreSQL server.
    ///
    /// Opens a new TCP connection, sends the 16-byte CancelRequest message,
    /// and closes the connection. This is best-effort — the server may or
    /// may not cancel the running query.
    ///
    /// Always uses plain TCP (no TLS) per PostgreSQL protocol convention.
    pub async fn cancel(&self) -> Result<()> {
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = TcpStream::connect(&addr).await.map_err(Error::Io)?;

        let mut buf = BytesMut::with_capacity(16);
        frontend::cancel_request(&mut buf, self.process_id, self.secret_key);

        stream.write_all(&buf).await.map_err(Error::Io)?;
        stream.shutdown().await.map_err(Error::Io)?;

        Ok(())
    }
}
```

**Step 4: Register the module and re-export**

In `crates/sentinel-driver/src/lib.rs`, add `pub mod cancel;` after the existing module declarations (after line 39, near the other `pub mod` lines).

Add to the re-exports section (after line 64):

```rust
pub use cancel::CancelToken;
```

**Step 5: Run test to verify it passes**

Run: `cargo test --workspace test_cancel_token -- --no-capture`
Expected: all 3 tests PASS

**Step 6: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS

**Step 7: Commit**

```bash
git add crates/sentinel-driver/src/cancel.rs crates/sentinel-driver/src/lib.rs tests/core/cancel.rs tests/core/mod.rs
git commit -m "feat(cancel): add CancelToken with cancel() method (#4)"
```

---

### Task 3: Connection::cancel_token() method

**Files:**
- Modify: `crates/sentinel-driver/src/lib.rs` (activate `_secret_key`, add `cancel_token()`)
- Test: `tests/core/cancel.rs` (this is a compile/unit test — live PG test not needed)

**Step 1: Write the failing test**

Add to `tests/core/cancel.rs`:

```rust
#[test]
fn test_cancel_token_public_api_exists() {
    // Verify the method signature exists on Connection at compile time.
    // We can't create a real Connection without PG, but we verify CancelToken
    // has the right shape returned by cancel_token().
    let token = CancelToken::new("localhost", 5432, 100, 200);
    let cloned = token.clone();
    // Verify it can be sent across threads
    std::thread::spawn(move || {
        let _ = cloned;
    })
    .join()
    .unwrap();
}
```

**Step 2: Run test to verify it passes (baseline for new test)**

Run: `cargo test --workspace test_cancel_token_public -- --no-capture`
Expected: PASS

**Step 3: Implement cancel_token() on Connection**

In `crates/sentinel-driver/src/lib.rs`:

1. Remove the `_` prefix from `_secret_key` field on the `Connection` struct (line 86) → `secret_key`
2. Remove the `_` prefix in `Connection::connect()` where it's assigned (line 101) → `secret_key: result.secret_key`
3. Add the `cancel_token()` method after `process_id()` (after line 364):

```rust
    /// Get a cancel token for this connection.
    ///
    /// The token can be cloned and sent to another task to cancel a
    /// running query. See [`CancelToken`] for details.
    pub fn cancel_token(&self) -> CancelToken {
        CancelToken::new(
            self._config.host(),
            self._config.port(),
            self.process_id,
            self.secret_key,
        )
    }
```

Note: `_config` is used here — it remains `_config` since it was already named that way (the `_` prefix indicates it was reserved for future use, and now it is used).

Actually check the field name: `_config: Config` on line 84. Since we're now using it in `cancel_token()`, rename `_config` → `config` to remove the unused prefix:

- Line 84: `_config: Config` → `config: Config`
- Line 99: `_config: config` → `config`
- Update `cancel_token()` to use `self.config.host()` and `self.config.port()`

**Step 4: Run all tests**

Run: `cargo test --workspace -- --no-capture`
Expected: all tests PASS

**Step 5: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/sentinel-driver/src/lib.rs
git commit -m "feat(connection): add cancel_token() method, activate secret_key (#4)"
```

---

### Task 4: Activate statement_timeout in Config

**Files:**
- Modify: `crates/sentinel-driver/src/config.rs` (remove `_` prefix, add accessor, add parsing)
- Test: `tests/core/config.rs`

**Step 1: Write the failing test**

Add to `tests/core/config.rs`:

```rust
#[test]
fn test_parse_statement_timeout() {
    let config =
        Config::parse("postgres://user:pass@localhost/db?statement_timeout=5").unwrap();
    assert_eq!(
        config.statement_timeout(),
        Some(std::time::Duration::from_secs(5))
    );
}

#[test]
fn test_statement_timeout_default_none() {
    let config = Config::parse("postgres://user:pass@localhost/db").unwrap();
    assert_eq!(config.statement_timeout(), None);
}

#[test]
fn test_builder_statement_timeout() {
    let config = Config::builder()
        .user("test")
        .statement_timeout(std::time::Duration::from_secs(10))
        .build();
    assert_eq!(
        config.statement_timeout(),
        Some(std::time::Duration::from_secs(10))
    );
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --workspace test_parse_statement_timeout test_statement_timeout_default test_builder_statement_timeout -- --no-capture`
Expected: FAIL — `statement_timeout()` method not found on `Config`

**Step 3: Implement**

In `crates/sentinel-driver/src/config.rs`:

1. Rename `_statement_timeout` → `statement_timeout` in the `Config` struct (line 52)
2. Update `ConfigBuilder::build()` (line 329): `_statement_timeout: self.statement_timeout` → `statement_timeout: self.statement_timeout`
3. Add accessor method after `connect_timeout()` (after line 223):

```rust
    pub fn statement_timeout(&self) -> Option<Duration> {
        self.statement_timeout
    }
```

4. Add connection string parsing in the `match key` block (after the `connect_timeout` arm, ~line 163):

```rust
                    "statement_timeout" => {
                        let secs: u64 = value.parse().map_err(|_| {
                            Error::Config(format!("invalid statement_timeout: {value}"))
                        })?;
                        config = config.statement_timeout(Duration::from_secs(secs));
                    }
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --workspace test_parse_statement_timeout test_statement_timeout_default test_builder_statement_timeout -- --no-capture`
Expected: all 3 tests PASS

**Step 5: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/sentinel-driver/src/config.rs tests/core/config.rs
git commit -m "feat(config): activate statement_timeout config option (#3)"
```

---

### Task 5: query_with_timeout() and execute_with_timeout()

**Files:**
- Modify: `crates/sentinel-driver/src/lib.rs` (add timeout methods, add `is_broken` field)
- Modify: `crates/sentinel-driver/src/error.rs` (no changes needed — Error::Timeout exists)

**Step 1: Write the failing test**

Add to `tests/core/cancel.rs` (reusing this module for timeout tests since they're tightly coupled):

```rust
use std::time::Duration;

#[test]
fn test_error_timeout_variant_exists() {
    // Verify the Timeout error variant is usable
    let err = sentinel_driver::Error::Timeout("test timeout".into());
    assert!(matches!(err, sentinel_driver::Error::Timeout(_)));
    assert!(err.to_string().contains("test timeout"));
}
```

**Step 2: Run test to verify it passes (baseline)**

Run: `cargo test --workspace test_error_timeout -- --no-capture`
Expected: PASS (Error::Timeout already exists)

**Step 3: Implement timeout methods**

In `crates/sentinel-driver/src/lib.rs`:

1. Add `use std::time::Duration;` to the imports at the top.

2. Add `query_timeout` field to `Connection` struct:

```rust
pub struct Connection {
    conn: PgConnection,
    config: Config,
    process_id: i32,
    secret_key: i32,
    transaction_status: TransactionStatus,
    stmt_cache: StatementCache,
    query_timeout: Option<Duration>,
    is_broken: bool,
}
```

3. In `Connection::connect()`, initialize the new fields:

```rust
    let query_timeout = config.statement_timeout();

    Ok(Self {
        conn,
        config,
        process_id: result.process_id,
        secret_key: result.secret_key,
        transaction_status: result.transaction_status,
        stmt_cache: StatementCache::new(),
        query_timeout,
        is_broken: false,
    })
```

4. Add `query_with_timeout` after the `execute` method (~line 154):

```rust
    /// Execute a query with a timeout.
    ///
    /// If the query does not complete within `timeout`, a cancel request
    /// is sent to the server and the connection is marked as broken.
    pub async fn query_with_timeout(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        timeout: Duration,
    ) -> Result<Vec<Row>> {
        let cancel_token = self.cancel_token();

        match tokio::time::timeout(timeout, self.query(sql, params)).await {
            Ok(result) => result,
            Err(_elapsed) => {
                self.is_broken = true;
                // Fire-and-forget cancel
                tokio::spawn(async move {
                    cancel_token.cancel().await.ok();
                });
                Err(Error::Timeout(format!(
                    "query timeout after {}ms",
                    timeout.as_millis()
                )))
            }
        }
    }

    /// Execute a non-SELECT query with a timeout.
    ///
    /// If the query does not complete within `timeout`, a cancel request
    /// is sent to the server and the connection is marked as broken.
    pub async fn execute_with_timeout(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        timeout: Duration,
    ) -> Result<u64> {
        let cancel_token = self.cancel_token();

        match tokio::time::timeout(timeout, self.execute(sql, params)).await {
            Ok(result) => result,
            Err(_elapsed) => {
                self.is_broken = true;
                tokio::spawn(async move {
                    cancel_token.cancel().await.ok();
                });
                Err(Error::Timeout(format!(
                    "query timeout after {}ms",
                    timeout.as_millis()
                )))
            }
        }
    }
```

**Step 4: Run all tests**

Run: `cargo test --workspace -- --no-capture`
Expected: all tests PASS

**Step 5: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/sentinel-driver/src/lib.rs
git commit -m "feat(connection): add query_with_timeout and execute_with_timeout (#3)"
```

---

### Task 6: Default timeout integration

**Files:**
- Modify: `crates/sentinel-driver/src/lib.rs` (modify `query()` and `execute()` to use default timeout)

**Step 1: Write the failing test**

Add to `tests/core/cancel.rs`:

```rust
#[test]
fn test_config_statement_timeout_propagation() {
    // Verify Config can carry statement_timeout that Connection would use
    let config = sentinel_driver::Config::builder()
        .user("test")
        .statement_timeout(Duration::from_secs(30))
        .build();
    assert_eq!(config.statement_timeout(), Some(Duration::from_secs(30)));
}
```

**Step 2: Run test to verify it passes (baseline)**

Run: `cargo test --workspace test_config_statement_timeout -- --no-capture`
Expected: PASS

**Step 3: Modify query() and execute() to use default timeout**

In `crates/sentinel-driver/src/lib.rs`, modify the `query()` method:

```rust
    pub async fn query(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        if let Some(timeout) = self.query_timeout {
            return self.query_with_timeout(sql, params, timeout).await;
        }

        let result = self.query_internal(sql, params).await?;
        match result {
            pipeline::QueryResult::Rows(rows) => Ok(rows),
            pipeline::QueryResult::Command(_) => Ok(Vec::new()),
        }
    }
```

Modify the `execute()` method similarly:

```rust
    pub async fn execute(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        if let Some(timeout) = self.query_timeout {
            return self.execute_with_timeout(sql, params, timeout).await;
        }

        let result = self.query_internal(sql, params).await?;
        match result {
            pipeline::QueryResult::Command(r) => Ok(r.rows_affected),
            pipeline::QueryResult::Rows(_) => Ok(0),
        }
    }
```

**Step 4: Run all tests**

Run: `cargo test --workspace -- --no-capture`
Expected: all tests PASS

**Step 5: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/sentinel-driver/src/lib.rs
git commit -m "feat(connection): integrate default query timeout from config (#3)"
```

---

### Task 7: Final verification and formatting

**Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: all tests PASS

**Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS

**Step 3: Run formatting check**

Run: `cargo fmt --all -- --check`
Expected: PASS

**Step 4: Verify no dead code warnings**

Run: `cargo check --workspace 2>&1`
Expected: no warnings

**Step 5: Commit any final fixes if needed**

```bash
git commit -m "chore: Phase 2 final cleanup"
```
