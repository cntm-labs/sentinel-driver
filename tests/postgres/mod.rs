mod stream;

use std::time::Duration;

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

    let conn1 = pool.acquire().await.unwrap();
    assert_eq!(pool.total_count().await, 1);
    drop(conn1);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Reuses idle connection (Fast strategy, no query)
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

    let conn1 = pool.acquire().await.unwrap();
    assert_eq!(pool.total_count().await, 1);
    drop(conn1);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Reuses idle connection after check_alive passes
    let _conn2 = pool.acquire().await.unwrap();
    assert_eq!(pool.total_count().await, 1);
}

#[tokio::test]
async fn test_pool_expired_idle_connection() {
    let url = require_pg!();
    let config = Config::parse(&url).unwrap();
    let pool_config = PoolConfig::new()
        .max_connections(2)
        .idle_timeout(Some(Duration::from_millis(1)))
        .health_check(HealthCheckStrategy::Fast);
    let pool = Pool::new(config, pool_config);

    let conn1 = pool.acquire().await.unwrap();
    assert_eq!(pool.total_count().await, 1);
    drop(conn1);

    // Wait for idle timeout to expire
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Idle connection expired → is_fresh returns false → creates new connection
    let _conn2 = pool.acquire().await.unwrap();
    // Old connection discarded, new one created
    assert_eq!(pool.total_count().await, 1);
}

#[tokio::test]
async fn test_pool_health_check_failure_creates_new_connection() {
    let url = require_pg!();
    let config = Config::parse(&url).unwrap();

    // Use a separate admin connection to terminate pool connections
    let mut admin = sentinel_driver::Connection::connect(Config::parse(&url).unwrap())
        .await
        .unwrap();

    let pool_config = PoolConfig::new()
        .max_connections(2)
        .health_check(HealthCheckStrategy::Query);
    let pool = Pool::new(config, pool_config);

    // Acquire and return a connection to the pool
    let conn1 = pool.acquire().await.unwrap();
    assert_eq!(pool.total_count().await, 1);
    drop(conn1);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Kill all other connections (the idle pooled connection) from the server side
    admin
        .simple_query(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
             WHERE pid != pg_backend_pid() AND datname = current_database()",
        )
        .await
        .unwrap();

    // Small delay for the termination to take effect
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Now acquire — check_alive should fail on the dead connection,
    // triggering the health check failure path to create a new one
    let _conn2 = pool.acquire().await.unwrap();
    admin.close().await.unwrap();
}
