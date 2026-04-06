# Phase 1C: BIT/VARBIT Type Support

**Date:** 2026-04-06
**Status:** Approved

## Goal

Add BIT and VARBIT PostgreSQL type support using a built-in `PgBit` struct with no external dependencies.

## Design

### Type

```rust
/// PostgreSQL BIT / VARBIT type.
///
/// Stores a fixed- or variable-length bit string.
/// `data` holds raw bytes (MSB-first, padded with trailing zeros).
/// `bit_length` is the actual number of significant bits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PgBit {
    pub data: Vec<u8>,
    pub bit_length: i32,
}
```

### OIDs

| Type | OID | Array OID |
|------|-----|-----------|
| BIT | 1560 | 1561 |
| VARBIT | 1562 | 1563 |

### Wire Format (PG Binary)

```
bit_length: i32    (4 bytes, big-endian — number of bits)
data: [u8]         (ceil(bit_length / 8) bytes, MSB-first)
```

Trailing bits in the last byte are zero-padded (not significant).

### ToSql

`PgBit` encodes with OID `VARBIT` (1562) by default — VARBIT is a superset of BIT. Users who need fixed-width BIT can cast in SQL: `$1::bit(n)`.

### FromSql

`PgBit` decodes from both BIT (1560) and VARBIT (1562). Since the wire format is identical, the same `from_sql` handles both.

### Array Support

`Vec<PgBit>` encodes as VARBIT_ARRAY (1563) and decodes from both BIT_ARRAY (1561) and VARBIT_ARRAY (1563).

### Helper Methods

```rust
impl PgBit {
    /// Create from a slice of bools (true = 1, false = 0).
    pub fn from_bools(bits: &[bool]) -> Self;

    /// Get the bit at the given index (0-based, MSB-first).
    pub fn get(&self, index: usize) -> Option<bool>;

    /// Number of bits.
    pub fn len(&self) -> usize;

    /// True if zero bits.
    pub fn is_empty(&self) -> bool;
}
```

### Files

- Create: `crates/sentinel-driver/src/types/bit.rs`
- Modify: `crates/sentinel-driver/src/types/mod.rs` (add OIDs + `pub mod bit`)
- Modify: `crates/sentinel-driver/src/types/builtin.rs` (register BIT, VARBIT)
- Modify: `crates/sentinel-driver/src/types/encode.rs` (array macro)
- Modify: `crates/sentinel-driver/src/types/decode.rs` (array macro)
- Create: `tests/core/types_bit.rs`
- Modify: `tests/core/mod.rs`

### Decision: No HSTORE

HSTORE is a PG extension with no fixed OID — requires runtime type resolution via `pg_type` catalog queries. Deferred until runtime OID infrastructure exists. JSONB covers most HSTORE use cases.
