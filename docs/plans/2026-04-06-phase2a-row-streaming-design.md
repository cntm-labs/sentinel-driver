# Phase 2A: Row Streaming Design

**Date:** 2026-04-06
**Status:** Approved

## Goal

Add row-by-row streaming for query results so large result sets don't need to be fully materialized in memory.

## Approach

Two-phase implementation:
1. **Lending iterator** — `RowStream<'a>` with `async fn next()` (this phase)
2. **`futures::Stream` adapter** — wraps RowStream with `impl Stream<Item>` (future phase)

## Design: RowStream<'a>

### API

```rust
// Create a streaming query
let mut stream = conn.query_stream("SELECT * FROM users", &[]).await?;

// Consume rows one by one
while let Some(row) = stream.next().await? {
    let name: String = row.get(0);
}
```

### Struct

```rust
pub struct RowStream<'a> {
    conn: &'a mut PgConnection,
    description: Arc<RowDescription>,
    done: bool,
}
```

### Lifecycle

1. **`query_stream(sql, params)`** sends Parse + Bind + Describe + Execute + Sync
2. Reads ParseComplete, BindComplete, RowDescription
3. Returns `RowStream` holding `&mut conn` and the description
4. **`stream.next()`** calls `conn.recv()`:
   - `DataRow` → construct `Row`, return `Ok(Some(row))`
   - `CommandComplete` → read `ReadyForQuery`, set `done = true`, return `Ok(None)`
   - `ErrorResponse` → set `done = true`, return `Err`
5. **Drop** — if `!done`, drain remaining messages up to ReadyForQuery to keep connection clean

### Connection Locking

`&'a mut PgConnection` enforces exclusive access at compile time. No other queries can run while the stream is open. This is the correct semantic — PG sends DataRow messages sequentially on the same connection.

### Error Handling

- `ErrorResponse` from server → return `Err(Error::Server(...))`, mark done
- Connection I/O error → return `Err(Error::Io(...))`, mark connection broken
- Drop without consuming → drain silently (log warning via tracing)

### Files

- Create: `crates/sentinel-driver/src/stream.rs`
- Modify: `crates/sentinel-driver/src/lib.rs` (add `query_stream()`, re-export `RowStream`)
- Modify: `crates/sentinel-driver/src/connection/stream.rs` (if needed for drain helper)
- Test: `tests/core/stream.rs`

### No New Dependencies

Uses the same `conn.recv()` loop as pipeline/copy — no `futures-core` needed for phase 1.

### Relationship to Pool

`Pool::query_stream()` acquires a `PooledConnection` and returns a `RowStream` that holds ownership of the pooled connection for its lifetime. The connection returns to the pool when the stream is dropped.
