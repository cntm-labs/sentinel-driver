# Codebase Refactoring Design

## Problem

The codebase has structural inconsistencies that make it hard to understand:

1. **lib.rs is a God File** — 531 lines, 37 functions mixing re-exports + Connection struct + all methods
2. **27 tests scattered in 7 source files** — `#[cfg(test)]` modules inside `src/` instead of `tests/`
3. **types/mod.rs does too many things** — OID constants + trait definitions + Option impls + helpers in one file
4. **Test naming is inconsistent** — some use prefixes (`types_encode.rs`), some don't (`cache.rs`), `types_misc.rs` is a grab-bag
5. **No subdirectory structure in tests** — flat list of 25+ files makes it hard to find related tests

## Design Decisions

### 1. Split Connection out of lib.rs by responsibility

lib.rs becomes re-exports only (~70 lines). Connection moves to `connection/` with methods split by responsibility:

```
lib.rs              — pub mod + pub use only
connection/
  mod.rs            — Connection struct + pub use submodules
  client.rs         — connect, close, is_broken, cancel_token, accessors
  query.rs          — query, query_one, query_opt, execute, with_timeout variants
  transaction.rs    — begin, commit, rollback, savepoint, rollback_to
  copy.rs           — copy_in, copy_out
  notify.rs         — listen, unlisten, unlisten_all, notify, wait_for_notification
  pipeline.rs       — pipeline, execute_pipeline
  prepare.rs        — prepare, register_statement, cache_metrics
  internal.rs       — query_internal, drain_until_ready (pub(crate))
```

**Pattern:** Facade (Connection) delegates to focused impl blocks in separate files.

### 2. Move all local tests to tests/core/ with subdirectories

Remove all `#[cfg(test)]` modules from source files. Tests go to `tests/core/` with subdirectories matching source structure:

```
tests/core/
  auth/scram.rs, md5.rs
  pool/health.rs, config.rs, pool.rs
  pipeline/batch.rs
  notify/notify.rs
  types/encode.rs, decode.rs, builtin.rs, interval.rs, network.rs, ...
  protocol/backend.rs, codec.rs, frontend.rs
  cache.rs, config.rs, row.rs, statement.rs, transaction.rs, ...
```

**Rule:** Every source file gets exactly one test file. No `_misc.rs` grab-bags.

### 3. Split types/mod.rs into 3 focused files

```
types/
  mod.rs      — module declarations + re-exports only
  oid.rs      — Oid struct, all OID constants, From impls
  traits.rs   — ToSql, FromSql trait definitions, Option<T> impls, encode_param helpers
```

### 4. Split types_misc.rs into types/xml.rs + types/lsn.rs

One test file per source file. No exceptions.

### What stays unchanged (YAGNI)

- `encode.rs` / `decode.rs` — primitive impls are small enough to stay together
- `sentinel-derive/src/lib.rs` — single file proc-macro, refactor when expanding features
- All type module files (interval.rs, network.rs, etc.) — already well-structured

## Code Smells Fixed (refactoring.guru)

| Smell | Where | Fix |
|-------|-------|-----|
| Large Class | lib.rs Connection | Extract Class → split by responsibility |
| Long Method | lib.rs (37 methods) | Move Method → separate files per concern |
| Shotgun Surgery | tests scattered in source | Consolidate → tests/core/ with structure |
| Divergent Change | types/mod.rs (OIDs + traits) | Extract Class → oid.rs + traits.rs |
| Data Clumps | types_misc.rs (unrelated types) | Split into per-type test files |

## Refactoring Techniques Used (refactoring.guru)

- **Extract Class** — Connection methods → focused files
- **Move Method** — impl blocks to new files via `impl Connection` in separate modules
- **Extract Interface** — types/traits.rs as standalone trait definitions
- **Encapsulate Field** — OID constants into their own module

## Risk

- **Low risk**: All changes are structural (move code between files). No logic changes.
- **Verification**: `cargo test --workspace` must pass after each step.
- **Reversible**: Git history preserves everything.
