# Phase 4A: Critical Protocol Gaps

**Date:** 2026-04-15
**Status:** Approved
**Goal:** Close 5 critical driver-level gaps found in competitor comparison.

## 1. simple_query Returning Rows

**Current:** `simple_query(sql)` → `Vec<CommandResult>` — discards row data.

**Fix:** Return `Vec<SimpleQueryMessage>` with text-format row access.

```rust
pub enum SimpleQueryMessage {
    Row(SimpleQueryRow),
    CommandComplete(u64),
}

pub struct SimpleQueryRow {
    columns: Vec<Option<String>>,  // text format, None for NULL
}

impl SimpleQueryRow {
    pub fn get(&self, idx: usize) -> Option<&str>;
    pub fn try_get(&self, idx: usize) -> Result<&str>;
    pub fn len(&self) -> usize;
}
```

**Protocol:** Already sends Query message — just need to parse RowDescription + DataRow messages in text format instead of skipping them.

## 2. GenericClient Trait

Write code generic over Connection, Transaction, and PooledConnection.

```rust
#[async_trait]
pub trait GenericClient {
    async fn query(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>>;
    async fn query_one(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row>;
    async fn query_opt(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>>;
    async fn execute(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64>;
    async fn simple_query(&mut self, sql: &str) -> Result<Vec<SimpleQueryMessage>>;
}
```

Implement for Connection, Transaction<'_>, and PooledConnection. No new dependencies needed — use manual Future impl or add `async-trait` crate.

## 3. query_typed() — Skip Prepare

Execute queries without a prepare round-trip by providing types inline.

```rust
conn.query_typed(
    "SELECT $1::int4 + $2::int4",
    &[(&1i32, Oid::INT4), (&2i32, Oid::INT4)],
).await?;
```

**Protocol:** Send Parse(unnamed) + Bind + Describe + Execute + Sync in one batch. The param types are specified in Parse message directly — server doesn't need a separate Describe round-trip.

**Methods:** `query_typed()`, `query_typed_one()`, `query_typed_opt()`, `execute_typed()`.

## 4. Json<T> Wrapper

Serialize/deserialize arbitrary types to JSONB.

```rust
pub struct Json<T>(pub T);

impl<T: serde::Serialize> ToSql for Json<T> {
    fn oid(&self) -> Oid { Oid::JSONB }
    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(1); // JSONB version byte
        serde_json::to_writer(buf.writer(), &self.0)?;
        Ok(())
    }
}

impl<T: serde::de::DeserializeOwned> FromSql for Json<T> {
    fn oid() -> Oid { Oid::JSONB }
    fn from_sql(buf: &[u8]) -> Result<Self> {
        let data = if buf.first() == Some(&1) { &buf[1..] } else { buf };
        Ok(Json(serde_json::from_slice(data)?))
    }
}
```

**Dependencies:** Add `serde = { version = "1", optional = true }` and `serde_json = { version = "1", optional = true }`.
**Feature flag:** `with-serde-json = ["dep:serde", "dep:serde_json"]`

## 5. Infinity Dates/Timestamps

Handle PG `+infinity` / `-infinity` for temporal types.

**PG wire format:**
- `i64::MAX` (0x7FFFFFFFFFFFFFFF) = positive infinity
- `i64::MIN` (0x8000000000000000) = negative infinity
- Same for date (i32::MAX / i32::MIN as days)

**Approach:** Add special-case handling in existing chrono encode/decode:
- Decode: `i64::MAX` → `NaiveDateTime::MAX`, `i64::MIN` → `NaiveDateTime::MIN`
- Encode: `NaiveDateTime::MAX` → `i64::MAX`, `NaiveDateTime::MIN` → `i64::MIN`
- Same pattern for `NaiveDate`, `DateTime<Utc>`

No new types needed — extends existing impls.

## Files

| Item | Files to Create/Modify |
|------|----------------------|
| simple_query rows | `lib.rs` (or connection method), new `SimpleQueryRow` type |
| GenericClient | Create `client.rs`, modify `lib.rs` |
| query_typed | Modify `lib.rs` + `pipeline/` |
| Json<T> | Create `types/json.rs`, modify `Cargo.toml` |
| Infinity | Modify `types/encode.rs` + `types/decode.rs` |

## Dependencies

- `serde = { version = "1", optional = true }`
- `serde_json = { version = "1", optional = true }`
- Feature: `with-serde-json = ["dep:serde", "dep:serde_json"]`
