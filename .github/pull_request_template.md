## Change Kind (pick one)

- [ ] ➕ Additive — new pub API, default-off feature, new impl, doc change
- [ ] ⚠️ Breaking — requires `v2` branch per `GOVERNANCE.md`
- [ ] 🧹 Internal — no `pub` surface change, tests, refactor, CI

## Self-Verification

- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo test --workspace` passes (integration tests optional without `DATABASE_URL`)
- [ ] If pub API changed: `cargo semver-checks` result reviewed

## Summary

<!-- Brief description of the changes -->

## Changes

-

## Test Plan

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] New tests added for new functionality

## Checklist

- [ ] Code follows project conventions (see CLAUDE.md)
- [ ] No `unwrap()` in production code
- [ ] Public APIs are documented
