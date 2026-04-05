# Phase 1 Design: Array Types (#5) + Realtime Health Check (#2)

**Date:** 2026-04-05
**Status:** Approved
**Issues:** cntm-labs/sentinel-driver#5, cntm-labs/sentinel-driver#2

## Motivation

sentinel-core requires two features from sentinel-driver before pool integration:

1. **Array types** — `WHERE id = ANY($1)` pattern needs `Vec<T>` to encode/decode as PG arrays
2. **Realtime health check** — pool must verify idle connections are alive before checkout

## Issue #5: PostgreSQL Array Types

### Approach: Macro-based concrete implementations

Use an internal `impl_array!` macro to generate `ToSql` and `FromSql` for each concrete `Vec<T>`. No new public traits. This keeps the API surface minimal and avoids orphan rule complexity.

### PG Binary Array Format

Encode (wire format for 1-D arrays):

```
i32  ndim         = 1
i32  has_null     = 0
u32  element_oid
i32  dim_len      = vec.len()
i32  dim_lbound   = 1          (PG arrays are 1-based)
foreach element:
    i32  data_len
    [u8] encoded element
```

Decode (inverse):

```
read ndim (must be 0 or 1, error on ndim > 1)
read has_null flag (error if 1 — nullable elements not supported yet)
read element_oid (validate against expected)
read dim_len, dim_lbound
foreach dim_len elements:
    read i32 len (-1 = NULL, error for this phase)
    read len bytes, decode via T::from_sql()
```

### Supported Types

| Rust Type | PG Array Type | Array OID |
|-----------|---------------|-----------|
| `Vec<bool>` | bool[] | 1000 |
| `Vec<i16>` | int2[] | 1005 |
| `Vec<i32>` | int4[] | 1007 |
| `Vec<i64>` | int8[] | 1016 |
| `Vec<f32>` | float4[] | 1021 |
| `Vec<f64>` | float8[] | 1022 |
| `Vec<String>` | text[] | 1009 |
| `Vec<uuid::Uuid>` | uuid[] | 2951 |

`Vec<&str>` gets `ToSql` only (encoding). No `FromSql` since `&str` cannot own data.

### Scope Boundaries

- 1-D arrays only. Multi-dimensional arrays return an error.
- Non-nullable elements only (`Vec<T>`, not `Vec<Option<T>>`).
- Empty arrays are valid and encoded as ndim=0.

### Files Changed

- `types/mod.rs` — add `impl_array_to_sql!` and `impl_array_from_sql!` macros, invoke for all 9 types
- `types/encode.rs` — array encoding helper `encode_array_to_sql()`
- `types/decode.rs` — array decoding helper `decode_array_from_sql()`

### Tests

- Encode/decode round-trip for each of the 9 types
- Empty array encode/decode
- Single-element array
- Multi-dimensional array rejection (error case)
- OID validation on decode

## Issue #2: Realtime Health Check

### Approach: Empty query protocol

Use PG's empty query (`""`) as the health check. This is the cheapest possible round-trip:
server responds with `EmptyQueryResponse` + `ReadyForQuery` (~50us).

### Implementation

**pool/health.rs** — add async function:

```rust
pub(crate) async fn check_alive(conn: &mut PgConnection) -> bool
```

Sends `""` via simple query protocol, drains until `ReadyForQuery`. Returns `false` on any I/O error.

**pool/mod.rs** — modify `acquire()` flow:

1. Pop idle connection from deque
2. Run fast checks (broken flag, idle timeout, max lifetime) — same as current
3. If `HealthCheckStrategy::Query` → call `check_alive()` on the connection
4. If alive → return connection; if dead → discard, create new one
5. `HealthCheckStrategy::Fast` and `None` → unchanged behavior

The `is_healthy()` method becomes `async` since it may need to send a query.

### Files Changed

- `pool/health.rs` — add `check_alive()` function
- `pool/mod.rs` — make health check async, integrate `HealthCheckStrategy::Query`

### Tests

- Unit test: `check_alive` returns true on healthy mock (protocol-level)
- Integration: pool with `HealthCheckStrategy::Query` acquires successfully
- Broken connection detection: `check_alive` returns false on closed stream
