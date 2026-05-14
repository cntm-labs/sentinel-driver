//! Instrumentation surface for `sentinel-driver`.
//!
//! Install via `Config::with_instrumentation`, `Pool::with_instrumentation`,
//! or `Connection::set_instrumentation`. Default is a no-op.

use std::sync::Arc;
use std::time::Duration;

use crate::Error;
use crate::transaction::IsolationLevel;

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

#[non_exhaustive]
pub enum Event<'a> {
    // Connection lifecycle
    Connect       { host: &'a str, port: u16 },
    Authenticated { user: &'a str },
    Disconnect    { reason: DisconnectReason },

    // Prepare
    PrepareStart  { name: &'a str, sql: &'a str },
    PrepareFinish {
        name: &'a str,
        param_oids: &'a [u32],
        col_count: u16,
        duration: Duration,
        cache_hit: bool,
    },

    // Execute (covers query / query_one / query_opt / execute / query_typed*)
    ExecuteStart  { stmt: StmtRef<'a>, param_count: usize },
    ExecuteFinish {
        stmt: StmtRef<'a>,
        rows: u64,
        duration: Duration,
        outcome: Outcome<'a>,
    },

    // Pipeline
    PipelineStart { batch_len: usize },
    PipelineFlush { batch_len: usize, total_duration: Duration },

    // Transaction
    TxBegin    { isolation: Option<IsolationLevel> },
    TxCommit   { duration: Duration },
    TxRollback { duration: Duration, reason: RollbackReason<'a> },

    // Pool
    PoolAcquireStart  { pending: usize },
    PoolAcquireFinish { wait: Duration, outcome: AcquireOutcome },
    PoolRelease,

    // PG async messages
    Notice       { severity: &'a str, code: &'a str, message: &'a str },
    Notification { channel: &'a str, payload: &'a str, pid: i32 },

    // Sentinel-level (sntl crate emits these; driver itself never does)
    QueryMacro      { macro_name: &'a str, query_id: &'a str, sql: &'a str },
    ReducerBegin    { name: &'a str },
    ReducerCommit   { name: &'a str, duration: Duration },
    ReducerRollback { name: &'a str, error: &'a str },
    MigrationApply  { version: &'a str, duration: Duration, checksum: &'a str },
    MigrationDrift  { version: &'a str, recorded: &'a str, current: &'a str },
}

#[non_exhaustive]
pub enum StmtRef<'a> {
    Named  { name: &'a str },
    Inline { sql: &'a str },
}

impl<'a> StmtRef<'a> {
    /// SQL text if available (Inline), else the prepared name (Named).
    pub fn sql_or_name(&self) -> &'a str {
        match self {
            StmtRef::Named { name } => name,
            StmtRef::Inline { sql } => sql,
        }
    }

    /// First word of the SQL, uppercased. "OTHER" if no leading keyword found.
    ///
    /// Allocation-free: uses `eq_ignore_ascii_case` rather than uppercasing
    /// the input, so it is safe to call on every `ExecuteStart` event.
    pub fn op_hint(&self) -> &'static str {
        let s = self.sql_or_name();
        let first = s.split_ascii_whitespace().next().unwrap_or("");
        if      first.eq_ignore_ascii_case("SELECT")   { "SELECT"   }
        else if first.eq_ignore_ascii_case("INSERT")   { "INSERT"   }
        else if first.eq_ignore_ascii_case("UPDATE")   { "UPDATE"   }
        else if first.eq_ignore_ascii_case("DELETE")   { "DELETE"   }
        else if first.eq_ignore_ascii_case("BEGIN")    { "BEGIN"    }
        else if first.eq_ignore_ascii_case("COMMIT")   { "COMMIT"   }
        else if first.eq_ignore_ascii_case("ROLLBACK") { "ROLLBACK" }
        else if first.eq_ignore_ascii_case("WITH")     { "WITH"     }
        else                                           { "OTHER"    }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_hint_is_case_insensitive() {
        assert_eq!(StmtRef::Inline { sql: "SELECT 1" }.op_hint(), "SELECT");
        assert_eq!(StmtRef::Inline { sql: "select 1" }.op_hint(), "SELECT");
        assert_eq!(StmtRef::Inline { sql: "Insert into t values (1)" }.op_hint(), "INSERT");
        assert_eq!(StmtRef::Inline { sql: "   WITH cte AS (SELECT 1) SELECT * FROM cte" }.op_hint(), "WITH");
        assert_eq!(StmtRef::Inline { sql: "CREATE TABLE x ()" }.op_hint(), "OTHER");
        assert_eq!(StmtRef::Inline { sql: "" }.op_hint(), "OTHER");
    }
}

#[non_exhaustive]
pub enum Outcome<'a> {
    Ok,
    Err(&'a Error),
}

#[non_exhaustive]
pub enum DisconnectReason {
    Graceful,
    BrokenPipe,
    Timeout,
    ServerKill,
}

#[non_exhaustive]
pub enum RollbackReason<'a> {
    Explicit,
    Drop,
    Error(&'a Error),
}

#[non_exhaustive]
pub enum AcquireOutcome {
    Ok,
    Timeout,
    PoolClosed,
}
