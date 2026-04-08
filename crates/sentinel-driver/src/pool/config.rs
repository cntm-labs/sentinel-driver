use std::time::Duration;

use crate::pool::health::HealthCheckStrategy;

/// Configuration for the connection pool.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_connections: usize,
    pub min_connections: usize,
    pub connect_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
    pub health_check: HealthCheckStrategy,
    pub acquire_timeout: Duration,
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
}

fn num_cpus() -> usize {
    std::thread::available_parallelism().map_or(8, |n| n.get() * 2)
}
