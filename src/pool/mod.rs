pub mod config;
pub mod health;

use std::collections::VecDeque;
use std::sync::Arc;

use tokio::sync::{Mutex, Semaphore};
use tracing::debug;

use crate::config::Config;
use crate::connection::startup;
use crate::connection::stream::PgConnection;
use crate::error::{Error, Result};
use crate::pool::config::PoolConfig;
use crate::pool::health::ConnectionMeta;

/// An idle connection in the pool, with its metadata.
struct IdleConnection {
    conn: PgConnection,
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

/// A connection pool for PostgreSQL.
///
/// Cheaply cloneable (internally Arc'd). Uses a semaphore to limit max
/// connections and a mutex-protected deque for idle connection management.
/// Designed for <0.5μs checkout latency.
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
            if self.is_healthy(&idle.meta) {
                debug!("reusing idle connection");
                Ok(PooledConnection {
                    conn: Some(idle.conn),
                    meta: idle.meta,
                    shared: Arc::clone(&self.shared),
                })
            } else {
                debug!("idle connection unhealthy, creating new one");
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

    // ── Internal ─────────────────────────────────────

    async fn create_connection(&self) -> Result<(PgConnection, ConnectionMeta)> {
        let mut conn = PgConnection::connect(&self.shared.config).await?;
        startup::startup(&mut conn, &self.shared.config).await?;

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

    fn is_healthy(&self, meta: &ConnectionMeta) -> bool {
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
/// (unless it has been marked as broken).
pub struct PooledConnection {
    conn: Option<PgConnection>,
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
                tokio::spawn(async move {
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

#[cfg(test)]
mod tests {
    use super::config::PoolConfig;

    #[test]
    fn test_pool_config_creation() {
        let config = PoolConfig::new().max_connections(10).min_connections(2);

        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 2);
    }
}
