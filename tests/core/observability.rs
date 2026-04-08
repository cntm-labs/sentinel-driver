use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use sentinel_driver::observability::{ObservabilityConfig, QueryMetrics};

#[test]
fn test_observability_config_default() {
    let config = ObservabilityConfig::default();
    assert!(config.slow_query_threshold.is_none());
    assert!(config.on_query.is_none());
}

#[test]
fn test_observability_config_debug() {
    let config = ObservabilityConfig::default();
    let debug = format!("{config:?}");
    assert!(debug.contains("slow_query_threshold"));
    assert!(debug.contains("on_query"));
}

#[test]
fn test_query_metrics_callback() {
    let counter = Arc::new(AtomicU64::new(0));
    let counter_clone = Arc::clone(&counter);

    let config = ObservabilityConfig {
        slow_query_threshold: Some(Duration::from_millis(100)),
        on_query: Some(Arc::new(move |_metrics: &QueryMetrics| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        })),
    };

    // Simulate calling the callback
    let metrics = QueryMetrics {
        sql: "SELECT 1".to_string(),
        elapsed: Duration::from_millis(50),
        rows_affected: 1,
        cache_hit: true,
    };

    if let Some(ref cb) = config.on_query {
        cb(&metrics);
    }

    assert_eq!(counter.load(Ordering::Relaxed), 1);
}

#[test]
fn test_query_metrics_struct() {
    let m = QueryMetrics {
        sql: "INSERT INTO t VALUES ($1)".to_string(),
        elapsed: Duration::from_millis(15),
        rows_affected: 1,
        cache_hit: false,
    };
    assert_eq!(m.sql, "INSERT INTO t VALUES ($1)");
    assert_eq!(m.elapsed.as_millis(), 15);
    assert_eq!(m.rows_affected, 1);
    assert!(!m.cache_hit);
}

#[test]
fn test_query_metrics_clone() {
    let m = QueryMetrics {
        sql: "SELECT 1".to_string(),
        elapsed: Duration::from_millis(1),
        rows_affected: 0,
        cache_hit: true,
    };
    let m2 = m.clone();
    assert_eq!(m2.sql, "SELECT 1");
}

#[test]
fn test_log_slow_query_below_threshold() {
    // Does not panic — below threshold
    sentinel_driver::observability::log_slow_query(
        "SELECT 1",
        Duration::from_millis(10),
        Duration::from_millis(100),
    );
}

#[test]
fn test_query_span_creation() {
    let _span = sentinel_driver::observability::query_span("SELECT * FROM users");
}
