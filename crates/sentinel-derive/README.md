# sentinel-derive

Derive macros for [sentinel-driver](https://crates.io/crates/sentinel-driver).

Provides `#[derive(FromRow)]`, `#[derive(ToSql)]`, and `#[derive(FromSql)]` for automatic type mapping between Rust structs and PostgreSQL rows.

## Usage

```toml
[dependencies]
sentinel-driver = { version = "0.1", features = ["derive"] }
```

```rust
use sentinel_driver::FromRow;

#[derive(FromRow)]
struct User {
    id: i32,
    name: String,
    email: String,
}
```

## License

MIT OR Apache-2.0
