# sentinel-driver — Design Document

> High-performance PostgreSQL wire protocol driver for Rust.
> Foundation layer for Sentinel ORM.

**Date:** 2026-04-03
**Status:** Approved
**Author:** mrbt + Claude

---

## Overview

sentinel-driver is a standalone Rust crate implementing the PostgreSQL wire protocol v3.
It serves as the database driver for Sentinel ORM but can be used independently.

### Why build a new driver instead of using sqlx or tokio-postgres?

**sqlx problems:**
- Compile-time macros require live DB connection (builds hang 2+ hours)
- 3 soundness bugs in SQLite (unsafe code in safe APIs)
- SCRAM auth does not SASLprep passwords (security issue)
- Multi-database abstraction tax (PG features are second-class)
- cargo sqlx prepare requires cargo clean

**tokio-postgres limitations:**
- Two-task architecture adds channel overhead (~1-2 us/query)
- No built-in connection pooling
- No automatic prepared statement caching
- No built-in pipeline abstraction

**sentinel-driver advantages:**
- PG-only = every PG feature is first-class
- Single-task architecture (no channel overhead)
- Pipeline mode as core architecture
- COPY protocol built-in
- LISTEN/NOTIFY as first-class (foundation for realtime Layer 2)
- Correct SCRAM-SHA-256 with SASLprep
- Zero-copy parsing for large values
- No live DB needed at compile time

---

## Architecture

```
sentinel-driver/
|- src/
|  |- lib.rs              — Public API
|  |- config.rs           — Connection configuration
|  |- error.rs            — Error types
|  |- protocol/
|  |  |- mod.rs           — Protocol types
|  |  |- frontend.rs      — Client-to-server messages
|  |  |- backend.rs       — Server-to-client messages
|  |  |- codec.rs         — Encoder/decoder (zero-copy)
|  |- connection/
|  |  |- mod.rs           — Connection struct
|  |  |- stream.rs        — TCP/TLS stream
|  |  |- startup.rs       — Handshake + auth
|  |- auth/
|  |  |- mod.rs           — Auth dispatcher
|  |  |- scram.rs         — SCRAM-SHA-256 (correct SASLprep)
|  |  |- md5.rs           — MD5 (legacy support)
|  |- pool/
|  |  |- mod.rs           — Connection pool
|  |  |- config.rs        — Pool configuration
|  |  |- health.rs        — Connection health check
|  |- pipeline/
|  |  |- mod.rs           — Pipeline engine
|  |  |- batch.rs         — Automatic batching
|  |- copy/
|  |  |- mod.rs           — COPY IN/OUT
|  |  |- binary.rs        — Binary COPY format
|  |  |- text.rs          — Text COPY format
|  |- notify/
|  |  |- mod.rs           — LISTEN/NOTIFY
|  |  |- channel.rs       — Notification dispatch
|  |- types/
|  |  |- mod.rs           — Type registry
|  |  |- encode.rs        — Rust to PG (binary)
|  |  |- decode.rs        — PG to Rust (binary)
|  |  |- builtin.rs       — Built-in types (int, text, uuid, etc.)
|  |- tls/
|  |  |- mod.rs           — TLS wrapper (rustls)
|  |- row.rs              — Row type (zero-copy column access)
|  |- statement.rs        — Prepared statement
|  |- transaction.rs      — Transaction wrapper
|- sentinel-driver-derive/
|  |- lib.rs              — FromRow, ToSql, FromSql proc macros
```

---

## Wire Protocol Implementation

### Message Format

PG wire protocol v3 messages:
- Type byte (1 byte) + Length (4 bytes, includes self) + Payload
- Frontend messages: Query(Q), Parse(P), Bind(B), Execute(E), Sync(S), etc.
- Backend messages: DataRow(D), RowDescription(T), CommandComplete(C), etc.

### Zero-Copy Parsing

```rust
// Use bytes::Bytes for zero-copy column values
pub struct DataRow {
    buf: Bytes,           // original message buffer
    columns: Vec<Range>,  // column offsets into buf
}

impl DataRow {
    pub fn get_bytes(&self, idx: usize) -> Option<Bytes> {
        let range = &self.columns[idx];
        Some(self.buf.slice(range.start..range.end))  // zero-copy
    }
}
```

Hybrid approach: zero-copy for values >64 bytes, inline copy for smaller values.

### Binary Encoding

All types use PG binary format by default:
- int4: 4 bytes big-endian (not ASCII)
- timestamp: 8 bytes int64 (microseconds since PG epoch)
- uuid: 16 bytes raw
- text/varchar: raw UTF-8 bytes (same as text format)

Binary encoding is 15-40% faster for non-text types.

---

## Pipeline Mode

Auto-pipeline: if query B is sent while query A response has not arrived,
automatically pipeline B behind A.

```rust
// User code — looks like sequential queries
let users = db.query("SELECT * FROM users WHERE active = $1", &[&true]).await?;
let posts = db.query("SELECT * FROM posts WHERE published = $1", &[&true]).await?;

// Under the hood — pipelined into single round-trip:
// Send: Parse1/Bind1/Execute1/Parse2/Bind2/Execute2/Sync
// Recv: all responses in order
```

Explicit pipeline API:
```rust
let (users, posts) = db.pipeline(|p| {
    let u = p.query("SELECT * FROM users WHERE active = $1", &[&true]);
    let po = p.query("SELECT * FROM posts WHERE published = $1", &[&true]);
    (u, po)
}).await?;
```

---

## COPY Protocol

```rust
// Bulk insert via COPY (10-50x faster than INSERT)
let mut copy = db.copy_in("COPY users (name, email) FROM STDIN (FORMAT binary)").await?;

for user in &users {
    copy.write_row(&[&user.name, &user.email]).await?;
}

let count = copy.finish().await?;  // returns rows inserted

// Streaming COPY OUT
let mut stream = db.copy_out("COPY users TO STDOUT (FORMAT binary)").await?;
while let Some(row) = stream.next().await? {
    let name: String = row.get(0);
    let email: String = row.get(1);
}
```

---

## Connection Pool

```rust
let pool = Pool::builder()
    .max_connections(num_cpus::get() * 2)
    .min_connections(2)
    .connect_timeout(Duration::from_secs(5))
    .idle_timeout(Duration::from_secs(600))
    .health_check(HealthCheck::Fast)  // flag-based, not query
    .build("postgres://user:pass@localhost/db")
    .await?;

let conn = pool.acquire().await?;  // <0.5 us checkout
// conn auto-returned to pool on drop
```

---

## Prepared Statement Cache

Two-tier cache:
1. Tier 1: HashMap for known queries (registered at startup or by ORM macros)
2. Tier 2: LRU-256 for ad-hoc queries

```rust
// Tier 1: pre-register known queries
pool.register_statement("find_user", "SELECT * FROM users WHERE id = $1").await?;

// Tier 2: automatic caching for ad-hoc
let rows = conn.query("SELECT * FROM users WHERE email = $1", &[&email]).await?;
// First call: Parse + cache. Subsequent calls: reuse cached statement.
```

Cache metrics exposed:
```rust
let metrics = pool.cache_metrics();
// CacheMetrics { tier1_hits: 15000, tier2_hits: 800, misses: 12, evictions: 0 }
```

---

## LISTEN/NOTIFY

```rust
// Subscribe to PG notifications
let mut listener = db.listen("user_changes").await?;

while let Some(notification) = listener.recv().await {
    println!("Channel: {}, Payload: {}", notification.channel, notification.payload);
}

// Multiple channels
let mut listener = db.listen_many(&["user_changes", "order_updates"]).await?;

// Notify
db.notify("user_changes", &serde_json::to_string(&event)?).await?;
```

Dedicated connection for subscriptions with auto-reconnect.

---

## Auth

SCRAM-SHA-256 implementation with correct SASLprep (RFC 7613):
- Normalize passwords before hashing
- Support channel binding (SCRAM-SHA-256-PLUS) when TLS active
- MD5 auth for legacy support (with deprecation warning log)
- No cleartext auth unless explicitly opted in

---

## Performance Targets

| Metric | sqlx | tokio-postgres | sentinel-driver |
|--------|------|----------------|-----------------|
| Simple SELECT | 75K q/s | 85K q/s | 90K+ q/s |
| Batch 100 queries | 3K/s | 8K/s | 15K+ batch/s |
| Bulk INSERT 10K rows | 50K/s | 200K/s | 500K+ rows/s |
| Pool checkout | 0.8 us | N/A (no pool) | <0.5 us |
| Per-query overhead | ~13 us | ~11 us | <10 us |

Key advantages over sqlx:
- Single-task (no channel), zero-copy, pipeline-first
- COPY protocol (sqlx does not support)

Key advantages over tokio-postgres:
- Built-in pool, stmt cache, pipeline abstraction
- Single-task (no Client+Connection split)

---

## Dependencies

Minimal dependency tree:
- tokio (async runtime)
- bytes (zero-copy buffers)
- rustls + webpki-roots (TLS)
- sha2, hmac (SCRAM auth)
- stringprep (SASLprep for SCRAM)
- chrono, uuid (common PG types)
- thiserror (error types)

No sqlx, no openssl, no libpq.
