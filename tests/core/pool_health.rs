use sentinel_driver::pool::health::HealthCheckStrategy;

#[test]
fn test_health_check_strategy_variants() {
    // Verify the Query variant exists and is usable
    let strategy = HealthCheckStrategy::Query;
    assert_eq!(strategy, HealthCheckStrategy::Query);

    let fast = HealthCheckStrategy::Fast;
    assert_eq!(fast, HealthCheckStrategy::Fast);

    let none = HealthCheckStrategy::None;
    assert_eq!(none, HealthCheckStrategy::None);
}
