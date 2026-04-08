//! # sentinel-driver
//!
//! High-performance PostgreSQL wire protocol driver for Rust.
//! Foundation layer for Sentinel ORM.
//!
//! ## Features
//!
//! - PG-only — every PostgreSQL feature is first-class
//! - Single-task architecture — no channel overhead
//! - Pipeline mode — automatic query batching (PG 14+)
//! - COPY protocol — bulk insert 10-50x faster than INSERT
//! - LISTEN/NOTIFY — first-class realtime notifications
//! - SCRAM-SHA-256 with correct SASLprep
//! - Zero-copy parsing for large column values
//! - Two-tier prepared statement cache
//! - Connection pool with <0.5μs checkout
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use sentinel_driver::{Config, Connection};
//!
//! # async fn example() -> sentinel_driver::Result<()> {
//! let config = Config::parse("postgres://user:pass@localhost/mydb")?;
//! let mut conn = Connection::connect(config).await?;
//!
//! let rows = conn.query("SELECT id, name FROM users WHERE active = $1", &[&true]).await?;
//! for row in &rows {
//!     let id: i32 = row.get(0);
//!     let name: String = row.get(1);
//! }
//! # Ok(())
//! # }
//! ```

pub mod advisory_lock;
pub mod auth;
pub mod cache;
pub mod cancel;
pub mod config;
pub mod connection;
pub mod copy;
pub mod error;
pub mod notify;
pub mod observability;
pub mod pipeline;
pub mod pool;
pub mod portal;
pub mod protocol;
pub mod row;
pub mod statement;
pub mod stream;
pub mod tls;
pub mod transaction;
pub mod types;

// ── Public re-exports ────────────────────────────────

pub use advisory_lock::{PgAdvisoryLock, PgAdvisoryLockGuard};
pub use cache::{CacheMetrics, StatementCache};
pub use cancel::CancelToken;
pub use config::{ChannelBinding, Config, SslMode};
pub use connection::Connection;
pub use copy::binary::{BinaryCopyDecoder, BinaryCopyEncoder};
pub use copy::text::{TextCopyDecoder, TextCopyEncoder};
pub use error::{Error, Result};
pub use notify::Notification;
pub use observability::{ObservabilityConfig, QueryMetrics, QueryMetricsCallback};
pub use pool::{Pool, PoolMetrics, PooledConnection};
pub use portal::Portal;
pub use row::{CommandResult, Row, RowDescription};
pub use statement::Statement;
pub use stream::RowStream;
pub use transaction::{IsolationLevel, TransactionConfig};
pub use types::{FromSql, Oid, ToSql};

// Re-export derive macros when the `derive` feature is enabled
#[cfg(feature = "derive")]
pub use sentinel_derive::{FromRow, FromSql, ToSql};
