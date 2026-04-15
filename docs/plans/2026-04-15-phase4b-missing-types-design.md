# Phase 4B: Missing Types & Array Gaps

**Date:** 2026-04-15
**Status:** Approved
**Goal:** Close remaining type coverage gaps against sqlx and tokio-postgres.

## 1. Missing Array OIDs + Impls

Add array encode/decode for types that have scalar support but no array variant:

| Type | Array OID | Notes |
|------|-----------|-------|
| JSON | 199 | serde_json::Value |
| JSONB | 3807 | serde_json::Value |
| TIMESTAMP | 1115 | NaiveDateTime |
| TIMESTAMPTZ | 1185 | DateTime<Utc> |
| DATE | 1182 | NaiveDate |
| TIME | 1183 | NaiveTime |
| BYTEA | 1001 | Vec<u8> |
| POINT | 1017 | geometric::Point |
| CIRCLE | 719 | geometric::Circle |

## 2. TIMETZ

- OID: 1266, Array: 1270
- `PgTimeTz { time: NaiveTime, offset_seconds: i32 }`
- Wire: i64 microseconds + i32 UTC offset (seconds, negated)
- File: `types/timetz.rs`

## 3. MACADDR8 (PG 10+)

- OID: 774, Array: 775
- `PgMacAddr8([u8; 8])` — 8-byte extended MAC
- Wire: 8 bytes raw
- File: add to `types/network.rs`

## 4. Multirange Types (PG 14+)

6 types + 6 array variants = 12 new OIDs:

| Type | OID | Array OID |
|------|-----|-----------|
| INT4MULTIRANGE | 4451 | 6150 |
| INT8MULTIRANGE | 4536 | 6157 |
| NUMMULTIRANGE | 4532 | 6151 |
| TSMULTIRANGE | 4533 | 6152 |
| TSTZMULTIRANGE | 4534 | 6153 |
| DATEMULTIRANGE | 4535 | 6155 |

- `PgMultirange<T>(pub Vec<PgRange<T>>)`
- Wire: count(i32) + [length(i32) + range_bytes] per range
- File: `types/multirange.rs`

## 5. Infinity Dates/Timestamps

PG uses `i64::MAX`/`i64::MIN` for `+/-infinity` in temporal types.

- Encode: `NaiveDateTime::MAX` → `i64::MAX`, `MIN` → `i64::MIN`
- Decode: `i64::MAX` → `NaiveDateTime::MAX`, `i64::MIN` → `NaiveDateTime::MIN`
- Same for NaiveDate (i32::MAX/MIN), DateTime<Utc>
- Modify: `types/encode.rs`, `types/decode.rs`

## 6. serde_json::Value Direct Support

Add `ToSql`/`FromSql` for `serde_json::Value` directly (OID JSONB) without needing `Json<T>` wrapper.

- Feature-gated under existing `with-serde-json`
- Modify: `types/json.rs`

## Expected OID Count After: ~80+
