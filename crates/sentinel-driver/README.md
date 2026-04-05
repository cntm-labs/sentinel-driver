<div align="center">

# sentinel-driver

**High-performance PostgreSQL wire protocol driver for Rust.**

[![crates.io](https://img.shields.io/crates/v/sentinel-driver?color=fc8d62)](https://crates.io/crates/sentinel-driver)
[![docs.rs](https://img.shields.io/docsrs/sentinel-driver)](https://docs.rs/sentinel-driver)

</div>

---

- **PG-only** -- no multi-database abstraction tax
- **Pipeline mode** -- automatic query batching (PG 14+)
- **Binary encoding** -- 15-40% faster for non-text types
- **Zero-copy** -- `bytes::Bytes` slices for large column values
- **SCRAM-SHA-256** -- correct SASLprep implementation
- **COPY protocol** -- bulk insert 10-50x faster than INSERT
- **LISTEN/NOTIFY** -- first-class realtime notifications
- **Cancel query** -- CancelToken for safe query cancellation
- **Per-query timeout** -- auto-cancel with configurable statement_timeout
- **Array types** -- `Vec<T>` encode/decode for PostgreSQL arrays

## Usage

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

## License

MIT OR Apache-2.0
