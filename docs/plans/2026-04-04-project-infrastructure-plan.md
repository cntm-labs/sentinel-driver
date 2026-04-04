# Project Infrastructure Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Set up complete project infrastructure — tooling config, test reorganization, CI workflows, GitHub templates, documentation, pre-commit hooks.

**Architecture:** Config-first approach. Set up lint/format config, then migrate tests (biggest change), then CI, then docs/templates. Each task is independently verifiable.

**Tech Stack:** Rust 1.75 MSRV, cargo-llvm-cov, GitHub Actions, PostgreSQL 13/16/17

---

### Task 1: Rust Tooling Config Files

**Files:**
- Create: `rust-toolchain.toml`
- Create: `clippy.toml`
- Create: `rustfmt.toml`
- Create: `.editorconfig`
- Modify: `Cargo.toml` (add `[lints]` section)
- Modify: `.gitattributes` (create or replace)
- Modify: `.gitignore` (expand)

**Step 1: Create `rust-toolchain.toml`**

```toml
[toolchain]
channel = "1.75"
profile = "minimal"
```

**Step 2: Create `clippy.toml`**

```toml
[[disallowed-methods]]
path = "core::cmp::Ord::min"
reason = "ambiguous — use std::cmp::min(a, b) instead"

[[disallowed-methods]]
path = "core::cmp::Ord::max"
reason = "ambiguous — use std::cmp::max(a, b) instead"

cognitive-complexity-threshold = 30
too-many-lines-threshold = 150
too-large-for-stack = 256
```

**Step 3: Create `rustfmt.toml`**

```toml
edition = "2021"
max_width = 100
```

**Step 4: Create `.editorconfig`**

```ini
root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
indent_style = space
indent_size = 4

[*.yml]
indent_size = 2

[*.md]
trim_trailing_whitespace = false
```

**Step 5: Create `.gitattributes`**

```
* text=auto eol=lf
```

**Step 6: Expand `.gitignore`**

Add to existing:
```
.env
.vscode/
.idea/
*.vim
*.vi
```

**Step 7: Add `[lints]` section to `Cargo.toml`**

Add after `[workspace.dependencies]`:

```toml
[lints.clippy]
# FORBID — cannot be #[allow]'d
unwrap_used = "forbid"
dbg_macro = "forbid"
todo = "forbid"
unimplemented = "forbid"
print_stdout = "forbid"
print_stderr = "forbid"
mem_forget = "forbid"
exit = "forbid"
# DENY — strict but #[expect()] allowed with justification
expect_used = "deny"
large_enum_variant = "deny"
result_large_err = "deny"
needless_pass_by_value = "deny"
redundant_closure_for_method_calls = "deny"
manual_let_else = "deny"
cloned_instead_of_copied = "deny"
implicit_clone = "deny"
# WARN — pedantic
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"

[lints.rust]
unsafe_code = "forbid"
```

**Step 8: Fix all clippy/fmt violations from new lint rules**

This is the hardest sub-step. The new `forbid` rules (especially `unwrap_used`) will break compilation. Every `.unwrap()` in non-test code must be replaced with `?`, `.expect("reason")`, or proper error handling.

Key files with `.unwrap()`:
- `src/protocol/backend.rs` — `read_i32/read_i16/read_u32` use `.unwrap()` on slice conversion
- `src/auth/scram.rs` — `HmacSha256::new_from_slice().expect()`
- `src/types/encode.rs` — `NaiveDate::from_ymd_opt().unwrap()`
- `src/copy/binary.rs` — `read_i32/read_i16` use `.unwrap()`
- `src/tls/mod.rs` — `ServerName::try_from().unwrap_or_else()`
- `src/connection/stream.rs` — `tcp.set_nodelay(true).ok()`

Strategy: `unwrap()` → `expect("reason")` where infallible, `?` where fallible. In tests, `unwrap()` is fine because test code is not under `[lints]` section (only `[lints.clippy]` applies to lib code, not test binary).

IMPORTANT: `[lints]` in workspace root `Cargo.toml` only applies to the `sentinel-driver` package. Tests in `tests/` are separate integration test targets and inherit lints too. If tests use `unwrap()`, we need `[workspace.lints]` + per-package override OR restructure.

Best approach: Use `[workspace.lints.clippy]` and in the main `Cargo.toml` add `[lints] workspace = true`. Tests can use `#[expect(clippy::unwrap_used)]` at crate level for integration test files.

**Step 9: Run verification**

```bash
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

**Step 10: Commit**

```bash
git add rust-toolchain.toml clippy.toml rustfmt.toml .editorconfig .gitattributes .gitignore Cargo.toml src/
git commit -m "chore: add Rust tooling config (clippy forbid, rustfmt, MSRV 1.75)"
```

---

### Task 2: Migrate Tests — Core (config, error, protocol)

**Files:**
- Create: `tests/core/config.rs`
- Create: `tests/core/error.rs`
- Create: `tests/core/protocol_frontend.rs`
- Create: `tests/core/protocol_backend.rs`
- Create: `tests/core/protocol_codec.rs`
- Modify: `src/config.rs` (remove `#[cfg(test)]` block)
- Modify: `src/error.rs` (remove `#[cfg(test)]` block if any)
- Modify: `src/protocol/frontend.rs` (remove `#[cfg(test)]` block)
- Modify: `src/protocol/backend.rs` (remove `#[cfg(test)]` block)
- Modify: `src/protocol/codec.rs` (remove `#[cfg(test)]` block)

**Step 1: Create each test file**

Move test functions from `src/` to `tests/core/`. Change `use super::*` to `use sentinel_driver::...` imports. Items that are `pub(crate)` need to be made `pub` or tested through public API.

Key consideration: Many tests use `pub(crate)` internals (e.g., `protocol::backend::decode`, `protocol::frontend::*`). These modules are `pub mod` in lib.rs, and the functions/types inside are `pub` — so integration tests CAN access them as `sentinel_driver::protocol::backend::decode(...)`.

Check which items are truly `pub(crate)` vs `pub` and adjust visibility where needed for testability.

**Step 2: Remove all `#[cfg(test)] mod tests` blocks from the 5 source files**

**Step 3: Run tests to verify**

```bash
cargo test --workspace
```

**Step 4: Commit**

```bash
git commit -m "test: migrate config, error, protocol tests to tests/core/"
```

---

### Task 3: Migrate Tests — Types (encode, decode, builtin)

**Files:**
- Create: `tests/core/types_encode.rs`
- Create: `tests/core/types_decode.rs`
- Create: `tests/core/types_builtin.rs`
- Modify: `src/types/encode.rs` (remove tests)
- Modify: `src/types/decode.rs` (remove tests)
- Modify: `src/types/builtin.rs` (remove tests)

Same pattern as Task 2. Run `cargo test --workspace` after.

**Commit:** `test: migrate type system tests to tests/core/`

---

### Task 4: Migrate Tests — Auth, Cache, Row, Statement, Transaction

**Files:**
- Create: `tests/core/auth_md5.rs`
- Create: `tests/core/auth_scram.rs`
- Create: `tests/core/cache.rs`
- Create: `tests/core/row.rs`
- Create: `tests/core/statement.rs`
- Create: `tests/core/transaction.rs`
- Modify: corresponding `src/` files (remove tests)

Same pattern. Run `cargo test --workspace` after.

**Commit:** `test: migrate auth, cache, row, statement, transaction tests to tests/core/`

---

### Task 5: Migrate Tests — Copy, Notify, Pipeline, Pool

**Files:**
- Create: `tests/core/copy_binary.rs`
- Create: `tests/core/copy_text.rs`
- Create: `tests/core/notify_channel.rs`
- Create: `tests/core/notify.rs`
- Create: `tests/core/pipeline.rs`
- Create: `tests/core/pool.rs`
- Modify: corresponding `src/` files (remove tests)

Same pattern. Run `cargo test --workspace` after.

**Commit:** `test: migrate copy, notify, pipeline, pool tests to tests/core/`

---

### Task 6: PostgreSQL Integration Test Skeleton

**Files:**
- Create: `tests/postgres/postgres.rs` (basic connect test, gated on `DATABASE_URL` env)
- Create: `tests/postgres/setup.sql`
- Create: `tests/docker-compose.yml`
- Create: `tests/fixtures/.gitkeep`
- Create: `tests/certs/.gitkeep`

Integration tests use `#[ignore]` by default — only run when `DATABASE_URL` is set. CI passes `DATABASE_URL` explicitly.

```rust
// tests/postgres/postgres.rs
fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}

macro_rules! require_pg {
    () => {
        let Some(url) = database_url() else { return; };
        url
    };
}

#[tokio::test]
async fn test_connect() {
    let url = require_pg!();
    // TODO: implement when Connection is wired to live PG
}
```

**Commit:** `test: add PostgreSQL integration test skeleton and docker-compose`

---

### Task 7: CI Workflows

**Files:**
- Replace: `.github/workflows/ci.yml`
- Create: `.github/workflows/postgresql.yml`
- Create: `.github/workflows/coverage.yml`
- Create: `.github/workflows/release.yml`

**ci.yml:**
- lint job: fmt + clippy + clippy +beta (non-blocking)
- test-core job: `cargo test --workspace` (no PG needed)

**postgresql.yml:**
- needs: ci
- matrix: PG 13, 16, 17
- docker-compose for PG service
- `cargo test --workspace` with `DATABASE_URL`

**coverage.yml:**
- needs: ci
- cargo-llvm-cov → Codecov

**release.yml:**
- trigger: tag `v*`
- cargo publish sentinel-driver-derive then sentinel-driver

**Commit:** `ci: restructure workflows (lint, test, pg-matrix, coverage, release)`

---

### Task 8: GitHub Issue/PR Templates

**Files:**
- Create: `.github/ISSUE_TEMPLATE/bug_report.yml`
- Create: `.github/ISSUE_TEMPLATE/feature_request.yml`
- Create: `.github/ISSUE_TEMPLATE/documentation.yml`
- Create: `.github/ISSUE_TEMPLATE/config.yml`
- Create: `.github/pull_request_template.md`

**Commit:** `chore: add GitHub issue and PR templates`

---

### Task 9: Project Documentation

**Files:**
- Create: `README.md`
- Create: `CHANGELOG.md`
- Create: `CONTRIBUTING.md`
- Create: `SECURITY.md`
- Create: `LICENSE-MIT`
- Create: `LICENSE-APACHE`

**Commit:** `docs: add README, CHANGELOG, CONTRIBUTING, SECURITY, LICENSE files`

---

### Task 10: Pre-commit Hook + Update CLAUDE.md

**Files:**
- Create: `.githooks/pre-commit`
- Modify: `CLAUDE.md` (update project structure)

`.githooks/pre-commit`:
```bash
#!/usr/bin/env bash
set -e
echo "Running pre-commit checks..."
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace --quiet
echo "All checks passed!"
```

Setup: `git config core.hooksPath .githooks`

**Commit:** `chore: add pre-commit hook and update CLAUDE.md`

---

### Verification Checklist

After all tasks complete:
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo test --workspace` — all 125+ tests pass
- [ ] No `#[cfg(test)]` blocks remain in `src/`
- [ ] `.githooks/pre-commit` runs successfully
- [ ] All GitHub templates render correctly
