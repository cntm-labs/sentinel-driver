//! `tracing`-based `Instrumentation` impl.
//!
//! Records OTel-conformant `db.*` fields onto the current `tracing::Span`.
//! Wire sites must have opened a span via `tracing::info_span!("db.query")`
//! and entered it (`Span::in_scope` or `Instrument::instrument`); this
//! adapter only records onto whatever span is current. It never creates
//! new spans itself, so it works correctly across async await points.

use std::time::Duration;

use crate::{Event, Instrumentation, Outcome};

/// Always-built tracing adapter.
#[derive(Clone)]
pub struct TracingInstrumentation {
    /// Truncate `db.statement` past this many bytes. Default 1024.
    pub max_sql_len: usize,
    /// Emit a WARN-level event when a query's duration exceeds this.
    /// `None` disables slow-query warnings.
    pub slow_threshold: Option<Duration>,
}

impl Default for TracingInstrumentation {
    fn default() -> Self {
        Self {
            max_sql_len: 1024,
            slow_threshold: None,
        }
    }
}

impl Instrumentation for TracingInstrumentation {
    fn on_event(&self, ev: &Event<'_>) {
        let span = tracing::Span::current();
        match ev {
            Event::ExecuteStart { stmt, param_count } => {
                let sql = truncate(stmt.sql_or_name(), self.max_sql_len);
                span.record("db.system", "postgresql");
                span.record("db.statement", &tracing::field::display(&sql));
                span.record("db.operation", stmt.op_hint());
                span.record("sntl.param_count", *param_count);
            }
            Event::ExecuteFinish {
                rows,
                duration,
                outcome,
                ..
            } => {
                span.record("db.rows_affected", *rows);
                span.record("sntl.duration_us", duration.as_micros() as i64);
                if let Outcome::Err(e) = outcome {
                    span.record("error", true);
                    tracing::error!(error = %e, "query failed");
                }
                if matches!(self.slow_threshold, Some(t) if *duration > t) {
                    tracing::warn!(slow = true, "slow query");
                }
            }
            Event::PrepareFinish {
                cache_hit,
                duration,
                ..
            } => {
                span.record("sntl.cache_hit", *cache_hit);
                span.record("sntl.prepare_us", duration.as_micros() as i64);
            }
            Event::TxBegin { isolation } => {
                tracing::info!(isolation = ?isolation, "tx begin");
            }
            Event::TxCommit { duration } => {
                tracing::info!(duration_us = duration.as_micros() as i64, "tx commit");
            }
            Event::TxRollback { duration, reason } => {
                tracing::warn!(
                    duration_us = duration.as_micros() as i64,
                    reason = ?reason,
                    "tx rollback"
                );
            }
            Event::PipelineFlush {
                batch_len,
                total_duration,
            } => {
                span.record("sntl.pipeline_batch_len", *batch_len);
                span.record("sntl.duration_us", total_duration.as_micros() as i64);
            }
            Event::PoolAcquireFinish { wait, outcome } => {
                tracing::debug!(
                    wait_us = wait.as_micros() as i64,
                    outcome = ?outcome,
                    "pool acquire"
                );
            }
            Event::Notice {
                severity,
                code,
                message,
            } => {
                tracing::warn!(severity = %severity, code = %code, "{}", message);
            }
            Event::Notification { channel, pid, .. } => {
                tracing::info!(channel = %channel, pid = pid, "notification");
            }
            // Other events (Connect, Disconnect, PoolRelease, PoolAcquireStart,
            // PipelineStart, PrepareStart, ExecuteStart-already-handled,
            // Authenticated, sntl-level events) are deliberately not echoed
            // here — the sntl bridge will handle Sentinel-level ones, and
            // the rest are pure correlation events without OTel mapping.
            _ => {}
        }
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    // Walk back to nearest char boundary to avoid panic on multi-byte
    let mut idx = max;
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    &s[..idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_returns_input() {
        assert_eq!(truncate("hello", 1024), "hello");
    }

    #[test]
    fn truncate_long_cuts_at_boundary() {
        let s = "a".repeat(2000);
        let t = truncate(&s, 1024);
        assert_eq!(t.len(), 1024);
    }

    #[test]
    fn truncate_respects_multibyte_boundary() {
        // 'é' is 2 bytes in UTF-8; truncating mid-char must not panic
        let s = "abcdéé";
        let t = truncate(s, 5);
        // max is 5; index 5 falls inside the 2-byte 'é' starting at index 4.
        // The function should walk back to 4 (char boundary).
        assert_eq!(t, "abcd");
    }

    #[test]
    fn default_max_sql_len_is_1024() {
        assert_eq!(TracingInstrumentation::default().max_sql_len, 1024);
    }

    #[test]
    fn default_slow_threshold_is_none() {
        assert!(TracingInstrumentation::default().slow_threshold.is_none());
    }
}
