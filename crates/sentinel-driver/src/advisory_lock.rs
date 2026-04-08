use std::hash::{Hash, Hasher};

use crate::error::Result;
use crate::Connection;

/// A PostgreSQL advisory lock identifier.
///
/// Advisory locks are application-level locks that don't lock any table or row.
/// They are useful for coordinating access to external resources.
///
/// # Example
///
/// ```rust,no_run
/// # async fn example(conn: &mut sentinel_driver::Connection) -> sentinel_driver::Result<()> {
/// use sentinel_driver::advisory_lock::PgAdvisoryLock;
///
/// let lock = PgAdvisoryLock::new(12345);
/// let guard = lock.acquire(conn).await?;
/// // ... do work under lock ...
/// guard.release(conn).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PgAdvisoryLock {
    key: i64,
}

impl PgAdvisoryLock {
    /// Create an advisory lock from a numeric key.
    pub fn new(key: i64) -> Self {
        Self { key }
    }

    /// Create an advisory lock from a string key.
    ///
    /// The string is hashed to produce a stable i64 key using the
    /// default hasher.
    pub fn from_name(name: &str) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        name.hash(&mut hasher);
        Self {
            key: hasher.finish() as i64,
        }
    }

    /// The numeric key for this lock.
    pub fn key(&self) -> i64 {
        self.key
    }

    /// Acquire this advisory lock (session-scoped, blocks until acquired).
    pub async fn acquire(&self, conn: &mut Connection) -> Result<PgAdvisoryLockGuard> {
        conn.execute("SELECT pg_advisory_lock($1)", &[&self.key])
            .await?;
        Ok(PgAdvisoryLockGuard { key: self.key })
    }

    /// Try to acquire this advisory lock without blocking.
    ///
    /// Returns `None` if the lock is already held by another session.
    pub async fn try_acquire(&self, conn: &mut Connection) -> Result<Option<PgAdvisoryLockGuard>> {
        let rows = conn
            .query("SELECT pg_try_advisory_lock($1)", &[&self.key])
            .await?;
        let acquired: bool = rows
            .first()
            .map(|r| r.try_get::<bool>(0))
            .transpose()?
            .unwrap_or(false);
        if acquired {
            Ok(Some(PgAdvisoryLockGuard { key: self.key }))
        } else {
            Ok(None)
        }
    }
}

/// A guard representing a held advisory lock.
///
/// The lock is NOT automatically released on drop — you must call `release()`.
/// This is intentional because releasing requires an async database call.
#[derive(Debug)]
pub struct PgAdvisoryLockGuard {
    key: i64,
}

impl PgAdvisoryLockGuard {
    /// The numeric key for this lock.
    pub fn key(&self) -> i64 {
        self.key
    }

    /// Release the advisory lock.
    pub async fn release(self, conn: &mut Connection) -> Result<()> {
        conn.execute("SELECT pg_advisory_unlock($1)", &[&self.key])
            .await?;
        Ok(())
    }
}
