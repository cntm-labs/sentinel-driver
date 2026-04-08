use sentinel_driver::pool::config::PoolConfig;
use sentinel_driver::pool::Pool;
use sentinel_driver::PoolMetrics;

#[tokio::test]
async fn test_pool_metrics_initial() {
    let config =
        sentinel_driver::Config::parse("postgres://user:pass@localhost/db").expect("valid config");
    let pool = Pool::connect_lazy(config, PoolConfig::new().max_connections(10));

    let metrics = pool.metrics().await;
    assert_eq!(metrics.active, 0);
    assert_eq!(metrics.idle, 0);
    assert_eq!(metrics.total, 0);
    assert_eq!(metrics.max, 10);
}

#[test]
fn test_pool_metrics_debug() {
    let m = PoolMetrics {
        active: 2,
        idle: 3,
        total: 5,
        max: 10,
    };
    let debug = format!("{m:?}");
    assert!(debug.contains("active: 2"));
    assert!(debug.contains("idle: 3"));
}

#[test]
fn test_pool_metrics_copy_clone() {
    let m = PoolMetrics {
        active: 1,
        idle: 2,
        total: 3,
        max: 5,
    };
    let m2 = m; // Copy
    let m3 = m.clone(); // Clone
    assert_eq!(m2.active, m3.active);
}
