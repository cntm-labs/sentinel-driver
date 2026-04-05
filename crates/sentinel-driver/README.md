# sentinel-driver

High-performance PostgreSQL wire protocol driver for Rust.

- **PG-only** — no multi-database abstraction tax
- **Pipeline mode** — automatic query batching (PG 14+)
- **Binary encoding** — 15-40% faster for non-text types
- **Zero-copy** — `bytes::Bytes` slices for large column values
- **SCRAM-SHA-256** — correct SASLprep implementation
- **COPY protocol** — bulk insert 10-50x faster than INSERT
- **LISTEN/NOTIFY** — first-class realtime notifications

## Usage

```toml
[dependencies]
sentinel-driver = "0.1"
```

```rust
use sentinel_driver::{Config, Connection};

#[tokio::main]
async fn main() -> Result<(), sentinel_driver::Error> {
    let config = Config::builder()
        .host("localhost")
        .database("mydb")
        .user("postgres")
        .password("secret")
        .build()?;

    let mut conn = Connection::connect(&config).await?;
    let rows = conn.query("SELECT id, name FROM users", &[]).await?;

    for row in &rows {
        let id: i32 = row.get(0)?;
        let name: &str = row.get(1)?;
        println!("{}: {}", id, name);
    }

    Ok(())
}
```

## License

MIT OR Apache-2.0
