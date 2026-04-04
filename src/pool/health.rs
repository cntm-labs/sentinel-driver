use std::time::Instant;

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
pub(crate) struct ConnectionMeta {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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
}
