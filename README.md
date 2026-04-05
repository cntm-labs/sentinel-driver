# sentinel-driver

High-performance PostgreSQL wire protocol driver for Rust. PG-only, zero-copy, pipeline-first.

Foundation layer for [Sentinel ORM](https://github.com/cntm-labs/sentinel). Can be used independently as a standalone PostgreSQL driver.

## Quick Start

Add to `Cargo.toml`:

```toml
[dependencies]
sentinel-driver = "0.1"
```

```rust
use sentinel_driver::{Config, Connection};

#[tokio::main]
async fn main() -> Result<(), sentinel_driver::Error> {
    let config = Config::parse("postgres://user:pass@localhost/mydb")?;
    let mut conn = Connection::connect(&config).await?;

    let rows = conn.query("SELECT id, name FROM users WHERE id = $1", &[&42i32]).await?;
    for row in &rows {
        let id: i32 = row.get(0);
        let name: String = row.get(1);
        println!("{id}: {name}");
    }

    Ok(())
}
```

## Features

- **SCRAM-SHA-256** with correct SASLprep (where sqlx gets it wrong)
- **Pipeline mode** (PG 14+) -- batch queries in a single round-trip
- **COPY protocol** -- bulk insert 10-50x faster than INSERT
- **LISTEN/NOTIFY** -- first-class realtime notifications
- **Two-tier statement cache** -- HashMap + LRU-256, ~99% hit rate
- **Connection pool** -- deadpool-style, <0.5 us checkout
- **Binary format** by default -- 15-40% faster for non-text types
- **Zero-copy** -- `bytes::Bytes` slices for large column values
- **rustls** -- no OpenSSL dependency

## Performance Targets

| Metric | Target |
|---|---|
| Simple SELECT | 90K+ queries/s |
| Batch 100 queries | 15K+ batches/s |
| Bulk INSERT 10K rows | 500K+ rows/s |
| Pool checkout | <0.5 us |
| Statement cache hit rate | ~99% |

## MSRV

Rust 1.75 (declared via `rust-version` in Cargo.toml).

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
