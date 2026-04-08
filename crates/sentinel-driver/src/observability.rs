use std::sync::Arc;
use std::time::Duration;

/// Metrics for a completed query execution.
#[derive(Debug, Clone)]
pub struct QueryMetrics {
    /// The SQL statement that was executed.
    pub sql: String,
    /// Time taken to execute the query.
    pub elapsed: Duration,
    /// Number of rows affected (for INSERT/UPDATE/DELETE) or returned.
    pub rows_affected: u64,
    /// Whether the statement was served from the prepared statement cache.
    pub cache_hit: bool,
}

/// Callback invoked after every query completion.
pub type QueryMetricsCallback = Arc<dyn Fn(&QueryMetrics) + Send + Sync>;

/// Configuration for query observability.
#[derive(Clone, Default)]
pub struct ObservabilityConfig {
    /// Queries exceeding this duration are logged at WARN level.
    /// `None` disables slow query logging.
    pub slow_query_threshold: Option<Duration>,
    /// Optional callback invoked after every query with timing metrics.
    pub on_query: Option<QueryMetricsCallback>,
}

impl std::fmt::Debug for ObservabilityConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObservabilityConfig")
            .field("slow_query_threshold", &self.slow_query_threshold)
            .field("on_query", &self.on_query.as_ref().map(|_| ".."))
            .finish()
    }
}

/// Log a slow query warning via tracing.
pub fn log_slow_query(sql: &str, elapsed: Duration, threshold: Duration) {
    if elapsed > threshold {
        let truncated = if sql.len() > 200 { &sql[..200] } else { sql };
        tracing::warn!(
            sql = %truncated,
            elapsed_ms = %elapsed.as_millis(),
            threshold_ms = %threshold.as_millis(),
            "slow query detected"
        );
    }
}

/// Emit a tracing span for a query execution.
pub fn query_span(sql: &str) -> tracing::Span {
    let truncated = if sql.len() > 100 { &sql[..100] } else { sql };
    tracing::info_span!("pg.query", db.statement = %truncated)
}
