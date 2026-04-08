use std::time::Duration;

use sentinel_driver::pool::config::PoolConfig;
use sentinel_driver::pool::Pool;

#[test]
fn test_after_connect_builder_compiles() {
    let _config = PoolConfig::new().after_connect(|_conn| Box::pin(async move { Ok(()) }));
}

#[test]
fn test_before_acquire_builder_compiles() {
    let _config = PoolConfig::new().before_acquire(|_conn| Box::pin(async move { Ok(true) }));
}

#[test]
fn test_after_release_builder_compiles() {
    let _config = PoolConfig::new().after_release(|_conn| Box::pin(async move { Ok(true) }));
}

#[test]
fn test_all_callbacks_chained() {
    let _config = PoolConfig::new()
        .after_connect(|_conn| Box::pin(async move { Ok(()) }))
        .before_acquire(|_conn| Box::pin(async move { Ok(true) }))
        .after_release(|_conn| Box::pin(async move { Ok(true) }));
}

#[test]
fn test_callbacks_with_other_config() {
    let _config = PoolConfig::new()
        .max_connections(20)
        .after_connect(|_conn| Box::pin(async move { Ok(()) }))
        .idle_timeout(None)
        .before_acquire(|_conn| Box::pin(async move { Ok(true) }))
        .acquire_timeout(Duration::from_secs(5));
}

#[test]
fn test_pool_config_debug_with_callbacks() {
    let config = PoolConfig::new().after_connect(|_conn| Box::pin(async move { Ok(()) }));

    let debug = format!("{config:?}");
    assert!(debug.contains("after_connect"));
    assert!(debug.contains("before_acquire"));
    assert!(debug.contains("after_release"));
}

#[test]
fn test_pool_config_debug_without_callbacks() {
    let config = PoolConfig::new();
    let debug = format!("{config:?}");
    // Callback fields should show None
    assert!(debug.contains("after_connect: None"));
    assert!(debug.contains("before_acquire: None"));
    assert!(debug.contains("after_release: None"));
}

#[test]
fn test_pool_config_clone_with_callbacks() {
    let config = PoolConfig::new()
        .after_connect(|_conn| Box::pin(async move { Ok(()) }))
        .before_acquire(|_conn| Box::pin(async move { Ok(true) }));

    // Clone should compile and not panic
    let _cloned = config.clone();
}

#[test]
fn test_connect_lazy_creates_pool() {
    let config =
        sentinel_driver::Config::parse("postgres://user:pass@localhost/db").expect("valid config");
    let pool = Pool::connect_lazy(config, PoolConfig::new().max_connections(5));

    assert_eq!(pool.max_connections(), 5);
}

#[tokio::test]
async fn test_connect_lazy_zero_initial_connections() {
    let config =
        sentinel_driver::Config::parse("postgres://user:pass@localhost/db").expect("valid config");
    let pool = Pool::connect_lazy(config, PoolConfig::new());

    assert_eq!(pool.idle_count().await, 0);
    assert_eq!(pool.total_count().await, 0);
}

#[test]
fn test_connect_lazy_with_callbacks() {
    let config =
        sentinel_driver::Config::parse("postgres://user:pass@localhost/db").expect("valid config");

    let pool_config = PoolConfig::new()
        .max_connections(10)
        .after_connect(|conn| {
            Box::pin(async move {
                conn.execute("SET search_path TO myapp", &[]).await?;
                Ok(())
            })
        })
        .before_acquire(|conn| Box::pin(async move { Ok(!conn.is_broken()) }))
        .after_release(|_conn| Box::pin(async move { Ok(true) }));

    let pool = Pool::connect_lazy(config, pool_config);
    assert_eq!(pool.max_connections(), 10);
}
