use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use futures_core::future::BoxFuture;

use crate::error::Result;
use crate::pool::health::HealthCheckStrategy;
use crate::Connection;

/// Called once per newly created connection, after TCP + TLS + auth completes.
///
/// Use for session setup like `SET search_path`. Error → connection discarded, pool retries.
pub type ConnectCallback = Arc<dyn Fn(&mut Connection) -> BoxFuture<'_, Result<()>> + Send + Sync>;

/// Called before returning a connection from the pool.
///
/// Return `false` to reject — connection discarded, pool tries next idle or creates new.
/// Error → connection discarded.
pub type AcquireCallback =
    Arc<dyn Fn(&mut Connection) -> BoxFuture<'_, Result<bool>> + Send + Sync>;

/// Called when a connection returns to the pool.
///
/// Return `false` to discard instead of returning to idle queue.
/// Error → connection discarded.
pub type ReleaseCallback =
    Arc<dyn Fn(&mut Connection) -> BoxFuture<'_, Result<bool>> + Send + Sync>;

/// Configuration for the connection pool.
///
/// Supports lifecycle callbacks for connection setup, validation, and cleanup.
/// All callbacks are optional and default to `None`.
#[derive(Clone)]
pub struct PoolConfig {
    pub(crate) max_connections: usize,
    pub(crate) min_connections: usize,
    pub(crate) connect_timeout: Duration,
    pub(crate) idle_timeout: Option<Duration>,
    pub(crate) max_lifetime: Option<Duration>,
    pub(crate) health_check: HealthCheckStrategy,
    pub(crate) acquire_timeout: Duration,
    pub(crate) after_connect: Option<ConnectCallback>,
    pub(crate) before_acquire: Option<AcquireCallback>,
    pub(crate) after_release: Option<ReleaseCallback>,
}

impl fmt::Debug for PoolConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PoolConfig")
            .field("max_connections", &self.max_connections)
            .field("min_connections", &self.min_connections)
            .field("connect_timeout", &self.connect_timeout)
            .field("idle_timeout", &self.idle_timeout)
            .field("max_lifetime", &self.max_lifetime)
            .field("health_check", &self.health_check)
            .field("acquire_timeout", &self.acquire_timeout)
            .field("after_connect", &self.after_connect.as_ref().map(|_| ".."))
            .field(
                "before_acquire",
                &self.before_acquire.as_ref().map(|_| ".."),
            )
            .field("after_release", &self.after_release.as_ref().map(|_| ".."))
            .finish()
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: num_cpus(),
            min_connections: 1,
            connect_timeout: Duration::from_secs(10),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(3600)),
            health_check: HealthCheckStrategy::Fast,
            acquire_timeout: Duration::from_secs(30),
            after_connect: None,
            before_acquire: None,
            after_release: None,
        }
    }
}

impl PoolConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Maximum number of connections in the pool.
    ///
    /// Default: 2 * number of CPUs.
    pub fn max_connections(mut self, n: usize) -> Self {
        self.max_connections = n;
        self
    }

    /// Minimum number of idle connections to maintain.
    ///
    /// The pool will create connections in the background to maintain this minimum.
    /// Default: 1.
    pub fn min_connections(mut self, n: usize) -> Self {
        self.min_connections = n;
        self
    }

    /// Timeout for establishing new connections.
    ///
    /// Default: 10 seconds.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Maximum time a connection can sit idle before being closed.
    ///
    /// Set to `None` to disable idle timeout. Default: 600 seconds.
    pub fn idle_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.idle_timeout = timeout;
        self
    }

    /// Maximum total lifetime of a connection before it's recycled.
    ///
    /// Set to `None` to disable max lifetime. Default: 3600 seconds.
    pub fn max_lifetime(mut self, lifetime: Option<Duration>) -> Self {
        self.max_lifetime = lifetime;
        self
    }

    /// Strategy for checking connection health on checkout.
    ///
    /// Default: `Fast` (flag-based, no query).
    pub fn health_check(mut self, strategy: HealthCheckStrategy) -> Self {
        self.health_check = strategy;
        self
    }

    /// Timeout for acquiring a connection from the pool.
    ///
    /// If the pool is full and no connection becomes available within this
    /// duration, an error is returned. Default: 30 seconds.
    pub fn acquire_timeout(mut self, timeout: Duration) -> Self {
        self.acquire_timeout = timeout;
        self
    }

    /// Set a callback that runs once per newly created connection.
    ///
    /// Called after TCP + TLS + auth completes, before the connection enters
    /// the pool. Use for session setup like `SET search_path`.
    ///
    /// If the callback returns an error, the connection is discarded and
    /// the pool retries with a new connection.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use sentinel_driver::pool::config::PoolConfig;
    /// PoolConfig::new()
    ///     .after_connect(|conn| Box::pin(async move {
    ///         conn.execute("SET search_path TO myapp", &[]).await?;
    ///         Ok(())
    ///     }));
    /// ```
    pub fn after_connect<F>(mut self, callback: F) -> Self
    where
        F: Fn(&mut Connection) -> BoxFuture<'_, Result<()>> + Send + Sync + 'static,
    {
        self.after_connect = Some(Arc::new(callback));
        self
    }

    /// Set a callback that runs before returning a connection from the pool.
    ///
    /// Called after health check passes. Return `false` to reject the
    /// connection — it will be discarded and the pool tries the next idle
    /// connection or creates a new one.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use sentinel_driver::pool::config::PoolConfig;
    /// PoolConfig::new()
    ///     .before_acquire(|conn| Box::pin(async move {
    ///         Ok(!conn.is_broken())
    ///     }));
    /// ```
    pub fn before_acquire<F>(mut self, callback: F) -> Self
    where
        F: Fn(&mut Connection) -> BoxFuture<'_, Result<bool>> + Send + Sync + 'static,
    {
        self.before_acquire = Some(Arc::new(callback));
        self
    }

    /// Set a callback that runs when a connection returns to the pool.
    ///
    /// Called before the connection enters the idle queue. Return `false`
    /// to discard the connection instead of returning it.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use sentinel_driver::pool::config::PoolConfig;
    /// PoolConfig::new()
    ///     .after_release(|conn| Box::pin(async move {
    ///         Ok(true) // always return to pool
    ///     }));
    /// ```
    pub fn after_release<F>(mut self, callback: F) -> Self
    where
        F: Fn(&mut Connection) -> BoxFuture<'_, Result<bool>> + Send + Sync + 'static,
    {
        self.after_release = Some(Arc::new(callback));
        self
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism().map_or(8, |n| n.get() * 2)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
