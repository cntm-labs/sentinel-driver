//! Compile-fail tests via `trybuild`.
//!
//! Each `.rs` file under `tests/ui/` is compiled; its paired `.stderr`
//! file captures the expected diagnostic output. Regenerate with
//! `TRYBUILD=overwrite cargo test --test ui_tests`.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
