# Phase 2B: Pool Callbacks + Lazy Connect

**Date:** 2026-04-06
**Status:** Approved

## Goal

Add lifecycle callbacks to the connection pool and support lazy initialization.

## Pool Callbacks

Three async callback hooks on `PoolConfig`:

```rust
type ConnectCallback = Arc<dyn Fn(&mut Connection) -> BoxFuture<'_, Result<()>> + Send + Sync>;
type AcquireCallback = Arc<dyn Fn(&mut Connection) -> BoxFuture<'_, Result<bool>> + Send + Sync>;
type ReleaseCallback = Arc<dyn Fn(&mut Connection) -> BoxFuture<'_, Result<bool>> + Send + Sync>;
```

### after_connect

Runs once per newly created connection. Use for session setup like `SET search_path`.

- Called after TCP + TLS + auth completes, before connection enters the pool
- Error → connection discarded, pool retries

### before_acquire

Runs before returning a connection from the pool. Return `false` to reject.

- Called after health check passes
- `false` → connection discarded, pool tries next idle or creates new
- Error → connection discarded

### after_release

Runs when a connection returns to the pool. Return `false` to discard.

- Called before connection enters idle queue
- `false` → connection closed instead of returned
- Error → connection discarded

### API

```rust
PoolConfig::new()
    .after_connect(|conn| Box::pin(async move {
        conn.execute("SET search_path TO myapp", &[]).await?;
        Ok(())
    }))
    .before_acquire(|conn| Box::pin(async move {
        Ok(!conn.is_broken())
    }))
    .after_release(|conn| Box::pin(async move {
        Ok(true)
    }))
```

## Lazy Connect

```rust
let pool = Pool::connect_lazy(config);  // synchronous, no I/O
let conn = pool.acquire().await?;       // first connection created here
```

- `connect_lazy()` initializes pool state (semaphore, config) but opens zero connections
- First `acquire()` triggers connection establishment
- `min_connections` background fill starts after first successful acquire

## Dependencies

Add `futures-core = "0.3"` for `BoxFuture` type alias.

## Files

- Modify: `crates/sentinel-driver/src/pool/config.rs` (callback fields + builder methods)
- Modify: `crates/sentinel-driver/src/pool/mod.rs` (call hooks at lifecycle points, add `connect_lazy`)
- Modify: `crates/sentinel-driver/Cargo.toml` (add futures-core)
- Test: `tests/core/pool_callbacks.rs`
