pub mod config;
pub mod health;

use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use tokio::sync::{Mutex, Semaphore};
use tracing::debug;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::pool::config::PoolConfig;
use crate::pool::health::{ConnectionMeta, HealthCheckStrategy};
use crate::Connection;

/// An idle connection in the pool, with its metadata.
struct IdleConnection {
    conn: Connection,
    meta: ConnectionMeta,
}

/// Shared inner state of the pool, protected by a Mutex.
struct PoolState {
    idle: VecDeque<IdleConnection>,
    total_count: usize,
}

/// Shared data that lives behind an Arc, so PooledConnection can own a clone.
struct PoolShared {
    config: Config,
    pool_config: PoolConfig,
    semaphore: Semaphore,
    state: Mutex<PoolState>,
}

/// Snapshot of pool statistics.
///
/// Cheap to produce — reads from pool state under a single lock.
#[derive(Debug, Clone, Copy)]
pub struct PoolMetrics {
    /// Number of connections currently checked out by users.
    pub active: usize,
    /// Number of idle connections available for checkout.
    pub idle: usize,
    /// Total connections (active + idle).
    pub total: usize,
    /// Maximum allowed connections.
    pub max: usize,
}

/// A connection pool for PostgreSQL.
///
/// Cheaply cloneable (internally Arc'd). Uses a semaphore to limit max
/// connections and a mutex-protected deque for idle connection management.
/// Designed for <0.5μs checkout latency.
///
/// # Lifecycle Callbacks
///
/// Three optional callbacks control connection lifecycle:
/// - `after_connect` — runs once per new connection (session setup)
/// - `before_acquire` — runs before handing out a connection (validation)
/// - `after_release` — runs when a connection returns to the pool (cleanup)
///
/// # Example
///
/// ```rust,no_run
/// use sentinel_driver::{Config, pool::{Pool, config::PoolConfig}};
/// use std::time::Duration;
///
/// # async fn example() -> sentinel_driver::Result<()> {
/// let config = Config::parse("postgres://user:pass@localhost/db")?;
/// let pool = Pool::new(config, PoolConfig::new().max_connections(10));
///
/// let conn = pool.acquire().await?;
/// // use conn...
/// // conn is returned to pool on drop
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Pool {
    shared: Arc<PoolShared>,
}

impl Pool {
    /// Create a new connection pool. No connections are opened until `acquire()`.
    pub fn new(config: Config, pool_config: PoolConfig) -> Self {
        let shared = Arc::new(PoolShared {
            semaphore: Semaphore::new(pool_config.max_connections),
            config,
            pool_config,
            state: Mutex::new(PoolState {
                idle: VecDeque::new(),
                total_count: 0,
            }),
        });

        Self { shared }
    }

    /// Create a pool that defers all connection establishment until the
    /// first `acquire()` call.
    ///
    /// This is identical to `new()` — both are lazy. Provided for API
    /// compatibility with connection pools that eagerly open connections.
    ///
    /// ```rust,no_run
    /// # use sentinel_driver::{Config, pool::{Pool, config::PoolConfig}};
    /// # fn example() -> sentinel_driver::Result<()> {
    /// let config = Config::parse("postgres://user:pass@localhost/db")?;
    /// let pool = Pool::connect_lazy(config, PoolConfig::new());
    /// // No connections opened yet — first acquire() will connect.
    /// # Ok(())
    /// # }
    /// ```
    pub fn connect_lazy(config: Config, pool_config: PoolConfig) -> Self {
        Self::new(config, pool_config)
    }

    /// Acquire a connection from the pool.
    ///
    /// If an idle connection is available, it's returned immediately.
    /// Otherwise, a new connection is created (up to `max_connections`).
    /// If the pool is full, waits up to `acquire_timeout`.
    pub async fn acquire(&self) -> Result<PooledConnection> {
        let permit = tokio::time::timeout(
            self.shared.pool_config.acquire_timeout,
            self.shared.semaphore.acquire(),
        )
        .await
        .map_err(|_| Error::Pool("acquire timeout: pool exhausted".into()))?
        .map_err(|_| Error::Pool("pool closed".into()))?;

        // Release semaphore permit immediately — we track count ourselves.
        // The semaphore just rate-limits concurrent acquires.
        drop(permit);

        // Try to get an idle connection
        let idle_conn = {
            let mut state = self.shared.state.lock().await;
            state.idle.pop_front()
        };

        if let Some(idle) = idle_conn {
            if self.is_fresh(&idle.meta) {
                let mut conn = idle.conn;
                // If Query strategy, verify connection is alive
                if self.shared.pool_config.health_check == HealthCheckStrategy::Query
                    && !health::check_alive(conn.pg_connection_mut()).await
                {
                    debug!("idle connection failed health check, creating new one");
                    self.decrement_count().await;
                    let (conn, meta) = self.create_connection().await?;
                    return Ok(PooledConnection {
                        conn: Some(conn),
                        meta,
                        shared: Arc::clone(&self.shared),
                    });
                }

                // Run before_acquire callback
                if let Some(ref cb) = self.shared.pool_config.before_acquire {
                    match cb(&mut conn).await {
                        Ok(true) => { /* connection accepted */ }
                        Ok(false) => {
                            debug!("before_acquire rejected connection");
                            self.decrement_count().await;
                            let (conn, meta) = self.create_connection().await?;
                            return Ok(PooledConnection {
                                conn: Some(conn),
                                meta,
                                shared: Arc::clone(&self.shared),
                            });
                        }
                        Err(_) => {
                            debug!("before_acquire callback error, discarding connection");
                            self.decrement_count().await;
                            let (conn, meta) = self.create_connection().await?;
                            return Ok(PooledConnection {
                                conn: Some(conn),
                                meta,
                                shared: Arc::clone(&self.shared),
                            });
                        }
                    }
                }

                debug!("reusing idle connection");
                Ok(PooledConnection {
                    conn: Some(conn),
                    meta: idle.meta,
                    shared: Arc::clone(&self.shared),
                })
            } else {
                debug!("idle connection expired, creating new one");
                self.decrement_count().await;
                let (conn, meta) = self.create_connection().await?;
                Ok(PooledConnection {
                    conn: Some(conn),
                    meta,
                    shared: Arc::clone(&self.shared),
                })
            }
        } else {
            let (conn, meta) = self.create_connection().await?;
            Ok(PooledConnection {
                conn: Some(conn),
                meta,
                shared: Arc::clone(&self.shared),
            })
        }
    }

    /// Number of idle connections.
    pub async fn idle_count(&self) -> usize {
        self.shared.state.lock().await.idle.len()
    }

    /// Total number of connections (idle + in use).
    pub async fn total_count(&self) -> usize {
        self.shared.state.lock().await.total_count
    }

    /// Maximum number of connections allowed.
    pub fn max_connections(&self) -> usize {
        self.shared.pool_config.max_connections
    }

    /// Get a snapshot of pool metrics.
    pub async fn metrics(&self) -> PoolMetrics {
        let state = self.shared.state.lock().await;
        let idle = state.idle.len();
        let total = state.total_count;
        PoolMetrics {
            active: total.saturating_sub(idle),
            idle,
            total,
            max: self.shared.pool_config.max_connections,
        }
    }

    // ── Internal ─────────────────────────────────────

    async fn create_connection(&self) -> Result<(Connection, ConnectionMeta)> {
        let mut conn = Connection::connect(self.shared.config.clone()).await?;

        // Run after_connect callback
        if let Some(ref cb) = self.shared.pool_config.after_connect {
            if let Err(e) = cb(&mut conn).await {
                debug!(?e, "after_connect callback failed, discarding connection");
                return Err(e);
            }
        }

        let meta = ConnectionMeta::new();

        let mut state = self.shared.state.lock().await;
        state.total_count += 1;
        debug!(total = state.total_count, "created new connection");

        Ok((conn, meta))
    }

    async fn decrement_count(&self) {
        let mut state = self.shared.state.lock().await;
        state.total_count = state.total_count.saturating_sub(1);
    }

    fn is_fresh(&self, meta: &ConnectionMeta) -> bool {
        if meta.is_broken {
            return false;
        }

        if let Some(timeout) = self.shared.pool_config.idle_timeout {
            if meta.is_idle_expired(timeout) {
                return false;
            }
        }

        if let Some(lifetime) = self.shared.pool_config.max_lifetime {
            if meta.is_lifetime_expired(lifetime) {
                return false;
            }
        }

        true
    }
}

/// A connection checked out from the pool.
///
/// When dropped, the connection is automatically returned to the pool
/// (unless it has been marked as broken). The `after_release` callback
/// runs before the connection re-enters the idle queue.
pub struct PooledConnection {
    conn: Option<Connection>,
    meta: ConnectionMeta,
    shared: Arc<PoolShared>,
}

impl PooledConnection {
    /// Mark this connection as broken. It will be discarded on drop
    /// instead of being returned to the pool.
    pub fn mark_broken(&mut self) {
        self.meta.is_broken = true;
    }
}

impl Deref for PooledConnection {
    type Target = Connection;

    #[allow(clippy::expect_used)]
    fn deref(&self) -> &Self::Target {
        self.conn
            .as_ref()
            .expect("PooledConnection used after drop")
    }
}

impl DerefMut for PooledConnection {
    #[allow(clippy::expect_used)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.conn
            .as_mut()
            .expect("PooledConnection used after drop")
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            let shared = Arc::clone(&self.shared);

            if self.meta.is_broken {
                tokio::spawn(async move {
                    drop(conn);
                    let mut state = shared.state.lock().await;
                    state.total_count = state.total_count.saturating_sub(1);
                    debug!("discarded broken connection");
                });
            } else {
                let created_at = self.meta.created_at;
                let after_release = self.shared.pool_config.after_release.clone();

                tokio::spawn(async move {
                    let mut conn = conn;

                    // Run after_release callback
                    if let Some(cb) = after_release {
                        match cb(&mut conn).await {
                            Ok(true) => { /* return to pool */ }
                            Ok(false) => {
                                debug!("after_release rejected connection, discarding");
                                let mut state = shared.state.lock().await;
                                state.total_count = state.total_count.saturating_sub(1);
                                return;
                            }
                            Err(_) => {
                                debug!("after_release callback error, discarding connection");
                                let mut state = shared.state.lock().await;
                                state.total_count = state.total_count.saturating_sub(1);
                                return;
                            }
                        }
                    }

                    let mut meta = ConnectionMeta::new();
                    meta.created_at = created_at;
                    meta.touch();

                    let mut state = shared.state.lock().await;
                    state.idle.push_back(IdleConnection { conn, meta });
                });
            }
        }
    }
}
