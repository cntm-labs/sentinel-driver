<div align="center">

# sentinel-derive

**Derive macros for [sentinel-driver](https://crates.io/crates/sentinel-driver).**

[![crates.io](https://img.shields.io/crates/v/sentinel-derive?color=fc8d62)](https://crates.io/crates/sentinel-derive)
[![docs.rs](https://img.shields.io/docsrs/sentinel-derive)](https://docs.rs/sentinel-derive)

</div>

---

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
