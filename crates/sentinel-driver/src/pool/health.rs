use std::time::Instant;

use crate::connection::stream::PgConnection;
use crate::protocol::backend::BackendMessage;
use crate::protocol::frontend;

/// Check if a connection is still alive by sending an empty query.
///
/// Sends `""` via simple query protocol. The server responds with
/// `EmptyQueryResponse` + `ReadyForQuery`. Returns `false` on any error.
/// Cost: ~50us round-trip.
pub(crate) async fn check_alive(conn: &mut PgConnection) -> bool {
    frontend::query(conn.write_buf(), "");
    try_check_alive(conn).await.unwrap_or(false)
}

/// Inner function that uses `?` for unified error handling.
async fn try_check_alive(conn: &mut PgConnection) -> crate::error::Result<bool> {
    conn.send().await?;

    // Drain until ReadyForQuery
    loop {
        if matches!(conn.recv().await?, BackendMessage::ReadyForQuery { .. }) {
            return Ok(true);
        }
    }
}

/// Strategy for checking connection health.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthCheckStrategy {
    /// Flag-based check — no query, just check if the connection has
    /// been marked as broken by a previous I/O error. Fastest option (<0.5μs).
    Fast,
    /// Send `SELECT 1` to verify the connection is alive.
    /// More reliable but adds ~100μs per checkout.
    Query,
    /// No health check — assume connections are always valid.
    /// Use only in controlled environments.
    None,
}

/// Metadata for a pooled connection — used for idle timeout and max lifetime.
#[derive(Debug)]
pub struct ConnectionMeta {
    /// When this connection was created.
    pub created_at: Instant,
    /// When this connection was last returned to the pool.
    pub last_used: Instant,
    /// Flag set when an I/O error occurs — marks connection as broken.
    pub is_broken: bool,
}

impl ConnectionMeta {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            created_at: now,
            last_used: now,
            is_broken: false,
        }
    }

    /// Mark this connection as recently used (returned to pool).
    pub fn touch(&mut self) {
        self.last_used = Instant::now();
    }

    /// Check if the connection has exceeded the idle timeout.
    pub fn is_idle_expired(&self, timeout: std::time::Duration) -> bool {
        self.last_used.elapsed() > timeout
    }

    /// Check if the connection has exceeded its max lifetime.
    pub fn is_lifetime_expired(&self, max_lifetime: std::time::Duration) -> bool {
        self.created_at.elapsed() > max_lifetime
    }
}
