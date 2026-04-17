# Phase 4D: Ecosystem Types — Domain, LTREE, CUBE, time crate

**Date:** 2026-04-17
**Status:** Approved
**Goal:** Add remaining ecosystem type support for full coverage.

## 1. Domain Types

PG domain types wrap a base type with constraints (`CREATE DOMAIN email AS text`).

**Approach:** No new OIDs needed. Domain types encode/decode as their base type. Our existing derive macro `#[sentinel(transparent)]` already handles newtype wrappers. Verify this works for domain types and add tests + documentation.

```rust
#[derive(ToSql, FromSql)]
#[sentinel(transparent)]
struct Email(String);  // encodes as TEXT, decodes as TEXT
```

**Files:** Verify `crates/sentinel-derive/src/lib.rs`, add domain-specific tests.

## 2. LTREE / LQUERY (PG Extension)

Hierarchical label tree for path-based queries.

```rust
/// PostgreSQL LTREE type — dot-separated label path.
pub struct PgLTree(pub String);

/// PostgreSQL LQUERY type — LTREE query pattern.
pub struct PgLQuery(pub String);
```

- Wire format: text (UTF-8 bytes), same as TEXT
- OID: Runtime (extension type) — use TEXT OID as carrier
- Encode: write string bytes
- Decode: read string bytes

**Files:** Create `types/ltree.rs`

## 3. CUBE (PG Extension)

Multi-dimensional points and boxes.

```rust
/// PostgreSQL CUBE type — n-dimensional point or box.
pub struct PgCube {
    /// For a point: [x, y, z, ...]
    /// For a box: [x1, y1, z1, ..., x2, y2, z2, ...]
    pub coordinates: Vec<f64>,
    /// True if this is a point (all dimensions equal), false if box.
    pub is_point: bool,
}
```

- Wire format (binary): ndim(u32) + flags(u32) + [f64; ndim * (is_point ? 1 : 2)]
  - flags bit 0: is_point
- OID: Runtime (extension type) — use custom OID or TEXT fallback

**Files:** Create `types/cube.rs`

## 4. `time` Crate Support

Feature-gated alternative to chrono for date/time types.

```toml
[dependencies]
time = { version = "0.3", optional = true }

[features]
with-time = ["dep:time"]
```

| Rust type | PG type | OID |
|-----------|---------|-----|
| `time::OffsetDateTime` | TIMESTAMPTZ | 1184 |
| `time::PrimitiveDateTime` | TIMESTAMP | 1114 |
| `time::Date` | DATE | 1082 |
| `time::Time` | TIME | 1083 |

PG epoch offset: same as chrono (2000-01-01).

**Files:** Create `types/time_support.rs` (cfg-gated under `with-time`)

## Files Summary

| File | Action |
|------|--------|
| `types/ltree.rs` | Create — PgLTree, PgLQuery |
| `types/cube.rs` | Create — PgCube |
| `types/time_support.rs` | Create — time crate impls (feature-gated) |
| `types/mod.rs` | Add modules |
| `Cargo.toml` | Add `time` optional dep, `with-time` feature |
| `crates/sentinel-derive/` | Verify transparent works for domains, add tests |
| Tests | Domain, LTREE, CUBE, time roundtrips |

## Dependencies
- `time = { version = "0.3", optional = true, features = ["std"] }`
