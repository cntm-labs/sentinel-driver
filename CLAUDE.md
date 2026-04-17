# CLAUDE.md — sentinel-driver

## Overview
High-performance PostgreSQL wire protocol driver for Rust. Foundation layer for Sentinel ORM.
Can be used independently as a standalone PG driver crate.

## Tech Stack
- **Language:** Rust (stable)
- **Database:** PostgreSQL (only)
- **Async:** tokio
- **TLS:** rustls + rustls-pemfile (client certs)
- **Crypto:** sha2, hmac, stringprep (for SCRAM-SHA-256 + SCRAM-SHA-256-PLUS)
- **Buffers:** bytes (zero-copy)
- **Async types:** futures-core (BoxFuture for pool callbacks)

## Project Structure
```
sentinel-driver/
├── crates/
│   ├── sentinel-driver/        # Main driver crate
│   │   └── src/
│   │       ├── lib.rs          # Public API
│   │       ├── config.rs       # Connection configuration
│   │       ├── error.rs        # Error types
│   │       ├── protocol/
│   │       │   ├── frontend.rs # Client-to-server messages
│   │       │   ├── backend.rs  # Server-to-client messages
│   │       │   └── codec.rs    # Encoder/decoder (zero-copy)
│   │       ├── connection/
│   │       │   ├── stream.rs   # TCP/TLS stream
│   │       │   └── startup.rs  # Handshake + auth
│   │       ├── auth/
│   │       │   ├── scram.rs    # SCRAM-SHA-256 + SCRAM-SHA-256-PLUS (channel binding)
│   │       │   └── md5.rs      # MD5 (legacy)
│   │       ├── pool/           # Connection pool (<0.5 μs checkout, callbacks, lazy connect)
│   │       ├── pipeline/       # PG pipeline mode (auto-batch)
│   │       ├── copy/           # COPY IN/OUT (binary + text)
│   │       ├── notify/         # LISTEN/NOTIFY engine
│   │       ├── types/          # PG type encode/decode (24 modules, 78 OIDs)
│   │       │   ├── oid.rs      # OID constants
│   │       │   ├── traits.rs   # ToSql/FromSql traits
│   │       │   ├── encode.rs   # ToSql implementations + array macros
│   │       │   ├── decode.rs   # FromSql implementations + array macros
│   │       │   ├── builtin.rs  # Type info registry
│   │       │   ├── network.rs  # INET/CIDR/MACADDR
│   │       │   ├── numeric.rs  # NUMERIC/Decimal (feature-gated)
│   │       │   ├── range.rs    # Range types (6 variants)
│   │       │   ├── interval.rs # PgInterval
│   │       │   ├── geometric.rs # Point/Line/LSeg/Box/Path/Polygon/Circle
│   │       │   ├── money.rs    # PgMoney
│   │       │   ├── bit.rs      # BIT/VARBIT (PgBit)
│   │       │   ├── hstore.rs   # HSTORE
│   │       │   ├── xml.rs      # XML
│   │       │   ├── lsn.rs      # PG_LSN
│   │       │   ├── timetz.rs   # TIMETZ (time with timezone)
│   │       │   ├── multirange.rs # Multirange types (PG 14+)
│   │       │   ├── ltree.rs    # LTREE/LQUERY (extension)
│   │       │   ├── cube.rs     # CUBE (extension)
│   │       │   ├── json.rs     # Json<T> wrapper + serde_json::Value
│   │       │   └── time_support.rs # time crate impls (feature-gated)
│   │       ├── tls/            # rustls + client certs + direct TLS (PG 17+)
│   │       ├── row.rs          # Row type (zero-copy column access)
│   │       ├── stream.rs       # RowStream (async row-by-row iteration)
│   │       ├── portal.rs       # Portal/Cursor (server-side pagination)
│   │       ├── advisory_lock.rs # Advisory locks (RAII)
│   │       ├── observability.rs # Tracing spans, slow query, metrics
│   │       ├── statement.rs    # Prepared statement
│   │       ├── cancel.rs       # Query cancellation via CancelToken
│   │       └── transaction.rs  # Transaction wrapper
│   └── sentinel-derive/        # Derive macros crate
│       └── src/
│           └── lib.rs          # FromRow, ToSql, FromSql (enums, composites, newtypes)
├── tests/
│   ├── core/               # Unit-level integration tests (no PG required)
│   ├── postgres/           # Live PG integration tests (DATABASE_URL required)
│   ├── docker-compose.yml  # PG 13/16/17 for local testing
│   ├── fixtures/           # Test data files
│   └── certs/              # TLS test certificates
├── .github/
│   ├── workflows/          # CI: lint, test, pg-matrix, coverage, release
│   ├── ISSUE_TEMPLATE/     # Bug report, feature request, docs
│   └── pull_request_template.md
├── .githooks/
│   └── pre-commit          # fmt + clippy + test
├── docs/
│   └── plans/
├── clippy.toml             # Clippy config (disallowed methods, thresholds)
├── rustfmt.toml            # Format config (edition 2021, max_width 100)
├── .editorconfig           # Editor config
├── Cargo.toml              # Workspace root (no package)
└── Cargo.lock
```

## Build Commands
```sh
cargo check                      # Type check
cargo test --workspace           # Run all tests
cargo clippy --workspace -- -D warnings  # Lint
cargo fmt --all                  # Format
cargo fmt --all -- --check       # Check formatting (CI)
```

## Git Hooks
```sh
git config core.hooksPath .githooks   # Enable pre-commit hook
```

## Design Principles
- **PG-only** — no multi-database abstraction tax
- **Single-task architecture** — no channel overhead between tasks
- **Binary encoding by default** — 15-40% faster for non-text types
- **Pipeline-first** — automatic query batching
- **Zero-copy** — bytes::Bytes slices for large column values

## Performance Targets
| Metric | Target |
|--------|--------|
| Simple SELECT | 90K+ q/s |
| Batch 100 queries | 15K+ batch/s |
| Bulk INSERT 10K rows | 500K+ rows/s |
| Pool checkout | <0.5 μs |
| Stmt cache hit rate | ~99% |

## Key Features
- SCRAM-SHA-256 + SCRAM-SHA-256-PLUS with correct SASLprep and channel binding
- Client certificate authentication + Direct TLS (PG 17+)
- Pipeline mode (PG 14+) — batch queries in single round-trip
- Row streaming — async row-by-row iteration for large result sets
- Portal/Cursor — server-side pagination within transactions
- COPY protocol — bulk insert 10-50x faster than INSERT
- LISTEN/NOTIFY — first-class realtime notifications
- Two-tier prepared statement cache (HashMap + LRU-256) with metrics
- Connection pool (<0.5 μs checkout) with lifecycle callbacks + lazy connect
- Advisory locks (RAII) — session and transaction scoped
- Observability — tracing spans, slow query logging, query metrics callback
- 78 OIDs — full PG type coverage including enums, composites, ranges, multiranges, geometric, HSTORE, LTREE, CUBE
- Multi-host failover with load balancing + target_session_attrs (read-write/read-only routing)
- Unix domain socket support
- GenericClient trait — write code generic over Connection/Transaction/Pool
- query_typed() — skip prepare round-trip for serverless environments
- Json\<T\> wrapper — arbitrary Serialize/Deserialize to JSONB
- `time` crate support (feature-gated alternative to chrono)
- Criterion benchmarks for performance validation

## Conventions
- No unsafe code — zero-copy via `bytes::Bytes` safe API (unsafe_code = "forbid")
- Binary format for all PG types by default
- Every public API must be documented
- 100% test coverage target

## Lint Policy
Workspace lints defined in `Cargo.toml` `[workspace.lints.clippy]`:
- **forbid**: `unwrap_used`, `dbg_macro`, `todo`, `unimplemented`, `print_stdout/stderr`, `mem_forget`, `exit`, `unsafe_code`
- **deny**: `expect_used` (use `#[allow(clippy::expect_used)]` with justification), `large_enum_variant`, `needless_pass_by_value`
- **warn**: pedantic group (with select allows for noise reduction)

Use `expect("reason")` with `#[allow(clippy::expect_used)]` for infallible operations (constant dates, known-valid inputs).

## Dependencies (minimal)
- tokio, bytes, rustls, tokio-rustls, webpki-roots, rustls-pemfile
- sha2, hmac, stringprep, base64, rand
- chrono, uuid, thiserror, futures-core
- tracing, lru, criterion (dev)
- rust_decimal (optional: `with-rust-decimal`)
- serde, serde_json (optional: `with-serde-json`)
- time (optional: `with-time`)

No sqlx, no openssl, no libpq.

## Related Projects
- **sentinel** — ORM built on this driver (at ../sentinel)
- **layer-2** — Future realtime platform (at ../layer-2)

## Design Document
See `docs/plans/2026-04-03-sentinel-driver-design.md` for full design.
