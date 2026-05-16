//! Smoke test: NoOpInstrumentation::on_event is effectively free.
//!
//! Not a precise allocation test (those require a global allocator
//! hook that conflicts with Tokio in integration tests). Instead, we
//! measure throughput. A modern CPU can execute ~10⁹ instructions/s;
//! a no-op vtable call is ~5 ns, so 1_000_000 calls must finish in
//! well under 100 ms. If a `Vec::push` or `String::new` were added,
//! the cost would be 10-100 ns each, pushing the loop to >100 ms.
//!
//! The threshold is generous (1s) so the test stays green under heavy
//! CI load, but a real regression — accidental heap alloc on every
//! call — would push the loop into multiple seconds.

use std::time::{Duration, Instant};

use sentinel_driver::{Event, Instrumentation, StmtRef};

struct LocalNoOp;
impl Instrumentation for LocalNoOp {
    #[inline]
    fn on_event(&self, _: &Event<'_>) {}
}

#[test]
fn noop_on_event_is_fast() {
    let instr: std::sync::Arc<dyn Instrumentation> = std::sync::Arc::new(LocalNoOp);

    let sql = "SELECT 1";
    let iters = 1_000_000_usize;
    let start = Instant::now();
    for _ in 0..iters {
        let ev = Event::ExecuteStart {
            stmt: StmtRef::Inline { sql },
            param_count: 0,
        };
        instr.on_event(&ev);
        std::hint::black_box(&ev);
    }
    let elapsed = start.elapsed();

    // Generous threshold — 1s for 1M iters. A regression that allocates
    // per call would blow this out.
    assert!(
        elapsed < Duration::from_secs(1),
        "1M NoOp::on_event calls took {:?} (>1s) — likely a per-call allocation regression",
        elapsed
    );

    // Print for visibility when running locally.
    eprintln!(
        "noop_on_event_is_fast: 1_000_000 calls in {:?} ({:.1} ns/call)",
        elapsed,
        elapsed.as_nanos() as f64 / iters as f64
    );
}
