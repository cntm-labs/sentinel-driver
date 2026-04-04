# sentinel-driver — Project Infrastructure Design

> Setup GitHub templates, CI, tooling, tests reorganization, and project docs.

**Date:** 2026-04-04
**Status:** Approved

---

## 1. Rust Tooling Config

### rust-toolchain.toml
- `channel = "1.75"`, `profile = "minimal"`
- MSRV pinned to 1.75 (matches Cargo.toml rust-version)

### Cargo.toml [lints.clippy]
- **forbid** (cannot be `#[allow]`'d): `unwrap_used`, `dbg_macro`, `todo`, `unimplemented`, `print_stdout`, `print_stderr`, `mem_forget`, `exit`
- **deny**: `expect_used`, `large_enum_variant`, `result_large_err`, `needless_pass_by_value`, `redundant_closure_for_method_calls`, `manual_let_else`, `cloned_instead_of_copied`, `implicit_clone`
- **warn**: `pedantic` group (with `module_name_repetitions = "allow"`)

### clippy.toml
- Ban `.min()/.max()` methods (use `std::cmp::min/max`)
- `cognitive-complexity-threshold = 30`
- `too-many-lines-threshold = 150`
- `too-large-for-stack = 256`

### rustfmt.toml
- `edition = "2021"`, `max_width = 100`

### .editorconfig
- UTF-8, LF, 4 spaces (2 for YAML)

---

## 2. Test Reorganization

Move ALL tests from `src/` inline `#[cfg(test)]` to `tests/` directory.

### tests/core/ — No database required
- `config.rs`, `error.rs`
- `protocol_frontend.rs`, `protocol_backend.rs`, `protocol_codec.rs`
- `types_encode.rs`, `types_decode.rs`, `types_builtin.rs`
- `row.rs`, `statement.rs`, `transaction.rs`, `cache.rs`
- `copy_binary.rs`, `copy_text.rs`
- `auth_md5.rs`, `auth_scram.rs`
- `notify_channel.rs`, `pipeline.rs`

### tests/postgres/ — Live PG required
- `postgres.rs`, `types.rs`, `auth.rs`, `pipeline.rs`, `pool.rs`
- `copy.rs`, `notify.rs`, `transaction.rs`, `tls.rs`, `error.rs`, `derives.rs`
- `setup.sql`, `Dockerfile`, `migrations/`

### Supporting files
- `tests/docker-compose.yml` — PG 13, 16, 17 containers
- `tests/certs/` — TLS certificates
- `tests/fixtures/` — Shared SQL

---

## 3. CI Workflows

### ci.yml (push/PR to main)
- **lint**: cargo fmt --check + clippy -D warnings + clippy +beta
- **test-core**: cargo test (core tests, no PG)

### postgresql.yml (push/PR to main, needs ci)
- **pg-integration**: Matrix PG 13,16,17 × TLS on/off

### coverage.yml (push/PR to main, needs ci)
- **coverage**: cargo-llvm-cov → Codecov

### release.yml (tag v*)
- **publish**: cargo publish to crates.io

---

## 4. GitHub Templates

### .github/ISSUE_TEMPLATE/
- `bug_report.yml` — Structured form (description, repro, version, PG version, OS, Rust version)
- `feature_request.yml` — Structured form (description, solution, breaking change?)
- `documentation.yml` — Structured form (which doc, what's wrong)
- `config.yml` — `blank_issues_enabled: false`, redirect to Discussions + Discord

### .github/pull_request_template.md
- Issue reference, breaking change?, test checklist

---

## 5. Project Docs

- `README.md` — Badges, quick start, features, benchmarks, license
- `CHANGELOG.md` — Keep a Changelog format
- `CONTRIBUTING.md` — Setup, PR workflow, test requirements
- `SECURITY.md` — GitHub Security Advisories
- `LICENSE-MIT` + `LICENSE-APACHE` — Dual license

---

## 6. Git Setup

### .gitattributes
- `* text=auto eol=lf`

### .gitignore
- Add: `.env`, `.vscode/`, `.idea/`, `*.vim`

### .githooks/pre-commit
- Shell script: `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo test`
- Setup: `git config core.hooksPath .githooks`

---

## 7. Update CLAUDE.md
- Reflect new folder structure (tests/, .githooks/, etc.)
