# Contributing to sentinel-driver

## Getting Started

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Make your changes
4. Run checks:
   ```sh
   cargo fmt --all -- --check
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   ```
5. Commit and open a pull request

## Conventions

See [CLAUDE.md](CLAUDE.md) for project conventions, lint policy, and architecture.

Key rules:
- No `unwrap()` in production code (use `?`, `expect()` with `#[allow]`, or proper error handling)
- No `unsafe` code
- Binary format for all PG types by default
- All public APIs must be documented

## Pre-commit Hook

Enable the pre-commit hook to run checks automatically:

```sh
git config core.hooksPath .githooks
```

## Running Integration Tests

Integration tests require a running PostgreSQL instance:

```sh
docker compose -f tests/docker-compose.yml up -d postgres16
export DATABASE_URL=postgres://sentinel:sentinel_test@localhost:5416/sentinel_test
cargo test --workspace
```
