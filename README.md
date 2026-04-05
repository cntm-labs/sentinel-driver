<div align="center">

# sentinel-driver

**High-performance PostgreSQL wire protocol driver for Rust. PG-only, zero-copy, pipeline-first.**

[![CI](https://github.com/cntm-labs/sentinel-driver/actions/workflows/ci.yml/badge.svg)](https://github.com/cntm-labs/sentinel-driver/actions/workflows/ci.yml)
[![PostgreSQL Integration](https://github.com/cntm-labs/sentinel-driver/actions/workflows/postgresql.yml/badge.svg)](https://github.com/cntm-labs/sentinel-driver/actions/workflows/postgresql.yml)
[![codecov](https://codecov.io/gh/cntm-labs/sentinel-driver/branch/main/graph/badge.svg)](https://codecov.io/gh/cntm-labs/sentinel-driver)

[![crates.io sentinel-driver](https://img.shields.io/crates/v/sentinel-driver?label=sentinel-driver&color=fc8d62)](https://crates.io/crates/sentinel-driver)
[![crates.io sentinel-derive](https://img.shields.io/crates/v/sentinel-derive?label=sentinel-derive&color=fc8d62)](https://crates.io/crates/sentinel-derive)
[![docs.rs](https://img.shields.io/docsrs/sentinel-driver?label=docs.rs)](https://docs.rs/sentinel-driver)

[![Rust](https://img.shields.io/badge/Rust-6k_LOC-dea584?logo=rust&logoColor=white)](crates/)
[![Shell](https://img.shields.io/badge/Config-1k_LOC-89e051)](./)
[![Total Lines](https://img.shields.io/badge/Total-8k+_LOC-blue)](./)

[![Rust](https://img.shields.io/badge/Rust-dea584?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tokio](https://img.shields.io/badge/Tokio-dea584?logo=rust&logoColor=white)](https://tokio.rs/)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-4169E1?logo=postgresql&logoColor=white)](https://www.postgresql.org/)
[![rustls](https://img.shields.io/badge/rustls-dea584?logo=rust&logoColor=white)](https://github.com/rustls/rustls)

</div>

---

Foundation layer for [Sentinel ORM](https://github.com/cntm-labs/sentinel). Can be used independently as a standalone PostgreSQL driver.

## Quick Start

```toml
[dependencies]
sentinel-driver = "0.1"
```

```rust
use sentinel_driver::{Config, Connection};

#[tokio::main]
async fn main() -> Result<(), sentinel_driver::Error> {
    let config = Config::parse("postgres://user:pass@localhost/mydb")?;
    let mut conn = Connection::connect(config).await?;

    let rows = conn.query("SELECT id, name FROM users WHERE id = $1", &[&42i32]).await?;
    for row in &rows {
        let id: i32 = row.get(0);
        let name: String = row.get(1);
    }

    Ok(())
}
```

## Features

- **SCRAM-SHA-256** with correct SASLprep (where sqlx gets it wrong)
- **Pipeline mode** (PG 14+) -- batch queries in a single round-trip
- **COPY protocol** -- bulk insert 10-50x faster than INSERT
- **LISTEN/NOTIFY** -- first-class realtime notifications
- **Cancel query** -- CancelToken for safe query cancellation from any task
- **Per-query timeout** -- auto-cancel with configurable statement_timeout
- **Two-tier statement cache** -- HashMap + LRU-256, ~99% hit rate
- **Connection pool** -- deadpool-style, <0.5 us checkout
- **Binary format** by default -- 15-40% faster for non-text types
- **Zero-copy** -- `bytes::Bytes` slices for large column values
- **Array types** -- `Vec<T>` encode/decode for PostgreSQL arrays
- **rustls** -- no OpenSSL dependency

## Architecture

```
crates/
├── sentinel-driver    # Main driver crate — connection, protocol, pool, types
└── sentinel-derive    # Derive macros — FromRow, ToSql, FromSql
```

## Performance Targets

| Metric | Target |
|---|---|
| Simple SELECT | 90K+ queries/s |
| Batch 100 queries | 15K+ batches/s |
| Bulk INSERT 10K rows | 500K+ rows/s |
| Pool checkout | <0.5 us |
| Statement cache hit rate | ~99% |

## Development

```sh
cargo check --workspace                  # Type check
cargo test --workspace                   # Run all tests
cargo clippy --workspace -- -D warnings  # Lint
cargo fmt --all                          # Format

# Setup pre-commit hook
git config core.hooksPath .githooks
```

## MSRV

Rust 1.75 (declared via `rust-version` in Cargo.toml).

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
