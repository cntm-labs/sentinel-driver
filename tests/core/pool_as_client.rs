//! Verify the &Pool: GenericClient blanket impl. Live-PG, skips without DATABASE_URL.

use sentinel_driver::pool::config::PoolConfig;
use sentinel_driver::{Config, GenericClient, Pool};

async fn make_pool() -> Option<Pool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let cfg = Config::parse(&url).ok()?;
    Some(Pool::new(cfg, PoolConfig::new().max_connections(2)))
}

#[tokio::test]
async fn pool_ref_query() {
    let Some(pool) = make_pool().await else { return };
    let mut p = &pool; // bind so we have `&mut &Pool` for trait dispatch
    let rows = p.query("SELECT 1::int4 AS n", &[]).await.unwrap();
    assert_eq!(rows.len(), 1);
    let n: i32 = rows[0].try_get(0).unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn pool_ref_query_one() {
    let Some(pool) = make_pool().await else { return };
    let mut p = &pool;
    let row = p.query_one("SELECT 'hello'::text", &[]).await.unwrap();
    let s: String = row.try_get(0).unwrap();
    assert_eq!(s, "hello");
}

#[tokio::test]
async fn pool_ref_query_typed_one() {
    let Some(pool) = make_pool().await else { return };
    let mut p = &pool;
    let row = p
        .query_typed_one(
            "SELECT $1::int4 + 1",
            &[(
                &41_i32 as &(dyn sentinel_driver::ToSql + Sync),
                sentinel_driver::Oid::INT4,
            )],
        )
        .await
        .unwrap();
    let n: i32 = row.try_get(0).unwrap();
    assert_eq!(n, 42);
}

#[tokio::test]
async fn pool_ref_execute() {
    let Some(pool) = make_pool().await else { return };
    let mut p = &pool;
    let n = p.execute("SELECT 1", &[]).await.unwrap();
    // SELECT returns "affected rows" = 0 in PG's CommandComplete
    assert_eq!(n, 0);
}
