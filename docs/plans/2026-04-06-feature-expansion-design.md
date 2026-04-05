# Feature Expansion Design — sentinel-driver v0.2+

**Date:** 2026-04-06
**Status:** Approved
**Goal:** Close feature gaps against sqlx, tokio-postgres, and diesel to make sentinel-driver the most complete PG-only driver in Rust.

---

## Context

sentinel-driver v0.1 has strong architectural advantages:
- Only Rust PG driver with true pipeline mode (PG 14+)
- Correct SCRAM-SHA-256 with SASLprep (sqlx has a bug, tokio-postgres silently falls back)
- Two-tier statement cache (HashMap + LRU-256) with metrics
- Single-task architecture (<0.5μs pool checkout, no channel overhead)
- Built-in query timeout + cancel (no other driver has both)
- Binary encoding by default

However, type coverage (18 scalar types) lags significantly behind competitors (25-30+), and several query engine capabilities are missing.

## Competitor Analysis Summary

| Area | sentinel | sqlx | tokio-postgres | diesel |
|------|:--------:|:----:|:--------------:|:------:|
| Pipeline mode | YES | NO | NO | NO |
| Correct SCRAM | YES | NO | partial | libpq |
| 2-tier stmt cache | YES | NO | NO | NO |
| Pool <0.5μs | YES | NO | NO | NO |
| Built-in timeout+cancel | YES | NO | NO | NO |
| Type coverage | 18 | 25+ | 30+ | 25+ |
| Row streaming | NO | YES | YES | YES |
| Simple query protocol | NO | YES | YES | YES |
| Portal/cursor | NO | NO | YES | NO |
| Certificate auth | NO | YES | YES | YES |
| Channel binding | NO | NO | YES | libpq |
| Advisory locks API | NO | YES | NO | NO |
| tracing spans | NO | NO | NO | NO |
| Benchmarks | NO | NO | NO | YES |

---

## Phase 1 — Type System Expansion

### Tier A: High-Impact Types

#### 1. NUMERIC / Decimal
- **Rust type:** `rust_decimal::Decimal`
- **OIDs:** NUMERIC (1700), NUMERIC[] (1231)
- **Feature flag:** `with-rust-decimal`
- **Encoding:** Binary format — sign(u16) + dscale(u16) + weight(i16) + ndigits(u16) + base-10000 digits
- **Why:** Every financial/accounting application requires exact decimal arithmetic

#### 2. INET / CIDR
- **Rust types:** `std::net::IpAddr` (built-in), `ipnetwork::IpNetwork` (feature-gated)
- **OIDs:** INET (869), CIDR (650), INET[] (1041), CIDR[] (651)
- **Feature flag:** `with-ipnetwork` for IpNetwork, IpAddr is always available
- **Encoding:** Binary — family(u8) + netmask(u8) + is_cidr(u8) + length(u8) + address bytes
- **Why:** Network applications, access control, IP allowlists

#### 3. Custom PostgreSQL Enums
- **Rust type:** User-defined `enum` with derive macro
- **OIDs:** Resolved at runtime via `pg_type` catalog query
- **Derive:** `#[derive(PgEnum)]` with `#[sentinel(name = "mood")]` and `#[sentinel(rename_all = "snake_case")]`
- **Encoding:** Text label as binary UTF-8 bytes
- **Why:** Nearly every production app has custom enums in the database

#### 4. Interval
- **Rust type:** `PgInterval { months: i32, days: i32, microseconds: i64 }` (built-in struct)
- **OIDs:** INTERVAL (1186), INTERVAL[] (1187)
- **Feature flag:** None (built-in)
- **Encoding:** Binary — microseconds(i64) + days(i32) + months(i32)
- **Optional:** `with-chrono` feature maps to `chrono::Duration` (lossy — no months)
- **Why:** Duration calculations, scheduling, recurring events

#### 5. Range Types
- **Rust type:** `PgRange<T> { start: Bound<T>, end: Bound<T> }`
- **OIDs:**
  - INT4RANGE (3904), INT4RANGE[] (3905)
  - INT8RANGE (3926), INT8RANGE[] (3927)
  - NUMRANGE (3906), NUMRANGE[] (3907)
  - TSRANGE (3908), TSRANGE[] (3909)
  - TSTZRANGE (3910), TSTZRANGE[] (3911)
  - DATERANGE (3912), DATERANGE[] (3913)
- **Encoding:** Binary — flags(u8) + lower bound + upper bound, where flags encode empty/inclusive/exclusive/infinite
- **Why:** Scheduling systems, availability windows, temporal queries, exclusion constraints

### Tier B: Completeness Types

#### 6. Money
- **Rust type:** `PgMoney(i64)` — value in cents
- **OIDs:** MONEY (790), MONEY[] (791)
- **Encoding:** Binary i64 (locale-dependent display, but storage is always integer cents)

#### 7. BIT / VARBIT
- **Rust type:** `BitVec` (via `bit-vec` crate)
- **OIDs:** BIT (1560), VARBIT (1562), BIT[] (1561), VARBIT[] (1563)
- **Feature flag:** `with-bit-vec`
- **Encoding:** Binary — bit_length(i32) + raw bytes

#### 8. HSTORE
- **Rust type:** `HashMap<String, Option<String>>`
- **OIDs:** HSTORE (extension, runtime OID)
- **Encoding:** Binary — count(i32) + pairs of length-prefixed strings (-1 for NULL value)

#### 9. MACADDR
- **Rust type:** `[u8; 6]` (built-in), `MacAddress` (feature-gated via `with-eui48`)
- **OIDs:** MACADDR (829), MACADDR[] (1040)
- **Encoding:** Binary 6 bytes

#### 10. Geometric Types
- **Rust types (built-in structs):**
  - `PgPoint { x: f64, y: f64 }` — POINT (600)
  - `PgLine { a: f64, b: f64, c: f64 }` — LINE (628)
  - `PgLSeg { start: PgPoint, end: PgPoint }` — LSEG (601)
  - `PgBox { upper_right: PgPoint, lower_left: PgPoint }` — BOX (603)
  - `PgPath { closed: bool, points: Vec<PgPoint> }` — PATH (602)
  - `PgPolygon { points: Vec<PgPoint> }` — POLYGON (604)
  - `PgCircle { center: PgPoint, radius: f64 }` — CIRCLE (718)
- **Optional:** `with-geo-types` feature for `geo_types` crate interop

#### 11. Composite Types
- **Rust type:** User-defined struct with derive macro
- **OIDs:** Resolved at runtime via `pg_type` + `pg_attribute` catalog queries
- **Derive:** `#[derive(PgComposite)]` with `#[sentinel(name = "address")]`
- **Encoding:** Binary — field_count(i32) + [oid(u32) + length(i32) + data] per field

#### 12. XML
- **Rust type:** `String`
- **OID:** XML (142)
- **Encoding:** UTF-8 text as binary bytes

#### 13. PG_LSN
- **Rust type:** `PgLsn(u64)`
- **OID:** PG_LSN (3220)
- **Encoding:** Binary u64

### Type Registration Architecture

```
types/
├── builtin.rs          # OID registry (expanded from 18 → 50+ entries)
├── encode.rs           # ToSql trait + impls
├── decode.rs           # FromSql trait + impls
├── numeric.rs          # NUMERIC binary encode/decode
├── network.rs          # INET/CIDR encode/decode
├── range.rs            # PgRange<T> encode/decode
├── interval.rs         # PgInterval encode/decode
├── geometric.rs        # Point, Line, LSeg, Box, Path, Polygon, Circle
├── money.rs            # PgMoney encode/decode
├── hstore.rs           # HSTORE encode/decode
├── macaddr.rs          # MACADDR encode/decode
├── bit.rs              # BIT/VARBIT encode/decode (feature-gated)
├── enums.rs            # Custom enum resolution + encode/decode
├── composite.rs        # Composite type resolution + encode/decode
├── xml.rs              # XML encode/decode
├── lsn.rs              # PG_LSN encode/decode
└── array.rs            # Array encode/decode (existing, extended)
```

---

## Phase 2 — Query Engine Enhancement

### 1. Row Streaming
- Add `Connection::query_stream(statement, params)` → `impl Stream<Item = Result<Row>>`
- Rows yielded as DataRow messages arrive from socket — no buffering
- Uses `futures_core::Stream` trait
- Pool variant: `Pool::query_stream()` — holds PooledConnection for stream lifetime
- Add `RowStream` struct wrapping the connection's read loop

### 2. Simple Query Protocol
- Add `Connection::simple_query(sql)` → `Vec<SimpleQueryMessage>`
- `SimpleQueryMessage` enum: `Row(SimpleQueryRow)` | `CommandComplete(u64)`
- `SimpleQueryRow` — text-only column access via `get::<&str>(idx)`
- Add `Connection::batch_execute(sql)` — discard results
- Uses `Query` frontend message (type 'Q')
- Supports multiple semicolon-separated statements

### 3. Portal / Cursor Support
- `Transaction::bind(statement, params)` → `Portal`
- `Transaction::query_portal(portal, max_rows)` → `Vec<Row>`
- Portal sends Bind (named portal) + Execute (with max_rows limit)
- Portal auto-closed on drop (sends Close message)
- Portal only valid within owning transaction

### 4. Pool Callbacks
- `PoolConfig::after_connect(async fn(&mut Connection) -> Result<()>)`
- `PoolConfig::before_acquire(async fn(&mut Connection) -> Result<bool>)`
- `PoolConfig::after_release(async fn(&mut Connection) -> Result<bool>)`
- Stored as `Arc<dyn Fn>` in pool config
- `after_connect` runs once per new connection (e.g., `SET search_path`)
- `before_acquire` can reject a connection (return false → try another)
- `after_release` can discard a connection (return false → don't return to pool)

### 5. Lazy Connect
- `Pool::connect_lazy(config)` — creates pool immediately, connects on first acquire
- Pool semaphore initialized but no TCP connections opened
- First `acquire()` triggers connection establishment

---

## Phase 3 — Security & Observability

### Security

#### 1. Certificate Authentication
- Add `ssl_client_cert: PathBuf` and `ssl_client_key: PathBuf` to `ConnectConfig`
- Also support PEM inline: `ssl_client_cert_pem: String`
- Load into rustls `ClientConfig` as client certificate chain
- Environment variables: `PGSSLCERT`, `PGSSLKEY`

#### 2. SCRAM-SHA-256-PLUS (Channel Binding)
- Extract server certificate from TLS session after handshake
- Compute SHA-256 hash of DER-encoded certificate → `tls-server-end-point` binding
- Send `SCRAM-SHA-256-PLUS` mechanism in SASLInitialResponse
- Include `c=` channel binding data in client-final message
- Fallback to `SCRAM-SHA-256` if server doesn't support PLUS

#### 3. Direct TLS (PG 17+)
- Add `SslNegotiation::Direct` option
- Skip SSLRequest message, connect TLS immediately
- Set ALPN protocol to `postgresql`
- Auto-detect: try direct first, fallback to SSLRequest on failure

### Observability

#### 4. Tracing Spans
- Wrap every query execution in `tracing::info_span!("pg.query", sql = %truncated_sql)`
- Record: `db.statement`, `db.operation`, `db.rows_affected`, `db.duration_ms`
- Pool operations: `tracing::debug_span!("pg.pool.acquire")`
- Feature flag: `with-tracing` (default enabled)

#### 5. Slow Query Logging
- Config: `slow_query_threshold: Option<Duration>` (default: None)
- When query exceeds threshold: `tracing::warn!(sql = %sql, elapsed_ms = %ms, "slow query detected")`
- Integrates with tracing spans

#### 6. Query Metrics Callback
- Config: `on_query: Option<Arc<dyn Fn(QueryMetrics) + Send + Sync>>`
- `QueryMetrics { sql: &str, elapsed: Duration, rows_affected: u64, cache_hit: bool }`
- Called after every query completion
- Use for Prometheus/OpenTelemetry integration

#### 7. Pool Metrics
- `Pool::metrics()` → `PoolMetrics`
- `PoolMetrics { active: u32, idle: u32, waiters: u32, total_created: u64, total_recycled: u64 }`
- Cheap snapshot (read atomics)

### Extras

#### 8. Advisory Locks (RAII)
- `PgAdvisoryLock::new(key: i64)` or `PgAdvisoryLock::from_str(name: &str)` (HKDF-SHA256 → i64)
- `lock.acquire(&conn)` → `PgAdvisoryLockGuard` (session-scoped)
- `lock.try_acquire(&conn)` → `Option<PgAdvisoryLockGuard>` (non-blocking)
- Guard releases on drop via `pg_advisory_unlock`
- Also: `acquire_xact(&txn)` for transaction-scoped locks (auto-released on commit/rollback)

#### 9. Criterion Benchmarks
- `benches/` directory with criterion benchmark suite
- Benchmarks: simple_select, parametrized_query, batch_pipeline, bulk_copy_insert, pool_checkout, type_encode_decode
- CI integration: run benchmarks on PR, compare against baseline
- Proves performance claims in README

---

## Feature Flags Summary

```toml
[features]
default = ["derive", "with-tracing"]
derive = ["sentinel-derive"]
with-tracing = ["tracing"]
with-rust-decimal = ["rust_decimal"]
with-ipnetwork = ["ipnetwork"]
with-bit-vec = ["bit-vec"]
with-eui48 = ["eui48"]
with-geo-types = ["geo-types"]
```

Built-in (no feature flag needed): interval, range, money, geometric structs, hstore, macaddr, xml, pg_lsn, enums, composites.

---

## Implementation Order

1. **Phase 1A** — NUMERIC, INET/CIDR, Interval, Range (highest impact types)
2. **Phase 1B** — Custom Enums, Composite Types (require runtime OID resolution)
3. **Phase 1C** — Geometric, Money, BIT, HSTORE, MACADDR, XML, PG_LSN (completeness)
4. **Phase 2A** — Row Streaming, Simple Query Protocol (most requested capabilities)
5. **Phase 2B** — Portal/Cursor, Pool Callbacks, Lazy Connect
6. **Phase 3A** — Certificate Auth, SCRAM-PLUS, Direct TLS
7. **Phase 3B** — Tracing Spans, Slow Query Logging, Metrics Callback, Pool Metrics
8. **Phase 3C** — Advisory Locks, Criterion Benchmarks

Each sub-phase is independently shippable as a minor version bump.

---

## Success Criteria

After full implementation:
- Type coverage: 50+ OIDs (matching or exceeding all competitors)
- Only Rust PG driver with: pipeline + streaming + cursor + advisory locks + correct SCRAM-PLUS
- Benchmark suite proving performance claims
- Enterprise-ready: cert auth, channel binding, direct TLS, full observability
- Feature parity or superiority in every category vs sqlx, tokio-postgres, and diesel
