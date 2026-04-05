# Phase 1 Implementation Plan: Array Types + Health Check

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement PostgreSQL array type encode/decode (`Vec<T>`) and realtime pool health checks to unblock sentinel-core integration.

**Architecture:** Macro-based concrete `ToSql`/`FromSql` implementations for 9 array types using PG binary array wire format. Pool health check via empty query protocol on checkout when `HealthCheckStrategy::Query` is configured.

**Tech Stack:** Rust, bytes crate (existing), tokio (existing for async health check)

---

### Task 1: Array encoding helper + macro

**Files:**
- Modify: `crates/sentinel-driver/src/types/encode.rs` (append after line 195)
- Modify: `crates/sentinel-driver/src/types/mod.rs` (add array OID mapping)
- Test: `tests/core/types_encode.rs`

**Step 1: Write failing tests for array encoding**

Add to `tests/core/types_encode.rs`:

```rust
#[test]
fn test_encode_vec_i32() {
    use sentinel_driver::types::Oid;

    let v: Vec<i32> = vec![1, 2, 3];
    let mut buf = BytesMut::new();
    v.to_sql(&mut buf).unwrap();

    // Verify OID
    assert_eq!(v.oid(), Oid::INT4_ARRAY);

    // Verify binary format header
    let ndim = i32::from_be_bytes(buf[0..4].try_into().unwrap());
    let has_null = i32::from_be_bytes(buf[4..8].try_into().unwrap());
    let elem_oid = u32::from_be_bytes(buf[8..12].try_into().unwrap());
    let dim_len = i32::from_be_bytes(buf[12..16].try_into().unwrap());
    let dim_lbound = i32::from_be_bytes(buf[16..20].try_into().unwrap());

    assert_eq!(ndim, 1);
    assert_eq!(has_null, 0);
    assert_eq!(elem_oid, Oid::INT4.0);
    assert_eq!(dim_len, 3);
    assert_eq!(dim_lbound, 1);
}

#[test]
fn test_encode_vec_empty() {
    let v: Vec<i32> = vec![];
    let mut buf = BytesMut::new();
    v.to_sql(&mut buf).unwrap();

    let ndim = i32::from_be_bytes(buf[0..4].try_into().unwrap());
    assert_eq!(ndim, 0);
    // Empty array: ndim=0, has_null=0, elem_oid
    assert_eq!(buf.len(), 12);
}

#[test]
fn test_encode_vec_string() {
    use sentinel_driver::types::Oid;

    let v: Vec<String> = vec!["hello".into(), "world".into()];
    let mut buf = BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(v.oid(), Oid::TEXT_ARRAY);
}

#[test]
fn test_encode_vec_bool() {
    use sentinel_driver::types::Oid;

    let v: Vec<bool> = vec![true, false, true];
    let mut buf = BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(v.oid(), Oid::BOOL_ARRAY);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --workspace test_encode_vec -- --no-capture`
Expected: FAIL — `ToSql` is not implemented for `Vec<i32>`

**Step 3: Implement array encoding**

In `crates/sentinel-driver/src/types/encode.rs`, append after the UUID impl (line 195):

```rust
// ── Array types ─────────────────────────────────────

/// Encode a Vec<T> as a PostgreSQL 1-D binary array.
///
/// Wire format:
/// - i32 ndim (0 for empty, 1 for non-empty)
/// - i32 has_null (always 0 — nullable elements not supported)
/// - u32 element_oid
/// - i32 dim_len (array length)
/// - i32 dim_lbound (always 1, PG arrays are 1-based)
/// - for each element: i32 data_len + encoded bytes
fn encode_array<T: ToSql>(
    vec: &[T],
    element_oid: Oid,
    buf: &mut BytesMut,
) -> Result<()> {
    if vec.is_empty() {
        buf.put_i32(0); // ndim = 0
        buf.put_i32(0); // has_null = 0
        buf.put_u32(element_oid.0);
        return Ok(());
    }

    buf.put_i32(1); // ndim = 1
    buf.put_i32(0); // has_null = 0
    buf.put_u32(element_oid.0);
    buf.put_i32(vec.len() as i32); // dim_len
    buf.put_i32(1); // dim_lbound (1-based)

    for elem in vec {
        let len_pos = buf.len();
        buf.put_i32(0); // placeholder for element length
        let data_start = buf.len();
        elem.to_sql(buf)?;
        let data_len = (buf.len() - data_start) as i32;
        buf[len_pos..len_pos + 4].copy_from_slice(&data_len.to_be_bytes());
    }

    Ok(())
}

/// Macro to implement ToSql for Vec<T> for a specific element type.
macro_rules! impl_array_to_sql {
    ($elem_ty:ty, $array_oid:expr, $elem_oid:expr) => {
        impl ToSql for Vec<$elem_ty> {
            fn oid(&self) -> Oid {
                $array_oid
            }

            fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
                encode_array(self, $elem_oid, buf)
            }
        }
    };
}

impl_array_to_sql!(bool,         Oid::BOOL_ARRAY,   Oid::BOOL);
impl_array_to_sql!(i16,          Oid::INT2_ARRAY,   Oid::INT2);
impl_array_to_sql!(i32,          Oid::INT4_ARRAY,   Oid::INT4);
impl_array_to_sql!(i64,          Oid::INT8_ARRAY,   Oid::INT8);
impl_array_to_sql!(f32,          Oid::FLOAT4_ARRAY, Oid::FLOAT4);
impl_array_to_sql!(f64,          Oid::FLOAT8_ARRAY, Oid::FLOAT8);
impl_array_to_sql!(String,       Oid::TEXT_ARRAY,    Oid::TEXT);
impl_array_to_sql!(uuid::Uuid,   Oid::UUID_ARRAY,   Oid::UUID);
```

Also add `ToSql` for `Vec<&str>` (encode-only, cannot use macro due to lifetime):

```rust
impl ToSql for Vec<&str> {
    fn oid(&self) -> Oid {
        Oid::TEXT_ARRAY
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        encode_array(self, Oid::TEXT, buf)
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --workspace test_encode_vec -- --no-capture`
Expected: all 4 tests PASS

**Step 5: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/sentinel-driver/src/types/encode.rs tests/core/types_encode.rs
git commit -m "feat(types): add ToSql for Vec<T> array encoding (#5)"
```

---

### Task 2: Array decoding helper + macro

**Files:**
- Modify: `crates/sentinel-driver/src/types/decode.rs` (append after line 191)
- Test: `tests/core/types_decode.rs`

**Step 1: Write failing tests for array round-trip decoding**

Add to `tests/core/types_decode.rs`:

```rust
#[test]
fn test_roundtrip_vec_i32() {
    roundtrip(&vec![1i32, 2, 3]);
    roundtrip(&vec![i32::MIN, 0, i32::MAX]);
}

#[test]
fn test_roundtrip_vec_empty_i32() {
    roundtrip(&Vec::<i32>::new());
}

#[test]
fn test_roundtrip_vec_i16() {
    roundtrip(&vec![1i16, -1, 0]);
}

#[test]
fn test_roundtrip_vec_i64() {
    roundtrip(&vec![1i64, i64::MAX]);
}

#[test]
fn test_roundtrip_vec_f32() {
    roundtrip(&vec![1.0f32, 3.14, -0.5]);
}

#[test]
fn test_roundtrip_vec_f64() {
    roundtrip(&vec![std::f64::consts::PI, 0.0]);
}

#[test]
fn test_roundtrip_vec_bool() {
    roundtrip(&vec![true, false, true]);
}

#[test]
fn test_roundtrip_vec_string() {
    roundtrip(&vec![String::from("hello"), String::from("world")]);
    roundtrip(&vec![String::from("")]);
}

#[test]
fn test_roundtrip_vec_uuid() {
    let id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    roundtrip(&vec![id, uuid::Uuid::nil()]);
}

#[test]
fn test_decode_array_multidim_rejected() {
    // Manually construct a 2-D array header
    use bytes::BufMut;
    let mut buf = BytesMut::new();
    buf.put_i32(2); // ndim = 2 (not supported)
    buf.put_i32(0);
    buf.put_u32(sentinel_driver::types::Oid::INT4.0);
    buf.put_i32(2); buf.put_i32(1);
    buf.put_i32(2); buf.put_i32(1);

    let result = Vec::<i32>::from_sql(&buf);
    assert!(result.is_err());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --workspace test_roundtrip_vec -- --no-capture`
Expected: FAIL — `FromSql` is not implemented for `Vec<i32>`

**Step 3: Implement array decoding**

In `crates/sentinel-driver/src/types/decode.rs`, append after the UUID impl (line 191):

```rust
// ── Array types ─────────────────────────────────────

/// Decode a PostgreSQL 1-D binary array into Vec<T>.
fn decode_array<T: FromSql>(buf: &[u8], expected_elem_oid: Oid) -> Result<Vec<T>> {
    if buf.len() < 12 {
        return Err(Error::Decode("array: header too short".into()));
    }

    let ndim = i32::from_be_bytes(buf[0..4].try_into().map_err(|_| {
        Error::Decode("array: invalid ndim".into())
    })?);

    let _has_null = i32::from_be_bytes(buf[4..8].try_into().map_err(|_| {
        Error::Decode("array: invalid has_null".into())
    })?);

    let elem_oid = u32::from_be_bytes(buf[8..12].try_into().map_err(|_| {
        Error::Decode("array: invalid element oid".into())
    })?);

    if ndim == 0 {
        return Ok(Vec::new());
    }

    if ndim != 1 {
        return Err(Error::Decode(format!(
            "array: multi-dimensional arrays not supported (ndim={ndim})"
        )));
    }

    if elem_oid != expected_elem_oid.0 {
        return Err(Error::Decode(format!(
            "array: expected element OID {}, got {elem_oid}",
            expected_elem_oid.0
        )));
    }

    if buf.len() < 20 {
        return Err(Error::Decode("array: dimension header too short".into()));
    }

    let dim_len = i32::from_be_bytes(buf[12..16].try_into().map_err(|_| {
        Error::Decode("array: invalid dim_len".into())
    })?) as usize;

    // dim_lbound at buf[16..20] — skip, not needed for decoding

    let mut offset = 20;
    let mut result = Vec::with_capacity(dim_len);

    for _ in 0..dim_len {
        if offset + 4 > buf.len() {
            return Err(Error::Decode("array: unexpected end of data".into()));
        }

        let elem_len = i32::from_be_bytes(buf[offset..offset + 4].try_into().map_err(|_| {
            Error::Decode("array: invalid element length".into())
        })?);
        offset += 4;

        if elem_len < 0 {
            return Err(Error::Decode(
                "array: NULL elements not supported".into(),
            ));
        }

        let elem_len = elem_len as usize;
        if offset + elem_len > buf.len() {
            return Err(Error::Decode("array: element data truncated".into()));
        }

        let elem = T::from_sql(&buf[offset..offset + elem_len])?;
        result.push(elem);
        offset += elem_len;
    }

    Ok(result)
}

/// Macro to implement FromSql for Vec<T> for a specific element type.
macro_rules! impl_array_from_sql {
    ($elem_ty:ty, $array_oid:expr, $elem_oid:expr) => {
        impl FromSql for Vec<$elem_ty> {
            fn oid() -> Oid {
                $array_oid
            }

            fn from_sql(buf: &[u8]) -> Result<Self> {
                decode_array::<$elem_ty>(buf, $elem_oid)
            }
        }
    };
}

impl_array_from_sql!(bool,         Oid::BOOL_ARRAY,   Oid::BOOL);
impl_array_from_sql!(i16,          Oid::INT2_ARRAY,   Oid::INT2);
impl_array_from_sql!(i32,          Oid::INT4_ARRAY,   Oid::INT4);
impl_array_from_sql!(i64,          Oid::INT8_ARRAY,   Oid::INT8);
impl_array_from_sql!(f32,          Oid::FLOAT4_ARRAY, Oid::FLOAT4);
impl_array_from_sql!(f64,          Oid::FLOAT8_ARRAY, Oid::FLOAT8);
impl_array_from_sql!(String,       Oid::TEXT_ARRAY,    Oid::TEXT);
impl_array_from_sql!(uuid::Uuid,   Oid::UUID_ARRAY,   Oid::UUID);
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --workspace test_roundtrip_vec test_decode_array -- --no-capture`
Expected: all 10 tests PASS

**Step 5: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/sentinel-driver/src/types/decode.rs tests/core/types_decode.rs
git commit -m "feat(types): add FromSql for Vec<T> array decoding (#5)"
```

---

### Task 3: Realtime health check implementation

**Files:**
- Modify: `crates/sentinel-driver/src/pool/health.rs`
- Modify: `crates/sentinel-driver/src/pool/mod.rs`
- Test: `tests/core/pool_health.rs` (create)
- Modify: `tests/core/mod.rs` (register new test module)

**Step 1: Write failing test for check_alive**

Create `tests/core/pool_health.rs`:

```rust
use sentinel_driver::pool::health::HealthCheckStrategy;

#[test]
fn test_health_check_strategy_variants() {
    // Verify the Query variant exists and is usable
    let strategy = HealthCheckStrategy::Query;
    assert_eq!(strategy, HealthCheckStrategy::Query);

    let fast = HealthCheckStrategy::Fast;
    assert_eq!(fast, HealthCheckStrategy::Fast);

    let none = HealthCheckStrategy::None;
    assert_eq!(none, HealthCheckStrategy::None);
}
```

Add to `tests/core/mod.rs`:

```rust
mod pool_health;
```

**Step 2: Run test to verify it passes (baseline)**

Run: `cargo test --workspace test_health_check_strategy -- --no-capture`
Expected: PASS (this establishes that the enum is accessible)

**Step 3: Implement check_alive in health.rs**

Add to `crates/sentinel-driver/src/pool/health.rs`:

```rust
use crate::connection::stream::PgConnection;
use crate::protocol::backend::BackendMessage;
use crate::protocol::frontend;

/// Check if a connection is still alive by sending an empty query.
///
/// Sends `""` via simple query protocol. The server responds with
/// `EmptyQueryResponse` + `ReadyForQuery`. Returns `false` on any error.
/// Cost: ~50μs round-trip.
pub(crate) async fn check_alive(conn: &mut PgConnection) -> bool {
    frontend::query(conn.write_buf(), "");

    if conn.send().await.is_err() {
        return false;
    }

    // Drain until ReadyForQuery
    loop {
        match conn.recv().await {
            Ok(BackendMessage::ReadyForQuery { .. }) => return true,
            Ok(_) => continue,
            Err(_) => return false,
        }
    }
}
```

**Step 4: Integrate into Pool::acquire**

Modify `crates/sentinel-driver/src/pool/mod.rs`:

Replace the `is_healthy` method and the `acquire` method's idle connection handling to support async health checks.

Change `is_healthy` to be a quick sync check (broken/idle/lifetime only):

```rust
fn is_fresh(&self, meta: &ConnectionMeta) -> bool {
    if meta.is_broken {
        return false;
    }
    if let Some(timeout) = self.shared.pool_config.idle_timeout {
        if meta.is_idle_expired(timeout) {
            return false;
        }
    }
    if let Some(lifetime) = self.shared.pool_config.max_lifetime {
        if meta.is_lifetime_expired(lifetime) {
            return false;
        }
    }
    true
}
```

Update `acquire()` to call `check_alive` when strategy is `Query`:

```rust
if let Some(idle) = idle_conn {
    if self.is_fresh(&idle.meta) {
        // If Query strategy, verify connection is alive
        let mut conn = idle.conn;
        if self.shared.pool_config.health_check == HealthCheckStrategy::Query {
            if !health::check_alive(&mut conn).await {
                debug!("idle connection failed health check, creating new one");
                self.decrement_count().await;
                let (conn, meta) = self.create_connection().await?;
                return Ok(PooledConnection {
                    conn: Some(conn),
                    meta,
                    shared: Arc::clone(&self.shared),
                });
            }
        }
        debug!("reusing idle connection");
        Ok(PooledConnection {
            conn: Some(conn),
            meta: idle.meta,
            shared: Arc::clone(&self.shared),
        })
    } else {
        debug!("idle connection expired, creating new one");
        self.decrement_count().await;
        let (conn, meta) = self.create_connection().await?;
        Ok(PooledConnection {
            conn: Some(conn),
            meta,
            shared: Arc::clone(&self.shared),
        })
    }
}
```

**Step 5: Run all tests**

Run: `cargo test --workspace -- --no-capture`
Expected: all tests PASS

**Step 6: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS

**Step 7: Commit**

```bash
git add crates/sentinel-driver/src/pool/health.rs crates/sentinel-driver/src/pool/mod.rs tests/core/pool_health.rs tests/core/mod.rs
git commit -m "feat(pool): implement Query health check strategy (#2)"
```

---

### Task 4: Final verification and formatting

**Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: all tests PASS

**Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: PASS

**Step 3: Run formatting check**

Run: `cargo fmt --all -- --check`
Expected: PASS (no formatting issues)

**Step 4: Verify no dead code warnings**

Run: `cargo check --workspace 2>&1`
Expected: no warnings

**Step 5: Commit any final fixes if needed**

```bash
git commit -m "chore: Phase 1 final cleanup"
```
