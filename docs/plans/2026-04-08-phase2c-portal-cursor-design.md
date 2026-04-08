# Phase 2C: Portal/Cursor Support

**Date:** 2026-04-08
**Status:** Approved

## Goal

Add server-side cursor support via named portals for incremental row fetching within transactions.

## Design

### API

```rust
let txn = conn.begin().await?;

let portal = txn.bind("SELECT * FROM big_table", &[]).await?;

let batch1 = txn.query_portal(&portal, 100).await?; // first 100 rows
let batch2 = txn.query_portal(&portal, 100).await?; // next 100
// empty Vec<Row> = cursor exhausted

drop(portal); // sends Close(Portal) + Sync
txn.commit().await?;
```

### Struct

```rust
pub struct Portal<'a> {
    conn: &'a mut PgConnection,
    name: String,
    description: Arc<RowDescription>,
    exhausted: bool,
}
```

Portal names are auto-incremented: `"p0"`, `"p1"`, etc.

### Protocol Flow

1. **`bind(sql, params)`** — sends Parse(unnamed) + Bind(named portal) + Describe(Portal) + Sync
   - Reads: ParseComplete + BindComplete + RowDescription + ReadyForQuery
   - Returns `Portal` holding `&mut conn` and the description

2. **`query_portal(portal, max_rows)`** — sends Execute(portal, max_rows) + Sync
   - Reads DataRows until:
     - `PortalSuspended` → more rows available, return `Vec<Row>`
     - `CommandComplete` → cursor exhausted, set `exhausted = true`, return `Vec<Row>`
   - Then reads ReadyForQuery

3. **Drop** — if not exhausted, sends Close(Portal) + Sync, reads CloseComplete + ReadyForQuery

### Constraints

- Portals only work inside transactions (PG requirement)
- `bind()` lives on `Transaction`, not `Connection`
- Portal borrows `&mut conn` — no concurrent queries while portal is open
- `max_rows = 0` means fetch all remaining (same as regular query)

### Protocol Messages Needed

Already implemented in `protocol/frontend.rs`:
- Parse, Bind, Describe, Execute, Sync, Close

May need to add:
- Bind with named portal (currently uses unnamed `""`)
- Execute with `max_rows` parameter (currently hardcoded to 0)
- Close with Portal type tag (currently only Statement)
- PortalSuspended backend message parsing

### Files

- Create: `crates/sentinel-driver/src/portal.rs`
- Modify: `crates/sentinel-driver/src/protocol/frontend.rs` (named portal in Bind, max_rows in Execute, Close Portal)
- Modify: `crates/sentinel-driver/src/protocol/backend.rs` (PortalSuspended message)
- Modify: `crates/sentinel-driver/src/transaction.rs` (add `bind()` and `query_portal()`)
- Modify: `crates/sentinel-driver/src/lib.rs` (re-export Portal)
- Test: `tests/core/portal.rs`
