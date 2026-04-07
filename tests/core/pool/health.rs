use std::time::Duration;

use sentinel_driver::pool::health::{ConnectionMeta, HealthCheckStrategy};

#[test]
fn test_connection_meta_new() {
    let meta = ConnectionMeta::new();
    assert!(!meta.is_broken);
    assert!(meta.created_at.elapsed() < Duration::from_secs(1));
}

#[test]
fn test_connection_meta_touch() {
    let mut meta = ConnectionMeta::new();
    let first_used = meta.last_used;
    // Touch updates last_used
    std::thread::sleep(Duration::from_millis(1));
    meta.touch();
    assert!(meta.last_used >= first_used);
}

#[test]
fn test_idle_not_expired() {
    let meta = ConnectionMeta::new();
    assert!(!meta.is_idle_expired(Duration::from_secs(600)));
}

#[test]
fn test_lifetime_not_expired() {
    let meta = ConnectionMeta::new();
    assert!(!meta.is_lifetime_expired(Duration::from_secs(3600)));
}

#[test]
fn test_health_check_strategy_variants() {
    let strategy = HealthCheckStrategy::Query;
    assert_eq!(strategy, HealthCheckStrategy::Query);

    let fast = HealthCheckStrategy::Fast;
    assert_eq!(fast, HealthCheckStrategy::Fast);

    let none = HealthCheckStrategy::None;
    assert_eq!(none, HealthCheckStrategy::None);
}
