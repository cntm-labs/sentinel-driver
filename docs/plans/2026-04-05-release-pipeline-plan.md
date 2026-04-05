# Automated crates.io Release Pipeline Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Set up release-please + crates.io automated publishing pipeline, matching the chorus project pattern.

**Architecture:** release-please creates Release PRs on conventional commits → merge triggers CI gate → publishes sentinel-derive then sentinel-driver to crates.io. Manual publish workflow as fallback.

**Tech Stack:** release-please v4, GitHub Actions, crates.io, GitHub Environments

---

### Task 1: Rename sentinel-driver-derive → sentinel-derive

**Files:**
- Modify: `derive/Cargo.toml` (package name + add publish metadata)
- Modify: `Cargo.toml` (workspace dependency reference + add publish metadata to root package)
- Modify: `src/lib.rs` (extern crate reference)
- Modify: `derive/src/lib.rs` (doc comments referencing old name)

**Step 1: Update `derive/Cargo.toml`**

Change package name and add crates.io metadata:

```toml
[package]
name = "sentinel-derive"
version = "0.1.0"
edition = "2021"
description = "Derive macros (FromRow, ToSql, FromSql) for sentinel-driver"
license = "MIT OR Apache-2.0"
repository = "https://github.com/cntm-labs/sentinel-driver"
homepage = "https://github.com/cntm-labs/sentinel-driver"
keywords = ["postgresql", "derive", "database", "orm"]
categories = ["database"]
readme = "../README.md"
rust-version = "1.75"

[lib]
proc-macro = true

[dependencies]
syn = { version = "2", features = ["full"] }
quote = "1"
proc-macro2 = "1"
```

**Step 2: Update root `Cargo.toml`**

Change the dependency name and add publish metadata:

```toml
# In [package] section, add:
repository = "https://github.com/cntm-labs/sentinel-driver"
homepage = "https://github.com/cntm-labs/sentinel-driver"
keywords = ["postgresql", "database", "driver", "async", "tokio"]
categories = ["database"]
readme = "README.md"

# Change dependency line:
sentinel-derive = { path = "derive", optional = true }

# Change feature:
derive = ["dep:sentinel-derive"]
```

**Step 3: Update `src/lib.rs`**

Change the extern crate use:

```rust
// Change:
pub use sentinel_driver_derive::{FromRow, FromSql, ToSql};
// To:
pub use sentinel_derive::{FromRow, FromSql, ToSql};
```

**Step 4: Update `derive/src/lib.rs` doc comments**

Replace all `sentinel_driver_derive` references in doc comments with `sentinel_derive`:

```rust
/// use sentinel_derive::FromRow;
/// use sentinel_derive::ToSql;
/// use sentinel_derive::FromSql;
```

**Step 5: Verify**

```bash
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

**Step 6: Commit**

```bash
git add Cargo.toml derive/ src/lib.rs
git commit -m "chore: rename sentinel-driver-derive to sentinel-derive

Add crates.io publish metadata (repository, homepage, keywords,
categories, readme) to both crates."
```

---

### Task 2: Create release-please Configuration

**Files:**
- Create: `release-please-config.json`
- Create: `.release-please-manifest.json`

**Step 1: Create `release-please-config.json`**

```json
{
  "$schema": "https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json",
  "packages": {
    "derive": {
      "release-type": "rust",
      "component": "sentinel-derive",
      "bump-minor-pre-major": true,
      "bump-patch-for-minor-pre-major": true
    },
    ".": {
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

**Step 2: Create `.release-please-manifest.json`**

```json
{
  "derive": "0.1.0",
  ".": "0.1.0"
}
```

**Step 3: Commit**

```bash
git add release-please-config.json .release-please-manifest.json
git commit -m "chore: add release-please configuration

Linked versions for sentinel-derive and sentinel-driver.
Pre-major bump policy for 0.x development."
```

---

### Task 3: Create release-please Workflow

**Files:**
- Create: `.github/workflows/release-please.yml`
- Delete: `.github/workflows/release.yml` (replaced)

**Step 1: Create `.github/workflows/release-please.yml`**

```yaml
name: Release Please

on:
  push:
    branches: [main]

permissions:
  contents: write
  pull-requests: write

jobs:
  release-please:
    runs-on: ubuntu-latest
    outputs:
      releases_created: ${{ steps.release.outputs.releases_created }}
      sentinel-derive--release_created: ${{ steps.release.outputs['sentinel-derive--release_created'] }}
      sentinel-driver--release_created: ${{ steps.release.outputs['sentinel-driver--release_created'] }}
      tag_name: ${{ steps.release.outputs['sentinel-driver--tag_name'] }}
    steps:
      - uses: googleapis/release-please-action@v4
        id: release
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
          config-file: release-please-config.json
          manifest-file: .release-please-manifest.json

  ci-gate:
    needs: release-please
    if: needs.release-please.outputs.releases_created == 'true'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo test --workspace

  publish-crates:
    needs: [release-please, ci-gate]
    if: needs.release-please.outputs.releases_created == 'true'
    runs-on: ubuntu-latest
    environment: crates-io
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Publish sentinel-derive
        if: needs.release-please.outputs['sentinel-derive--release_created'] == 'true'
        run: cargo publish -p sentinel-derive --no-verify
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}

      - name: Wait for crates.io index
        if: needs.release-please.outputs['sentinel-derive--release_created'] == 'true'
        run: sleep 30

      - name: Publish sentinel-driver
        if: needs.release-please.outputs['sentinel-driver--release_created'] == 'true'
        run: cargo publish -p sentinel-driver --no-verify
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

**Step 2: Delete old `release.yml`**

```bash
rm .github/workflows/release.yml
```

**Step 3: Commit**

```bash
git add .github/workflows/release-please.yml
git rm .github/workflows/release.yml
git commit -m "ci: replace manual release with release-please automation

Automated pipeline: conventional commits → Release PR → CI gate →
crates.io publish (sentinel-derive first, then sentinel-driver)."
```

---

### Task 4: Create Manual Publish Workflow

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

### Task 5: Verify & Dry-run Publish

**Step 1: Verify all builds pass**

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

This verifies crates.io metadata is complete and packaging works.

**Step 3: Commit any fixes and push**

---

### Post-Implementation: GitHub Setup (Manual)

After merging, the repo owner must:

1. **Create GitHub Environment** `crates-io` at Settings → Environments
2. **Add secret** `CARGO_REGISTRY_TOKEN` to the `crates-io` environment (generate at https://crates.io/settings/tokens)
3. **First release:** Push a conventional commit (e.g., `feat: initial release`) to main → release-please creates Release PR → merge it → auto-publishes
