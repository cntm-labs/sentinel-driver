# Crates Directory Restructure Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Move from flat workspace (root = crate + workspace) to `crates/` directory pattern matching chorus project, so each crate has clear ownership of its CHANGELOG.md and metadata.

**Architecture:** Root Cargo.toml becomes workspace-only (no `[package]`). Driver source moves to `crates/sentinel-driver/`, derive moves to `crates/sentinel-derive/`. All config files (release-please, workflows, CI) update paths accordingly.

**Tech Stack:** Rust workspace, release-please v4, GitHub Actions

**Prerequisites:** This plan runs on the `chore/release-pipeline` branch which already has:
- sentinel-derive rename (commit bbdacdc)
- release-please config (commit 5ed6a62)
- release-please workflow (commit f57cf89)

---

### Task 1: Move derive crate to `crates/sentinel-derive/`

**Files:**
- Move: `derive/` → `crates/sentinel-derive/`

**Step 1: Create directory and move files**

```bash
mkdir -p crates
git mv derive crates/sentinel-derive
```

**Step 2: Verify structure**

```bash
ls crates/sentinel-derive/src/lib.rs
```

Expected: file exists.

**Step 3: Commit**

```bash
git add -A
git commit -m "refactor: move derive crate to crates/sentinel-derive/"
```

---

### Task 2: Move driver crate to `crates/sentinel-driver/`

**Files:**
- Move: `src/` → `crates/sentinel-driver/src/`
- Create: `crates/sentinel-driver/Cargo.toml` (extracted from root)

**Step 1: Create driver crate directory and move source**

```bash
mkdir -p crates/sentinel-driver
git mv src crates/sentinel-driver/src
```

**Step 2: Create `crates/sentinel-driver/Cargo.toml`**

Extract the `[package]`, `[dependencies]`, `[features]`, and `[dev-dependencies]` from the root Cargo.toml into this new file. Update the derive dependency path:

```toml
[package]
name = "sentinel-driver"
version = "0.1.0"
edition = "2021"
description = "High-performance PostgreSQL wire protocol driver for Rust"
license = "MIT OR Apache-2.0"
repository = "https://github.com/cntm-labs/sentinel-driver"
homepage = "https://github.com/cntm-labs/sentinel-driver"
keywords = ["postgresql", "database", "driver", "async", "tokio"]
categories = ["database"]
readme = "../../README.md"
rust-version = "1.75"

[dependencies]
# Async runtime
tokio = { version = "1", features = ["net", "io-util", "time", "sync", "rt", "macros"] }

# Zero-copy buffers
bytes = "1"

# TLS
rustls = "0.23"
webpki-roots = "0.26"
tokio-rustls = "0.26"
rustls-pki-types = "1"

# Crypto (SCRAM-SHA-256)
sha2 = "0.10"
hmac = "0.12"
stringprep = "0.1"
base64 = "0.22"
rand = "0.8"

# Common PG types
chrono = { version = "0.4", default-features = false, features = ["std"] }
uuid = "1"

# Error handling
thiserror = "2"

# Logging
tracing = "0.1"

# LRU cache for prepared statements
lru = "0.12"

# Derive macros
sentinel-derive = { path = "../sentinel-derive", optional = true }

[features]
default = ["derive"]
derive = ["dep:sentinel-derive"]

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
```

Key changes from root:
- `sentinel-derive` path: `"derive"` → `"../sentinel-derive"`
- `readme`: `"README.md"` → `"../../README.md"`

**Step 3: Commit**

```bash
git add -A
git commit -m "refactor: move driver crate to crates/sentinel-driver/"
```

---

### Task 3: Convert root Cargo.toml to workspace-only

**Files:**
- Modify: `Cargo.toml` (remove `[package]`, `[dependencies]`, `[features]`, `[dev-dependencies]`, update `[workspace]`)

**Step 1: Rewrite root `Cargo.toml`**

Replace entire file with workspace-only config (modeled after chorus):

```toml
[workspace]
resolver = "2"
members = [
    "crates/sentinel-driver",
    "crates/sentinel-derive",
]

[workspace.lints.rust]
unsafe_code = "allow"
dead_code = "deny"
unused_imports = "deny"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
must_use_candidate = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"
dbg_macro = "deny"
todo = "warn"
```

Note: `unsafe_code = "allow"` (not "forbid") because sentinel-driver uses minimal unsafe for zero-copy parsing per CLAUDE.md conventions.

**Step 2: Verify compilation**

```bash
cargo check --workspace
```

Expected: compiles successfully.

**Step 3: Run full verification**

```bash
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

**Step 4: Commit**

```bash
git add -A
git commit -m "refactor: convert root to workspace-only Cargo.toml

Workspace now uses resolver 2 with shared lints.
Crates at crates/sentinel-driver and crates/sentinel-derive."
```

---

### Task 4: Update derive crate readme path

**Files:**
- Modify: `crates/sentinel-derive/Cargo.toml`

**Step 1: Fix readme path**

The derive Cargo.toml currently has `readme = "../README.md"` which was relative to the old `derive/` location. Update to:

```toml
readme = "../../README.md"
```

**Step 2: Commit**

```bash
git add crates/sentinel-derive/Cargo.toml
git commit -m "fix: update sentinel-derive readme path for new crate location"
```

---

### Task 5: Update release-please configuration for `crates/` paths

**Files:**
- Modify: `release-please-config.json`
- Modify: `.release-please-manifest.json`

**Step 1: Update `release-please-config.json`**

Change package paths from `"derive"` / `"."` to `"crates/sentinel-derive"` / `"crates/sentinel-driver"`:

```json
{
  "$schema": "https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json",
  "packages": {
    "crates/sentinel-derive": {
      "release-type": "rust",
      "component": "sentinel-derive",
      "bump-minor-pre-major": true,
      "bump-patch-for-minor-pre-major": true
    },
    "crates/sentinel-driver": {
      "release-type": "rust",
      "component": "sentinel-driver",
      "bump-minor-pre-major": true,
      "bump-patch-for-minor-pre-major": true,
      "extra-files": [
        {
          "type": "toml",
          "path": "Cargo.toml",
          "glob": true,
          "jsonpath": "$.dependencies.sentinel-derive.version"
        }
      ]
    }
  },
  "group-pull-request-title-pattern": "chore: release ${version}",
  "linked-versions": [
    {
      "tag": "v",
      "components": ["sentinel-derive", "sentinel-driver"]
    }
  ]
}
```

**Step 2: Update `.release-please-manifest.json`**

```json
{
  "crates/sentinel-derive": "0.1.0",
  "crates/sentinel-driver": "0.1.0"
}
```

**Step 3: Commit**

```bash
git add release-please-config.json .release-please-manifest.json
git commit -m "chore: update release-please paths for crates/ structure"
```

---

### Task 6: Update GitHub Actions workflows

**Files:**
- Modify: `.github/workflows/release-please.yml` (publish paths)
- Modify: `.github/workflows/ci.yml` (no changes needed — `--workspace` flag covers all)

**Step 1: Verify CI workflow**

Read `.github/workflows/ci.yml`. It uses `cargo test --workspace`, `cargo clippy --workspace`, `cargo fmt --all` — all workspace-level commands. **No changes needed.**

**Step 2: Verify release-please workflow**

Read `.github/workflows/release-please.yml`. It uses `cargo publish -p sentinel-derive` and `cargo publish -p sentinel-driver` — these use `-p` (package name) not path, so **no changes needed** for publish commands. The release-please action uses config files (already updated in Task 5).

**Step 3: Commit (only if changes were needed)**

If no changes needed, skip this step.

---

### Task 7: Create manual publish workflow

**Files:**
- Create: `.github/workflows/publish-crates.yml`

**Step 1: Create `.github/workflows/publish-crates.yml`**

```yaml
name: Publish Crates (Manual)

on:
  workflow_dispatch:
    inputs:
      dry-run:
        description: "Dry run (no actual publish)"
        required: false
        default: "false"
        type: choice
        options:
          - "false"
          - "true"

jobs:
  publish:
    runs-on: ubuntu-latest
    environment: crates-io
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Verify tests pass
        run: cargo test --workspace

      - name: Publish sentinel-derive
        run: cargo publish -p sentinel-derive --no-verify ${{ inputs.dry-run == 'true' && '--dry-run' || '' }}
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}

      - name: Wait for crates.io index
        if: inputs.dry-run == 'false'
        run: sleep 30

      - name: Publish sentinel-driver
        run: cargo publish -p sentinel-driver --no-verify ${{ inputs.dry-run == 'true' && '--dry-run' || '' }}
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

**Step 2: Commit**

```bash
git add .github/workflows/publish-crates.yml
git commit -m "ci: add manual publish workflow with dry-run option"
```

---

### Task 8: Update CLAUDE.md project structure

**Files:**
- Modify: `CLAUDE.md`

**Step 1: Update the Project Structure section**

Replace the project structure in CLAUDE.md to reflect the new `crates/` layout:

```markdown
## Project Structure
\```
sentinel-driver/
├── crates/
│   ├── sentinel-driver/        # Main driver crate
│   │   └── src/
│   │       ├── lib.rs          # Public API
│   │       ├── config.rs       # Connection configuration
│   │       ├── error.rs        # Error types
│   │       ├── protocol/       # PG wire protocol
│   │       ├── connection/     # TCP/TLS + handshake
│   │       ├── auth/           # SCRAM-SHA-256, MD5
│   │       ├── pool/           # Connection pool
│   │       ├── pipeline/       # PG pipeline mode
│   │       ├── copy/           # COPY IN/OUT
│   │       ├── notify/         # LISTEN/NOTIFY
│   │       ├── types/          # PG type encode/decode
│   │       ├── tls/            # rustls integration
│   │       ├── row.rs          # Row type
│   │       ├── statement.rs    # Prepared statement
│   │       └── transaction.rs  # Transaction wrapper
│   └── sentinel-derive/        # Derive macros crate
│       └── src/
│           └── lib.rs          # FromRow, ToSql, FromSql
├── docs/plans/
├── .github/workflows/
├── Cargo.toml                  # Workspace root (no package)
└── Cargo.lock
\```
```

**Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md project structure for crates/ layout"
```

---

### Task 9: Verify & dry-run publish

**Step 1: Full verification**

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

**Step 2: Dry-run publish both crates**

```bash
cargo publish -p sentinel-derive --dry-run
cargo publish -p sentinel-driver --dry-run
```

This verifies crates.io metadata is complete and packaging works correctly with the new paths.

**Step 3: Commit any fixes if needed**

---

### Post-Implementation: GitHub Setup (Manual)

After merging, the repo owner must:

1. **Create GitHub Environment** `crates-io` at Settings → Environments
2. **Add secret** `CARGO_REGISTRY_TOKEN` to the `crates-io` environment (generate at https://crates.io/settings/tokens)
3. **First release:** Push a conventional commit (e.g., `feat: initial release`) to main → release-please creates Release PR → merge it → auto-publishes
