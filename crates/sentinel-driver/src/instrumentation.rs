//! Instrumentation surface for `sentinel-driver`.
//!
//! Install via `Config::with_instrumentation`, `Pool::with_instrumentation`,
//! or `Connection::set_instrumentation`. Default is a no-op.

use std::sync::Arc;

/// A driver consumer's hook into every operation Sentinel performs.
///
/// Events are passed by borrow — the implementation MUST NOT retain
/// the `Event` past the call. Clone data inside the handler if needed.
pub trait Instrumentation: Send + Sync + 'static {
    fn on_event(&self, event: &Event<'_>);
}

/// Default no-op. Returns immediately via vtable dispatch.
pub(crate) struct NoOpInstrumentation;

impl Instrumentation for NoOpInstrumentation {
    #[inline]
    fn on_event(&self, _: &Event<'_>) {}
}

pub(crate) fn noop() -> Arc<dyn Instrumentation> {
    Arc::new(NoOpInstrumentation)
}

// Event taxonomy lands in Task 2.
pub enum Event<'a> {
    _Phantom(std::marker::PhantomData<&'a ()>),
}
