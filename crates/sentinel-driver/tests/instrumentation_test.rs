//! Live-PG tests verifying every wire site emits the expected events.
//! Skips silently when DATABASE_URL is unset.

use std::sync::{Arc, Mutex};

use sentinel_driver::pool::config::PoolConfig;
use sentinel_driver::{
    AcquireOutcome, Config, Connection, Event, Instrumentation, Outcome, Pool,
};

#[derive(Default)]
struct Recorder(Mutex<Vec<OwnedEvent>>);

#[derive(Debug, Clone, PartialEq)]
enum OwnedEvent {
    ExecuteStart { sql: String, param_count: usize },
    ExecuteFinish { sql: String, rows: u64, ok: bool },
    PrepareFinish { cache_hit: bool },
    TxBegin,
    TxCommit,
    TxRollback,
    PipelineFlush { batch_len: usize },
    PoolAcquireFinish { ok: bool },
    PoolRelease,
}

impl Instrumentation for Recorder {
    fn on_event(&self, ev: &Event<'_>) {
        let owned = match ev {
            Event::ExecuteStart { stmt, param_count } => OwnedEvent::ExecuteStart {
                sql: stmt.sql_or_name().to_string(),
                param_count: *param_count,
            },
            Event::ExecuteFinish { stmt, rows, outcome, .. } => OwnedEvent::ExecuteFinish {
                sql: stmt.sql_or_name().to_string(),
                rows: *rows,
                ok: matches!(outcome, Outcome::Ok),
            },
            Event::PrepareFinish { cache_hit, .. } => OwnedEvent::PrepareFinish {
                cache_hit: *cache_hit,
            },
            Event::TxBegin { .. } => OwnedEvent::TxBegin,
            Event::TxCommit { .. } => OwnedEvent::TxCommit,
            Event::TxRollback { .. } => OwnedEvent::TxRollback,
            Event::PipelineFlush { batch_len, .. } => OwnedEvent::PipelineFlush {
                batch_len: *batch_len,
            },
            Event::PoolAcquireFinish { outcome, .. } => OwnedEvent::PoolAcquireFinish {
                ok: matches!(outcome, AcquireOutcome::Ok),
            },
            Event::PoolRelease => OwnedEvent::PoolRelease,
            _ => return,
        };
        self.0.lock().unwrap().push(owned);
    }
}

async fn connect() -> Option<(Connection, Arc<Recorder>)> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let rec = Arc::new(Recorder::default());
    let cfg = Config::parse(&url).ok()?.with_instrumentation(rec.clone());
    let mut conn = Connection::connect(cfg).await.ok()?;
    // Suppress NOTICEs in the connection so `DROP TABLE IF EXISTS` etc.
    // don't cause the driver to bail with a Protocol error.
    conn.execute("SET client_min_messages = ERROR", &[]).await.ok()?;
    rec.0.lock().unwrap().clear();
    Some((conn, rec))
}

#[tokio::test]
async fn query_emits_start_then_finish() {
    let Some((mut conn, rec)) = connect().await else { return };
    conn.query("SELECT 1::int4", &[]).await.unwrap();
    let evs = rec.0.lock().unwrap();
    assert!(matches!(evs.first(), Some(OwnedEvent::ExecuteStart { .. })),
        "expected ExecuteStart first, got: {:?}", evs);
    assert!(matches!(evs.last(),  Some(OwnedEvent::ExecuteFinish { ok: true, .. })),
        "expected ExecuteFinish ok last, got: {:?}", evs);
}

#[tokio::test]
async fn transaction_emits_begin_then_commit() {
    let Some((mut conn, rec)) = connect().await else { return };
    conn.begin().await.unwrap();
    conn.commit().await.unwrap();
    let evs: Vec<_> = rec.0.lock().unwrap().iter()
        .filter(|e| matches!(e, OwnedEvent::TxBegin | OwnedEvent::TxCommit))
        .cloned()
        .collect();
    assert_eq!(evs, vec![OwnedEvent::TxBegin, OwnedEvent::TxCommit]);
}

#[tokio::test]
async fn prepare_emits_finish_with_cache_hit_false() {
    let Some((mut conn, rec)) = connect().await else { return };
    // Note: the current driver's prepare() always misses (cache wiring is
    // a future task). We verify the event is emitted, not the hit semantics.
    let _ = conn.prepare("SELECT 1::int4").await.unwrap();
    let pf: Vec<_> = rec.0.lock().unwrap().iter()
        .filter(|e| matches!(e, OwnedEvent::PrepareFinish { .. }))
        .cloned()
        .collect();
    assert_eq!(pf.len(), 1);
    assert!(matches!(pf[0], OwnedEvent::PrepareFinish { cache_hit: false }));
}

#[tokio::test]
async fn pool_acquire_release_pair() {
    let Some(url) = std::env::var("DATABASE_URL").ok() else { return };
    let rec = Arc::new(Recorder::default());
    let cfg = Config::parse(&url).unwrap();
    let pool = Pool::new(cfg, PoolConfig::new().max_connections(4))
        .with_instrumentation(rec.clone());
    {
        let _conn = pool.acquire().await.unwrap();
        // _conn dropped here → PoolRelease emitted synchronously
    }
    // Wait for any deferred Tokio spawn from the drop path to finish
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let evs: Vec<_> = rec.0.lock().unwrap().iter()
        .filter(|e| matches!(e,
            OwnedEvent::PoolAcquireFinish { .. } | OwnedEvent::PoolRelease))
        .cloned()
        .collect();
    assert!(!evs.is_empty(), "expected pool events, got nothing");
    assert!(matches!(evs[0], OwnedEvent::PoolAcquireFinish { ok: true }),
        "first pool event should be successful acquire, got: {:?}", evs);
    assert_eq!(evs[1], OwnedEvent::PoolRelease);
}
