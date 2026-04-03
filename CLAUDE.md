# CLAUDE.md — sentinel-driver

## Overview
High-performance PostgreSQL wire protocol driver for Rust. Foundation layer for Sentinel ORM.
Can be used independently as a standalone PG driver crate.

## Tech Stack
- **Language:** Rust (stable)
- **Database:** PostgreSQL (only)
- **Async:** tokio
- **TLS:** rustls
- **Crypto:** sha2, hmac, stringprep (for SCRAM-SHA-256)
- **Buffers:** bytes (zero-copy)

## Project Structure
```
sentinel-driver/
├── src/
│   ├── lib.rs              # Public API
│   ├── config.rs           # Connection configuration
│   ├── error.rs            # Error types
│   ├── protocol/
│   │   ├── frontend.rs     # Client-to-server messages
│   │   ├── backend.rs      # Server-to-client messages
│   │   └── codec.rs        # Encoder/decoder (zero-copy)
│   ├── connection/
│   │   ├── stream.rs       # TCP/TLS stream
│   │   └── startup.rs      # Handshake + auth
│   ├── auth/
│   │   ├── scram.rs        # SCRAM-SHA-256 (correct SASLprep)
│   │   └── md5.rs          # MD5 (legacy)
│   ├── pool/               # Connection pool (<0.5 μs checkout)
│   ├── pipeline/           # PG pipeline mode (auto-batch)
│   ├── copy/               # COPY IN/OUT (binary + text)
│   ├── notify/             # LISTEN/NOTIFY engine
│   ├── types/              # PG type encode/decode (binary format)
│   ├── tls/                # rustls integration
│   ├── row.rs              # Row type (zero-copy column access)
│   ├── statement.rs        # Prepared statement
│   └── transaction.rs      # Transaction wrapper
├── sentinel-driver-derive/  # FromRow, ToSql, FromSql proc macros
│   └── src/
├── docs/
│   └── plans/
└── Cargo.toml
```

## Build Commands
```sh
cargo check                      # Type check
cargo test                       # Run tests
cargo clippy -- -D warnings      # Lint
cargo fmt --all                  # Format
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
- SCRAM-SHA-256 with correct SASLprep (sqlx gets this wrong)
- Pipeline mode (PG 14+) — batch queries in single round-trip
- COPY protocol — bulk insert 10-50x faster than INSERT
- LISTEN/NOTIFY — first-class realtime notifications
- Two-tier prepared statement cache (HashMap + LRU-256)
- Connection pool (deadpool-style, <0.5 μs checkout)

## Conventions
- Minimal unsafe — only where required for zero-copy parsing
- All unsafe must have SAFETY comment explaining invariant
- Binary format for all PG types by default
- Every public API must be documented
- 100% test coverage target

## Dependencies (minimal)
- tokio, bytes, rustls, webpki-roots
- sha2, hmac, stringprep
- chrono, uuid, thiserror

No sqlx, no openssl, no libpq.

## Related Projects
- **sentinel** — ORM built on this driver (at ../sentinel)
- **layer-2** — Future realtime platform (at ../layer-2)

## Design Document
See `docs/plans/2026-04-03-sentinel-driver-design.md` for full design.
