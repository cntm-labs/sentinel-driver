use std::time::Duration;

use sentinel_driver::pool::config::PoolConfig;
use sentinel_driver::pool::health::HealthCheckStrategy;

#[test]
fn test_pool_config_defaults() {
    let config = PoolConfig::new();
    assert!(config.max_connections >= 2);
    assert_eq!(config.min_connections, 1);
    assert_eq!(config.connect_timeout, Duration::from_secs(10));
    assert_eq!(config.idle_timeout, Some(Duration::from_secs(600)));
    assert_eq!(config.max_lifetime, Some(Duration::from_secs(3600)));
    assert_eq!(config.acquire_timeout, Duration::from_secs(30));
}

#[test]
fn test_pool_config_builder() {
    let config = PoolConfig::new()
        .max_connections(20)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(5))
        .idle_timeout(None)
        .max_lifetime(Some(Duration::from_secs(7200)))
        .health_check(HealthCheckStrategy::Query)
        .acquire_timeout(Duration::from_secs(10));

    assert_eq!(config.max_connections, 20);
    assert_eq!(config.min_connections, 5);
    assert_eq!(config.connect_timeout, Duration::from_secs(5));
    assert_eq!(config.idle_timeout, None);
    assert_eq!(config.max_lifetime, Some(Duration::from_secs(7200)));
    assert!(matches!(config.health_check, HealthCheckStrategy::Query));
    assert_eq!(config.acquire_timeout, Duration::from_secs(10));
}

#[test]
fn test_pool_config_creation() {
    let config = PoolConfig::new().max_connections(10).min_connections(2);

    assert_eq!(config.max_connections, 10);
    assert_eq!(config.min_connections, 2);
}
