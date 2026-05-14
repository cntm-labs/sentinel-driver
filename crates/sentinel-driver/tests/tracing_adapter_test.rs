//! Verify TracingInstrumentation records the expected fields onto the
//! current span. Live-PG; skips silently without DATABASE_URL.

use std::sync::Arc;

use sentinel_driver::{Config, Connection, TracingInstrumentation};
use tracing::Instrument;
use tracing_test::traced_test;

#[tokio::test]
#[traced_test]
async fn execute_records_db_attributes() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else {
        return;
    };
    let cfg = Config::parse(&url)
        .unwrap()
        .with_instrumentation(Arc::new(TracingInstrumentation::default()));
    let mut conn = Connection::connect(cfg).await.unwrap();
    // Suppress NOTICEs that would otherwise surface as Protocol errors.
    conn.execute("SET client_min_messages = ERROR", &[])
        .await
        .unwrap();

    let span = tracing::info_span!(
        "db.query",
        db.system = tracing::field::Empty,
        db.statement = tracing::field::Empty,
        db.operation = tracing::field::Empty,
        db.rows_affected = tracing::field::Empty,
        sntl.param_count = tracing::field::Empty,
        sntl.duration_us = tracing::field::Empty,
    );
    async {
        conn.query("SELECT 1::int4", &[]).await.unwrap();
        // Emit a debug event so the formatter writes a log line that includes
        // all span fields recorded by the adapter (db.system, db.statement,
        // db.operation, …). Without an event, span.record() calls are silent.
        tracing::debug!("query done");
    }
    .instrument(span)
    .await;

    assert!(logs_contain("db.system"), "expected db.system in logs");
    assert!(logs_contain("postgresql"), "expected postgresql in logs");
    assert!(
        logs_contain("SELECT"),
        "expected SELECT (db.operation) in logs"
    );
}

#[tokio::test]
#[traced_test]
async fn slow_query_emits_warn() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else {
        return;
    };
    let cfg = Config::parse(&url)
        .unwrap()
        .with_instrumentation(Arc::new(TracingInstrumentation {
            max_sql_len: 1024,
            slow_threshold: Some(std::time::Duration::from_nanos(1)),
        }));
    let mut conn = Connection::connect(cfg).await.unwrap();
    conn.execute("SET client_min_messages = ERROR", &[])
        .await
        .unwrap();

    let span = tracing::info_span!("db.query");
    async {
        conn.query("SELECT 1::int4", &[]).await.unwrap();
    }
    .instrument(span)
    .await;

    assert!(
        logs_contain("slow query"),
        "expected slow-query warn in logs"
    );
}
