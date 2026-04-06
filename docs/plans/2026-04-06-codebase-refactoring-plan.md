# Codebase Refactoring Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Restructure the codebase so every file has one clear responsibility, all tests live in `tests/core/` with subdirectories, and lib.rs is only re-exports.

**Architecture:** Pure structural refactoring — move code between files, no logic changes. Every step must pass `cargo test --workspace`. The Connection struct splits into focused impl-block files. types/mod.rs splits into oid.rs + traits.rs. All 27 local tests move to tests/core/ subdirectories.

**Tech Stack:** Rust, cargo test, cargo clippy, cargo fmt

**Important:** This plan references the design doc at `docs/plans/2026-04-06-codebase-refactoring-design.md`. Read it first for full context.

**Verification after EVERY task:**
```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

---

## Task 1: Split types/mod.rs into oid.rs + traits.rs

This is the safest starting point — types/ is self-contained with no circular deps.

**Files:**
- Create: `crates/sentinel-driver/src/types/oid.rs`
- Create: `crates/sentinel-driver/src/types/traits.rs`
- Modify: `crates/sentinel-driver/src/types/mod.rs`

**Step 1: Create `types/oid.rs`**

Move the `Oid` struct, all OID constants, and `From` impls from `types/mod.rs` into a new file `types/oid.rs`:

```rust
/// Well-known PostgreSQL type OIDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Oid(pub u32);

impl Oid {
    // ALL existing pub const lines — copy exactly as-is from mod.rs
    pub const BOOL: Oid = Oid(16);
    // ... every single constant ...
}

impl From<u32> for Oid {
    fn from(v: u32) -> Self {
        Oid(v)
    }
}

impl From<Oid> for u32 {
    fn from(oid: Oid) -> Self {
        oid.0
    }
}
```

**Step 2: Create `types/traits.rs`**

Move `ToSql`, `FromSql` trait definitions, `Option<T>` impls, and `encode_param` helpers from `types/mod.rs`:

```rust
use crate::types::Oid;
use bytes::BytesMut;
use crate::error::Result;

/// Encode a Rust value into PostgreSQL binary format.
pub trait ToSql {
    fn oid(&self) -> Oid;
    fn to_sql(&self, buf: &mut BytesMut) -> Result<()>;
    fn to_sql_vec(&self) -> Result<Vec<u8>> {
        let mut buf = BytesMut::new();
        self.to_sql(&mut buf)?;
        Ok(buf.to_vec())
    }
}

/// Decode a Rust value from PostgreSQL binary format.
pub trait FromSql: Sized {
    fn oid() -> Oid;
    fn from_sql(buf: &[u8]) -> Result<Self>;
    fn from_sql_nullable(buf: Option<&[u8]>) -> Result<Self> {
        // ... existing code exactly as-is ...
    }
}

// Option<T> impls — copy exactly as-is
// encode_param, encode_param_nullable — copy exactly as-is
```

**Step 3: Rewrite `types/mod.rs` to just re-export**

```rust
pub mod builtin;
pub mod decode;
pub mod encode;
pub mod geometric;
pub mod interval;
pub mod lsn;
pub mod money;
pub mod network;
#[cfg(feature = "with-rust-decimal")]
pub mod numeric;
pub mod oid;
pub mod range;
pub mod traits;
pub mod xml;

// Re-export for backwards compatibility — all existing code uses `types::Oid`, `types::ToSql`, etc.
pub use oid::Oid;
pub use traits::{encode_param, encode_param_nullable, FromSql, ToSql};
```

**Step 4: Verify**

Run: `cargo test --workspace`
Run: `cargo clippy --workspace -- -D warnings`
Expected: All tests pass, no warnings. All existing `use crate::types::{Oid, ToSql, FromSql}` still work because of re-exports.

**Step 5: Commit**

```bash
git add crates/sentinel-driver/src/types/oid.rs \
       crates/sentinel-driver/src/types/traits.rs \
       crates/sentinel-driver/src/types/mod.rs
git commit -m "refactor(types): split mod.rs into oid.rs + traits.rs"
```

---

## Task 2: Split types_misc.rs into types_xml.rs + types_lsn.rs

**Files:**
- Create: `tests/core/types_xml.rs`
- Create: `tests/core/types_lsn.rs`
- Delete: `tests/core/types_misc.rs`
- Modify: `tests/core/mod.rs`

**Step 1: Create `tests/core/types_xml.rs`**

Move only the XML tests from `types_misc.rs`:

```rust
use bytes::BytesMut;
use sentinel_driver::types::xml::PgXml;
use sentinel_driver::types::{FromSql, Oid, ToSql};

// Move ALL #[test] functions that test PgXml from types_misc.rs
// Keep exact same code, just in a new file
```

**Step 2: Create `tests/core/types_lsn.rs`**

Move only the LSN tests from `types_misc.rs`:

```rust
use bytes::BytesMut;
use sentinel_driver::types::lsn::PgLsn;
use sentinel_driver::types::{FromSql, Oid, ToSql};

// Move ALL #[test] functions that test PgLsn from types_misc.rs
```

**Step 3: Update `tests/core/mod.rs`**

Replace `mod types_misc;` with:
```rust
mod types_lsn;
mod types_xml;
```

**Step 4: Delete `tests/core/types_misc.rs`**

```bash
rm tests/core/types_misc.rs
```

**Step 5: Verify**

Run: `cargo test --workspace`
Expected: Same number of tests pass as before.

**Step 6: Commit**

```bash
git add tests/core/types_xml.rs tests/core/types_lsn.rs tests/core/mod.rs
git rm tests/core/types_misc.rs
git commit -m "refactor(tests): split types_misc.rs into types_xml.rs + types_lsn.rs"
```

---

## Task 3: Move local tests from auth/ to tests/core/auth/

**Files:**
- Create: `tests/core/auth/mod.rs`
- Create: `tests/core/auth/scram.rs`
- Create: `tests/core/auth/md5.rs`
- Modify: `crates/sentinel-driver/src/auth/scram.rs` (remove `#[cfg(test)]` module)
- Modify: `crates/sentinel-driver/src/auth/md5.rs` (remove `#[cfg(test)]` module)
- Modify: `tests/core/mod.rs`

**Step 1: Read the existing local tests**

Read `crates/sentinel-driver/src/auth/scram.rs` from line 243 onward — copy the entire `#[cfg(test)] mod tests { ... }` block.
Read `crates/sentinel-driver/src/auth/md5.rs` from line 132 onward — same.

**Step 2: Create `tests/core/auth/mod.rs`**

```rust
mod md5;
mod scram;
```

**Step 3: Create `tests/core/auth/scram.rs`**

Rewrite the tests to use public API imports instead of `use super::*`. Each test function that called internal functions needs to import from `sentinel_driver::auth::scram::*` or use the public API. If any test relies on `pub(crate)` internals, make the tested function `pub` or restructure the test to use the public API.

**Step 4: Create `tests/core/auth/md5.rs`**

Same approach as scram.rs.

**Step 5: Remove `#[cfg(test)] mod tests` from source files**

In `crates/sentinel-driver/src/auth/scram.rs`: delete everything from `#[cfg(test)]` to end of file.
In `crates/sentinel-driver/src/auth/md5.rs`: delete everything from `#[cfg(test)]` to end of file.

**Step 6: Update `tests/core/mod.rs`**

Add: `mod auth;`

**Step 7: Verify**

Run: `cargo test --workspace`
Expected: Same tests pass. Test count unchanged.

**Step 8: Commit**

```bash
git add tests/core/auth/
git add crates/sentinel-driver/src/auth/scram.rs \
       crates/sentinel-driver/src/auth/md5.rs \
       tests/core/mod.rs
git commit -m "refactor(tests): move auth tests from source to tests/core/auth/"
```

---

## Task 4: Move local tests from pool/ to tests/core/pool/

**Files:**
- Create: `tests/core/pool/mod.rs`
- Create: `tests/core/pool/health.rs`
- Create: `tests/core/pool/config.rs`
- Create: `tests/core/pool/pool.rs`
- Modify: `crates/sentinel-driver/src/pool/health.rs` (remove `#[cfg(test)]`)
- Modify: `crates/sentinel-driver/src/pool/config.rs` (remove `#[cfg(test)]`)
- Modify: `crates/sentinel-driver/src/pool/mod.rs` (remove `#[cfg(test)]`)
- Move: `tests/core/pool_health.rs` → merge into `tests/core/pool/health.rs`
- Modify: `tests/core/mod.rs`

**Important:** `tests/core/pool_health.rs` already exists as an integration test file. Merge its contents with the local tests from `pool/health.rs` into the new `tests/core/pool/health.rs`.

**Step 1: Create `tests/core/pool/mod.rs`**

```rust
mod config;
mod health;
mod pool;
```

**Step 2: Create `tests/core/pool/health.rs`**

Merge tests from:
- `crates/sentinel-driver/src/pool/health.rs` local tests (4 tests)
- `tests/core/pool_health.rs` existing integration tests

**Step 3: Create `tests/core/pool/config.rs`**

Move tests from `crates/sentinel-driver/src/pool/config.rs` local tests.

**Step 4: Create `tests/core/pool/pool.rs`**

Move tests from `crates/sentinel-driver/src/pool/mod.rs` local test.

**Step 5: Remove local `#[cfg(test)]` from all 3 source files**

**Step 6: Update `tests/core/mod.rs`**

Remove: `mod pool_health;`
Add: `mod pool;`

**Step 7: Delete `tests/core/pool_health.rs`**

**Step 8: Verify + Commit**

```bash
cargo test --workspace
git commit -m "refactor(tests): move pool tests from source to tests/core/pool/"
```

---

## Task 5: Move local tests from pipeline/ and notify/

**Files:**
- Create: `tests/core/pipeline/mod.rs`
- Create: `tests/core/pipeline/batch.rs`
- Create: `tests/core/notify/mod.rs`
- Create: `tests/core/notify/notify.rs`
- Modify: `crates/sentinel-driver/src/pipeline/batch.rs` (remove `#[cfg(test)]`)
- Modify: `crates/sentinel-driver/src/notify/mod.rs` (remove `#[cfg(test)]`)
- Move: `tests/core/notify_channel.rs` → merge into `tests/core/notify/`
- Modify: `tests/core/mod.rs`

**Step 1: Create `tests/core/pipeline/mod.rs`**

```rust
mod batch;
```

**Step 2: Create `tests/core/pipeline/batch.rs`**

Move 5 tests from `crates/sentinel-driver/src/pipeline/batch.rs`.

**Step 3: Create `tests/core/notify/mod.rs`**

```rust
mod channel;
mod notify;
```

**Step 4: Create `tests/core/notify/notify.rs`**

Move 4 tests from `crates/sentinel-driver/src/notify/mod.rs`.

**Step 5: Move `tests/core/notify_channel.rs` → `tests/core/notify/channel.rs`**

```bash
mv tests/core/notify_channel.rs tests/core/notify/channel.rs
```

**Step 6: Remove local `#[cfg(test)]` from source files**

**Step 7: Update `tests/core/mod.rs`**

Remove: `mod notify_channel;`
Add: `mod pipeline;`
Add: `mod notify;`

**Step 8: Verify + Commit**

```bash
cargo test --workspace
git commit -m "refactor(tests): move pipeline and notify tests to tests/core/ subdirs"
```

---

## Task 6: Reorganize remaining flat test files into subdirectories

**Files:**
- Create: `tests/core/types/mod.rs`
- Move: all `tests/core/types_*.rs` → `tests/core/types/*.rs`
- Create: `tests/core/protocol/mod.rs`
- Move: all `tests/core/protocol_*.rs` → `tests/core/protocol/*.rs`
- Create: `tests/core/copy/mod.rs`
- Move: `tests/core/copy_binary.rs` → `tests/core/copy/binary.rs`
- Move: `tests/core/copy_text.rs` → `tests/core/copy/text.rs`
- Modify: `tests/core/mod.rs`

**Step 1: Create `tests/core/types/mod.rs`**

```rust
mod builtin;
mod decode;
mod encode;
mod geometric;
mod interval;
mod lsn;
mod money;
mod network;
mod numeric;
mod range;
mod xml;
```

**Step 2: Move all types test files**

```bash
mkdir -p tests/core/types
mv tests/core/types_builtin.rs tests/core/types/builtin.rs
mv tests/core/types_decode.rs tests/core/types/decode.rs
mv tests/core/types_encode.rs tests/core/types/encode.rs
mv tests/core/types_geometric.rs tests/core/types/geometric.rs
mv tests/core/types_interval.rs tests/core/types/interval.rs
mv tests/core/types_lsn.rs tests/core/types/lsn.rs
mv tests/core/types_money.rs tests/core/types/money.rs
mv tests/core/types_network.rs tests/core/types/network.rs
mv tests/core/types_numeric.rs tests/core/types/numeric.rs
mv tests/core/types_range.rs tests/core/types/range.rs
mv tests/core/types_xml.rs tests/core/types/xml.rs
```

**Step 3: Create `tests/core/protocol/mod.rs`**

```rust
mod backend;
mod codec;
mod frontend;
```

**Step 4: Move protocol test files**

```bash
mkdir -p tests/core/protocol
mv tests/core/protocol_backend.rs tests/core/protocol/backend.rs
mv tests/core/protocol_codec.rs tests/core/protocol/codec.rs
mv tests/core/protocol_frontend.rs tests/core/protocol/frontend.rs
```

**Step 5: Create `tests/core/copy/mod.rs`**

```rust
mod binary;
mod text;
```

**Step 6: Move copy test files**

```bash
mkdir -p tests/core/copy
mv tests/core/copy_binary.rs tests/core/copy/binary.rs
mv tests/core/copy_text.rs tests/core/copy/text.rs
```

**Step 7: Rewrite `tests/core/mod.rs`**

```rust
mod auth;
mod cache;
mod cancel;
mod config;
mod copy;
mod notify;
mod pipeline;
mod pool;
mod protocol;
mod row;
mod statement;
mod transaction;
mod types;
```

Note: `cancel.rs`, `cache.rs`, `config.rs`, `row.rs`, `statement.rs`, `transaction.rs` stay flat — they don't have subdirectories because they're single files.

**Step 8: Verify + Commit**

```bash
cargo test --workspace
git commit -m "refactor(tests): organize test files into subdirectories matching source structure"
```

---

## Task 7: Extract Connection struct from lib.rs to connection/

**Files:**
- Create: `crates/sentinel-driver/src/connection/client.rs`
- Create: `crates/sentinel-driver/src/connection/query.rs`
- Create: `crates/sentinel-driver/src/connection/transaction_impl.rs`
- Create: `crates/sentinel-driver/src/connection/copy_impl.rs`
- Create: `crates/sentinel-driver/src/connection/notify_impl.rs`
- Create: `crates/sentinel-driver/src/connection/pipeline_impl.rs`
- Create: `crates/sentinel-driver/src/connection/prepare.rs`
- Modify: `crates/sentinel-driver/src/connection/mod.rs`
- Modify: `crates/sentinel-driver/src/lib.rs`

**Step 1: Create Connection struct in `connection/mod.rs`**

Move the `Connection` struct definition, its fields, and the `use` imports from lib.rs:

```rust
use std::time::Duration;
use bytes::BytesMut;

use crate::cache::StatementCache;
use crate::cancel::CancelToken;
use crate::config::Config;
use crate::copy;
use crate::error::{Error, Result};
use crate::notify::{self, Notification};
use crate::pipeline::{self, batch::PipelineBatch};
use crate::protocol::backend::{BackendMessage, TransactionStatus};
use crate::protocol::frontend;
use crate::row::{self, CommandResult, Row};
use crate::statement::Statement;
use crate::transaction::TransactionConfig;
use crate::types::{Oid, ToSql};
use crate::cache::CacheMetrics;

mod client;
mod copy_impl;
mod notify_impl;
mod pipeline_impl;
mod prepare;
mod query;
mod transaction_impl;

pub use stream::PgConnection;

/// A high-level connection to PostgreSQL.
pub struct Connection {
    pub(crate) conn: PgConnection,
    pub(crate) config: Config,
    pub(crate) process_id: i32,
    pub(crate) secret_key: i32,
    pub(crate) transaction_status: TransactionStatus,
    pub(crate) stmt_cache: StatementCache,
    pub(crate) query_timeout: Option<Duration>,
    pub(crate) is_broken: bool,
}
```

Note: Fields become `pub(crate)` so impl blocks in other files within the connection module can access them.

**Step 2: Create `connection/client.rs`**

```rust
use super::*;

impl Connection {
    pub async fn connect(config: Config) -> Result<Self> { /* ... exact code from lib.rs ... */ }
    pub async fn close(self) -> Result<()> { /* ... */ }
    pub fn cancel_token(&self) -> CancelToken { /* ... */ }
    pub fn is_tls(&self) -> bool { /* ... */ }
    pub fn process_id(&self) -> i32 { /* ... */ }
    pub fn query_timeout(&self) -> Option<Duration> { /* ... */ }
    pub fn is_broken(&self) -> bool { /* ... */ }
    pub fn transaction_status(&self) -> TransactionStatus { /* ... */ }

    // Internal helpers used by other impl files
    pub(crate) async fn drain_until_ready(&mut self) -> Result<()> { /* ... */ }
    pub(crate) async fn query_internal(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<pipeline::QueryResult> { /* ... */ }
}
```

**Step 3: Create `connection/query.rs`**

```rust
use super::*;

impl Connection {
    pub async fn query(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> { /* ... */ }
    pub async fn query_one(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> { /* ... */ }
    pub async fn query_opt(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>> { /* ... */ }
    pub async fn execute(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> { /* ... */ }
    pub async fn query_with_timeout(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)], timeout: Duration) -> Result<Vec<Row>> { /* ... */ }
    pub async fn execute_with_timeout(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)], timeout: Duration) -> Result<u64> { /* ... */ }
}
```

**Step 4: Create `connection/transaction_impl.rs`**

```rust
use super::*;

impl Connection {
    pub async fn begin(&mut self) -> Result<()> { /* ... */ }
    pub async fn begin_with(&mut self, config: TransactionConfig) -> Result<()> { /* ... */ }
    pub async fn commit(&mut self) -> Result<()> { /* ... */ }
    pub async fn rollback(&mut self) -> Result<()> { /* ... */ }
    pub async fn savepoint(&mut self, name: &str) -> Result<()> { /* ... */ }
    pub async fn rollback_to(&mut self, name: &str) -> Result<()> { /* ... */ }
}
```

**Step 5: Create `connection/copy_impl.rs`**

```rust
use super::*;

impl Connection {
    pub async fn copy_in(&mut self, sql: &str) -> Result<copy::CopyIn<'_>> { /* ... */ }
    pub async fn copy_out(&mut self, sql: &str) -> Result<copy::CopyOut<'_>> { /* ... */ }
}
```

**Step 6: Create `connection/notify_impl.rs`**

```rust
use super::*;

impl Connection {
    pub async fn listen(&mut self, channel: &str) -> Result<()> { /* ... */ }
    pub async fn unlisten(&mut self, channel: &str) -> Result<()> { /* ... */ }
    pub async fn unlisten_all(&mut self) -> Result<()> { /* ... */ }
    pub async fn notify(&mut self, channel: &str, payload: &str) -> Result<()> { /* ... */ }
    pub async fn wait_for_notification(&mut self) -> Result<Notification> { /* ... */ }
}
```

**Step 7: Create `connection/pipeline_impl.rs`**

```rust
use super::*;

impl Connection {
    pub fn pipeline(&self) -> PipelineBatch { /* ... */ }
    pub async fn execute_pipeline(&mut self, batch: PipelineBatch) -> Result<Vec<pipeline::QueryResult>> { /* ... */ }
}
```

**Step 8: Create `connection/prepare.rs`**

```rust
use super::*;

impl Connection {
    pub async fn prepare(&mut self, sql: &str) -> Result<Statement> { /* ... */ }
    pub fn register_statement(&mut self, name: &str, statement: Statement) { /* ... */ }
    pub fn cache_metrics(&self) -> &CacheMetrics { /* ... */ }
}
```

**Step 9: Rewrite `lib.rs` to be re-exports only**

```rust
//! # sentinel-driver
//! (keep existing module doc)

pub mod auth;
pub mod cache;
pub mod cancel;
pub mod config;
pub mod connection;
pub mod copy;
pub mod error;
pub mod notify;
pub mod pipeline;
pub mod pool;
pub mod protocol;
pub mod row;
pub mod statement;
pub mod tls;
pub mod transaction;
pub mod types;

// Public re-exports
pub use cache::{CacheMetrics, StatementCache};
pub use cancel::CancelToken;
pub use config::{Config, SslMode};
pub use connection::Connection;
pub use copy::binary::{BinaryCopyDecoder, BinaryCopyEncoder};
pub use copy::text::{TextCopyDecoder, TextCopyEncoder};
pub use error::{Error, Result};
pub use notify::Notification;
pub use pool::Pool;
pub use row::{CommandResult, Row, RowDescription};
pub use statement::Statement;
pub use transaction::{IsolationLevel, TransactionConfig};
pub use types::{FromSql, Oid, ToSql};

#[cfg(feature = "derive")]
pub use sentinel_derive::{FromRow, FromSql, ToSql};
```

**Step 10: Verify**

Run: `cargo test --workspace`
Run: `cargo clippy --workspace -- -D warnings`
Run: `cargo fmt --all -- --check`
Expected: Everything passes. Public API unchanged.

**Step 11: Commit**

```bash
git commit -m "refactor: extract Connection from lib.rs into connection/ submodules"
```

---

## Task 8: Final cleanup and format check

**Files:**
- Modify: `tests/core/mod.rs` (verify final state)
- All files (format check)

**Step 1: Verify test count matches**

```bash
cargo test --workspace 2>&1 | grep "test result:"
```

Compare total test count with baseline (should be identical — we moved tests, not deleted them).

**Step 2: Full lint + format**

```bash
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --all -- --check
```

If fmt fails, run `cargo fmt --all` then commit.

**Step 3: Final commit**

```bash
git commit -m "refactor: final cleanup — format and verify"
```

---

## Summary

| Task | What | Files Changed | Risk |
|------|------|--------------|------|
| 1 | Split types/mod.rs → oid.rs + traits.rs | 3 | Low |
| 2 | Split types_misc.rs → xml.rs + lsn.rs | 4 | Low |
| 3 | Move auth local tests → tests/core/auth/ | 5 | Low |
| 4 | Move pool local tests → tests/core/pool/ | 8 | Low |
| 5 | Move pipeline/notify local tests | 7 | Low |
| 6 | Reorganize flat tests → subdirectories | ~20 | Medium (many file moves) |
| 7 | Extract Connection from lib.rs | ~10 | Medium (largest change) |
| 8 | Final cleanup | All | Low |

**Total: 8 tasks, ~60 file operations, 0 logic changes**
