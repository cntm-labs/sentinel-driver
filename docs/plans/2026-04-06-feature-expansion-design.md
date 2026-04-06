# Feature Expansion Design — sentinel-driver v0.2+

**Date:** 2026-04-06
**Last Updated:** 2026-04-06
**Status:** In Progress (Phase 1A complete on feature branch)
**Goal:** Close feature gaps against sqlx, tokio-postgres, and diesel to make sentinel-driver the most complete PG-only driver in Rust.

---

## Current Status

### What's shipped (main branch):
- 26 OIDs (18 scalar + 9 array)
- Pipeline mode, COPY, LISTEN/NOTIFY, Transactions
- Query timeout + cancel, Two-tier statement cache
- Connection pool (<0.5μs checkout)
- SCRAM-SHA-256 (correct SASLprep), TLS (5 modes)

### What's ready to merge (feature/phase1a-types):
- 62 OIDs (+36 new) — Interval, INET/CIDR/MACADDR, NUMERIC/Decimal, Range<T>, Geometric, Money, XML, PG_LSN
- 8 new type modules (1,157 LOC added)
- Optional `with-rust-decimal` feature flag
- Full test coverage for all new types

### What's still planned:
- Phase 1B: Custom Enums, Composite Types
- Phase 1C: BIT/VARBIT, HSTORE (remaining completeness types)
- Phase 2: Row Streaming, Simple Query, Portal/Cursor, Pool Callbacks
- Phase 3: Certificate Auth, SCRAM-PLUS, Direct TLS, Observability, Benchmarks

---

## Competitor Comparison (Updated)

### Architecture & Protocol

| Feature | sentinel | sqlx | tokio-postgres | diesel | sea-orm |
|---------|:--------:|:----:|:--------------:|:------:|:-------:|
| PG-only (no abstraction tax) | **YES** | NO (multi-DB) | YES | NO (multi-DB) | NO (multi-DB) |
| Pipeline mode (PG 14+) | **YES** | NO | NO* | NO | NO |
| Binary encoding default | **YES** | YES | YES | COPY only | NO (text) |
| Single-task architecture | **YES** | YES | NO (2-task) | sync | via sqlx |
| Zero-copy row access | **YES** | YES | YES | NO | NO |
| Simple query protocol | YES | YES | YES | YES | YES |

> \*tokio-postgres has "implicit pipelining" via concurrent futures but not PG pipeline protocol

### Connection Pool

| Feature | sentinel | sqlx | tokio-postgres | diesel | sea-orm |
|---------|:--------:|:----:|:--------------:|:------:|:-------:|
| Built-in pool | **YES** | YES | via deadpool/bb8 | via r2d2 | via sqlx |
| Pool checkout latency | **<0.5μs** | ~μs | ~μs | ~μs | via sqlx |
| Health check strategies | **3 modes** | ping only | recycle only | ping | ping |
| Pool callbacks | NO | **YES** (3) | deadpool recycle | NO | **YES** |
| Lazy connect | NO | **YES** | NO | NO | **YES** |
| Pool metrics | basic | NO | NO | NO | NO |

### Authentication & TLS

| Feature | sentinel | sqlx | tokio-postgres | diesel |
|---------|:--------:|:----:|:--------------:|:------:|
| SCRAM-SHA-256 | **YES** (correct SASLprep) | YES (buggy) | YES (fallback) | libpq |
| SCRAM-SHA-256-PLUS | NO | NO | **YES** | libpq |
| MD5 auth | YES | YES | YES | YES |
| Certificate auth | NO | **YES** | **YES** | YES |
| rustls TLS | **YES** | YES | third-party | NO |
| SSL modes | **5** (all) | 4 (Allow broken) | 3 | libpq |
| Direct TLS (PG 17+) | NO | NO | **YES** | NO |

### Prepared Statements & Cache

| Feature | sentinel | sqlx | tokio-postgres | diesel |
|---------|:--------:|:----:|:--------------:|:------:|
| Statement cache | **2-tier** (HashMap+LRU) | LRU-100 | NO (deadpool: unbounded) | compile-time |
| Cache metrics | **YES** | NO | NO | NO |
| Cache capacity | 256 (configurable) | 100 (fixed) | unbounded | N/A |

### Type System Coverage

| Type Category | sentinel (main) | sentinel (phase1a) | sqlx | tokio-postgres | diesel |
|---------------|:---:|:---:|:---:|:---:|:---:|
| **Primitives** (bool, int, float) | 6 | 6 | 7 | 7 | 7 |
| **String** (text, varchar, char) | 3 | 3 | 5+ | 5+ | 3 |
| **Binary** (bytea) | YES | YES | YES | YES | YES |
| **Temporal** (date, time, timestamp, timestamptz) | 4 | 4 | 6+ | 6+ | 4 |
| **Interval** | NO | **YES** | YES | YES | NO |
| **UUID** | YES | YES | YES | YES | YES |
| **JSON/JSONB** | YES | YES | YES | YES | YES |
| **NUMERIC/Decimal** | NO | **YES** (feature) | YES | third-party | YES |
| **INET/CIDR** | NO | **YES** | YES | YES | YES |
| **MACADDR** | NO | **YES** | YES | YES | YES |
| **Range types** (6 variants) | NO | **YES** | YES | YES | YES |
| **Geometric** (point, line, etc.) | NO | **YES** (7 types) | YES (7) | YES | NO |
| **Money** | NO | **YES** | YES | YES | YES |
| **XML** | NO | **YES** | NO | NO | NO |
| **PG_LSN** | NO | **YES** | NO | YES | YES |
| **Custom Enums** | NO | NO | YES | YES | YES |
| **Composite Types** | NO | NO | YES | YES | YES |
| **BIT/VARBIT** | NO | NO | YES | YES | NO |
| **HSTORE** | NO | NO | YES | YES | NO |
| **Arrays** | 9 types | 20+ types | any T (1D) | any T | any T |
| **Total OIDs** | **26** | **62** | ~70+ | ~200 | ~50+ |

### Query Execution

| Feature | sentinel | sqlx | tokio-postgres | diesel | sea-orm |
|---------|:--------:|:----:|:--------------:|:------:|:-------:|
| Query timeout (built-in) | **YES** | NO | NO | NO | server-side |
| Cancel query | **YES** | NO | **YES** | libpq | NO |
| Timeout + Cancel combined | **YES** | NO | NO | NO | NO |
| Row streaming | NO | **YES** | **YES** | YES | YES |
| Portal/Cursor | NO | NO | **YES** | NO | NO |
| COPY IN/OUT (text) | **YES** | YES | YES | YES (2.2+) | NO |
| COPY IN/OUT (binary) | **YES** | raw only | **YES** | **YES** | NO |
| LISTEN/NOTIFY | **YES** | **YES** (auto-reconnect) | manual | NO | NO |
| Pipeline batch | **YES** | NO | NO | NO | NO |
| Compile-time SQL check | NO | **YES** (query!()) | NO | **YES** | NO |

### Transactions

| Feature | sentinel | sqlx | tokio-postgres | diesel | sea-orm |
|---------|:--------:|:----:|:--------------:|:------:|:-------:|
| Isolation levels (4) | **YES** | custom SQL | **YES** | YES | YES |
| Savepoints | **YES** | auto-nested | **YES** | YES | YES |
| Read-only / Deferrable | **YES** | custom SQL | **YES** | NO | YES |

### Observability & DX

| Feature | sentinel | sqlx | tokio-postgres | diesel | sea-orm |
|---------|:--------:|:----:|:--------------:|:------:|:-------:|
| tracing integration | basic | log levels | log crate | NO | **YES** (spans) |
| Query metrics/callbacks | cache only | NO | NO | NO | **YES** |
| Slow query logging | NO | **YES** | NO | NO | **YES** |
| Advisory locks (RAII) | NO | **YES** | NO | NO | NO |
| Migrations | NO | **YES** | NO | **YES** | **YES** |
| Derive macros | **YES** (3) | **YES** (3+) | **YES** (2) | **YES** | via ORM |
| Mock driver | NO | NO | NO | NO | **YES** |
| Benchmarks | NO | NO | NO | YES | NO |

---

## Competitive Advantages (Unique to sentinel-driver)

| Advantage | Detail | Impact |
|-----------|--------|--------|
| **Only pipeline mode in Rust** | True PG 14+ protocol — batch N queries in 1 round-trip | 2-5x throughput for batch workloads |
| **Correct SCRAM-SHA-256** | sqlx has SASLprep bug on passwords, tokio-pg falls back silently | Non-ASCII passwords actually work |
| **Two-tier statement cache** | HashMap (permanent) + LRU-256 (ad-hoc) with hit/miss metrics | ~99% cache hit rate, zero eviction for hot queries |
| **Timeout + Cancel built-in** | No other driver combines both — query timeout auto-cancels on server | Prevents runaway queries without orphan server processes |
| **Pool <0.5μs checkout** | Single-task = no channel overhead vs tokio-pg's 2-task model | Negligible pool overhead |
| **PG-only design** | No multi-DB abstraction tax (unlike sqlx, diesel, sea-orm) | Smaller binary, simpler API, faster compilation |

---

## Competitive Weaknesses (Gaps to Close)

### Critical (blocks production adoption):
| Gap | Who Has It | Priority |
|-----|-----------|----------|
| Custom PG Enums | sqlx, tokio-pg, diesel | **Phase 1B** |
| Composite Types | sqlx, tokio-pg, diesel | **Phase 1B** |
| Row streaming | sqlx, tokio-pg | **Phase 2A** |

### Important (enterprise/production):
| Gap | Who Has It | Priority |
|-----|-----------|----------|
| Certificate auth | sqlx, tokio-pg | Phase 3A |
| SCRAM-SHA-256-PLUS | tokio-pg | Phase 3A |
| Pool callbacks | sqlx, sea-orm | Phase 2B |
| Advisory locks | sqlx | Phase 3C |
| Tracing spans | sea-orm | Phase 3B |
| Slow query logging | sqlx, sea-orm | Phase 3B |

### Nice-to-have (completeness):
| Gap | Who Has It | Priority |
|-----|-----------|----------|
| BIT/VARBIT | sqlx, tokio-pg | Phase 1C |
| HSTORE | sqlx, tokio-pg | Phase 1C |
| Portal/Cursor | tokio-pg | Phase 2B |
| Lazy connect | sqlx, sea-orm | Phase 2B |
| Direct TLS (PG 17+) | tokio-pg | Phase 3A |
| Benchmarks | diesel | Phase 3C |

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

## Implementation Progress

| Phase | Scope | Status | OIDs | Branch |
|-------|-------|--------|------|--------|
| **1A** | NUMERIC, INET/CIDR, Interval, Range, Geometric, Money, XML, LSN | **DONE** | +36 | `feature/phase1a-types` |
| **1B** | Custom Enums, Composite Types | PLANNED | +dynamic | — |
| **1C** | BIT/VARBIT, HSTORE | PLANNED | +4 | — |
| **2A** | Row Streaming, Simple Query Protocol | PLANNED | — | — |
| **2B** | Portal/Cursor, Pool Callbacks, Lazy Connect | PLANNED | — | — |
| **3A** | Certificate Auth, SCRAM-PLUS, Direct TLS | PLANNED | — | — |
| **3B** | Tracing Spans, Slow Query, Metrics, Pool Metrics | PLANNED | — | — |
| **3C** | Advisory Locks, Criterion Benchmarks | PLANNED | — | — |

Each sub-phase is independently shippable as a minor version bump.

---

## Success Criteria

After full implementation:
- Type coverage: 70+ OIDs (matching or exceeding all competitors)
- Only Rust PG driver with: pipeline + streaming + cursor + advisory locks + correct SCRAM-PLUS
- Benchmark suite proving performance claims
- Enterprise-ready: cert auth, channel binding, direct TLS, full observability
- Feature parity or superiority in every category vs sqlx, tokio-postgres, and diesel

---

## Codebase Metrics (Current)

| Metric | Main Branch | After Phase 1A Merge |
|--------|:-----------:|:--------------------:|
| Total OIDs | 26 | 62 |
| ToSql impls | 17 | 32 |
| FromSql impls | 13 | 28 |
| Type modules | 3 files | 11 files |
| Types LOC | 824 | 1,981 |
| Core tests | 150 | 180+ |
| Test files | 17 | 20+ |
