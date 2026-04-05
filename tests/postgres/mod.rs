use sentinel_driver::pool::config::PoolConfig;
use sentinel_driver::pool::health::HealthCheckStrategy;
use sentinel_driver::pool::Pool;
use sentinel_driver::Config;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}

macro_rules! require_pg {
    () => {
        match database_url() {
            Some(url) => url,
            None => return,
        }
    };
}

#[tokio::test]
async fn test_connect() {
    let url = require_pg!();
    let config = Config::parse(&url).unwrap();
    let conn = sentinel_driver::Connection::connect(config).await.unwrap();
    conn.close().await.unwrap();
}

#[tokio::test]
async fn test_pool_acquire_fast_strategy() {
    let url = require_pg!();
    let config = Config::parse(&url).unwrap();
    let pool_config = PoolConfig::new()
        .max_connections(2)
        .health_check(HealthCheckStrategy::Fast);
    let pool = Pool::new(config, pool_config);

    // First acquire — creates new connection
    let conn1 = pool.acquire().await.unwrap();
    assert_eq!(pool.total_count().await, 1);
    drop(conn1);

    // Wait briefly for connection to be returned to pool
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Second acquire — reuses idle connection (Fast strategy, no query)
    let _conn2 = pool.acquire().await.unwrap();
    assert_eq!(pool.total_count().await, 1);
}

#[tokio::test]
async fn test_pool_acquire_query_strategy() {
    let url = require_pg!();
    let config = Config::parse(&url).unwrap();
    let pool_config = PoolConfig::new()
        .max_connections(2)
        .health_check(HealthCheckStrategy::Query);
    let pool = Pool::new(config, pool_config);

    // First acquire — creates new connection
    let conn1 = pool.acquire().await.unwrap();
    assert_eq!(pool.total_count().await, 1);
    drop(conn1);

    // Wait briefly for connection to be returned to pool
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Second acquire — reuses idle connection after check_alive passes
    let _conn2 = pool.acquire().await.unwrap();
    assert_eq!(pool.total_count().await, 1);
}
