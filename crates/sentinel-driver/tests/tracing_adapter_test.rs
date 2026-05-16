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

// Synthetic coverage tests — fire every Event variant directly through
// the adapter without needing a live PG. Ensures every match arm
// executes for codecov.

#[test]
fn synthetic_all_event_arms_execute() {
    use sentinel_driver::Instrumentation;
    let adapter = TracingInstrumentation::default();

    // Pre-open a span so adapter has somewhere to record.
    let span = tracing::info_span!(
        "synth",
        db.system = tracing::field::Empty,
        db.statement = tracing::field::Empty,
        db.operation = tracing::field::Empty,
        db.rows_affected = tracing::field::Empty,
        sntl.param_count = tracing::field::Empty,
        sntl.duration_us = tracing::field::Empty,
        sntl.cache_hit = tracing::field::Empty,
        sntl.prepare_us = tracing::field::Empty,
        sntl.pipeline_batch_len = tracing::field::Empty,
        error = tracing::field::Empty,
    );
    let _guard = span.enter();

    let sql = "SELECT 1";

    use sentinel_driver::{
        AcquireOutcome, DisconnectReason, Event, Outcome, RollbackReason, StmtRef,
    };
    use std::time::Duration;

    // Each arm of TracingInstrumentation::on_event
    adapter.on_event(&Event::ExecuteStart {
        stmt: StmtRef::Inline { sql },
        param_count: 0,
    });
    adapter.on_event(&Event::ExecuteFinish {
        stmt: StmtRef::Inline { sql },
        rows: 1,
        duration: Duration::from_micros(100),
        outcome: Outcome::Ok,
    });
    adapter.on_event(&Event::PrepareFinish {
        name: "stmt1",
        param_oids: &[],
        col_count: 1,
        duration: Duration::from_micros(50),
        cache_hit: false,
    });
    adapter.on_event(&Event::TxBegin { isolation: None });
    adapter.on_event(&Event::TxCommit {
        duration: Duration::from_micros(10),
    });
    adapter.on_event(&Event::TxRollback {
        duration: Duration::from_micros(10),
        reason: RollbackReason::Explicit,
    });
    adapter.on_event(&Event::PipelineFlush {
        batch_len: 5,
        total_duration: Duration::from_millis(2),
    });
    adapter.on_event(&Event::PoolAcquireFinish {
        wait: Duration::from_micros(20),
        outcome: AcquireOutcome::Ok,
    });
    adapter.on_event(&Event::Notice {
        severity: "NOTICE",
        code: "00000",
        message: "test notice",
    });
    adapter.on_event(&Event::Notification {
        channel: "ch",
        payload: "p",
        pid: 1234,
    });

    // Pure correlation events that go through the `_ => {}` arm.
    adapter.on_event(&Event::PoolRelease);
    adapter.on_event(&Event::Connect {
        host: "localhost",
        port: 5432,
    });
    adapter.on_event(&Event::Authenticated { user: "sentinel" });
    adapter.on_event(&Event::Disconnect {
        reason: DisconnectReason::Graceful,
    });
    adapter.on_event(&Event::PrepareStart { name: "stmt1", sql });
    adapter.on_event(&Event::PipelineStart { batch_len: 5 });
    adapter.on_event(&Event::PoolAcquireStart { pending: 0 });
    adapter.on_event(&Event::QueryMacro {
        macro_name: "query",
        query_id: "abc",
        sql,
    });
    adapter.on_event(&Event::ReducerBegin { name: "r1" });
    adapter.on_event(&Event::ReducerCommit {
        name: "r1",
        duration: Duration::from_micros(15),
    });
    adapter.on_event(&Event::ReducerRollback {
        name: "r1",
        error: "rolled back",
    });
    adapter.on_event(&Event::MigrationApply {
        version: "20260514_120000_init",
        duration: Duration::from_millis(5),
        checksum: "deadbeef",
    });
    adapter.on_event(&Event::MigrationDrift {
        version: "20260514_120000_init",
        recorded: "aaa",
        current: "bbb",
    });

    // Also fire the error-outcome path of ExecuteFinish.
    let err = sentinel_driver::Error::Protocol("synthetic".to_string());
    adapter.on_event(&Event::ExecuteFinish {
        stmt: StmtRef::Inline { sql },
        rows: 0,
        duration: Duration::from_micros(100),
        outcome: Outcome::Err(&err),
    });

    // Slow query path
    let slow = TracingInstrumentation {
        max_sql_len: 1024,
        slow_threshold: Some(Duration::from_nanos(1)),
    };
    slow.on_event(&Event::ExecuteFinish {
        stmt: StmtRef::Inline { sql },
        rows: 1,
        duration: Duration::from_millis(50),
        outcome: Outcome::Ok,
    });
}
