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

pub mod auth;
pub mod cache;
pub mod cancel;
pub mod config;
pub mod connection;
pub mod copy;
pub mod error;
pub mod notify;
pub mod pipeline;
pub mod pool;
pub mod protocol;
pub mod row;
pub mod statement;
pub mod tls;
pub mod transaction;
pub mod types;

// ── Public re-exports ────────────────────────────────

pub use cache::{CacheMetrics, StatementCache};
pub use cancel::CancelToken;
pub use config::{Config, SslMode};
pub use copy::binary::{BinaryCopyDecoder, BinaryCopyEncoder};
pub use copy::text::{TextCopyDecoder, TextCopyEncoder};
pub use error::{Error, Result};
pub use notify::Notification;
pub use pool::Pool;
pub use row::{CommandResult, Row, RowDescription};
pub use statement::Statement;
pub use transaction::{IsolationLevel, TransactionConfig};
pub use types::{FromSql, Oid, ToSql};

// Re-export derive macros when the `derive` feature is enabled
#[cfg(feature = "derive")]
pub use sentinel_derive::{FromRow, FromSql, ToSql};

use bytes::BytesMut;

use crate::connection::startup::{self};
use crate::connection::stream::PgConnection;
use crate::pipeline::batch::PipelineBatch;
use crate::protocol::backend::{BackendMessage, TransactionStatus};
use crate::protocol::frontend;

/// A high-level connection to PostgreSQL.
///
/// Wraps the low-level `PgConnection` with statement caching,
/// convenient query methods, and transaction support.
pub struct Connection {
    conn: PgConnection,
    _config: Config,
    process_id: i32,
    _secret_key: i32,
    transaction_status: TransactionStatus,
    stmt_cache: StatementCache,
}

impl Connection {
    /// Connect to PostgreSQL and perform the startup handshake.
    pub async fn connect(config: Config) -> Result<Self> {
        let mut conn = PgConnection::connect(&config).await?;
        let result = startup::startup(&mut conn, &config).await?;

        Ok(Self {
            conn,
            _config: config,
            process_id: result.process_id,
            _secret_key: result.secret_key,
            transaction_status: result.transaction_status,
            stmt_cache: StatementCache::new(),
        })
    }

    /// Execute a query that returns rows.
    ///
    /// Parameters are encoded in binary format.
    ///
    /// ```rust,no_run
    /// # async fn example(conn: &mut sentinel_driver::Connection) -> sentinel_driver::Result<()> {
    /// let rows = conn.query("SELECT * FROM users WHERE id = $1", &[&42i32]).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        let result = self.query_internal(sql, params).await?;
        match result {
            pipeline::QueryResult::Rows(rows) => Ok(rows),
            pipeline::QueryResult::Command(_) => Ok(Vec::new()),
        }
    }

    /// Execute a query that returns a single row.
    ///
    /// Returns an error if no rows are returned.
    pub async fn query_one(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        let rows = self.query(sql, params).await?;
        rows.into_iter()
            .next()
            .ok_or_else(|| Error::Protocol("query returned no rows".into()))
    }

    /// Execute a query that returns an optional single row.
    pub async fn query_opt(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>> {
        let rows = self.query(sql, params).await?;
        Ok(rows.into_iter().next())
    }

    /// Execute a non-SELECT query (INSERT, UPDATE, DELETE, etc.).
    ///
    /// Returns the number of rows affected.
    pub async fn execute(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        let result = self.query_internal(sql, params).await?;
        match result {
            pipeline::QueryResult::Command(r) => Ok(r.rows_affected),
            pipeline::QueryResult::Rows(_) => Ok(0),
        }
    }

    /// Execute a simple query (no parameters, text protocol).
    ///
    /// Useful for DDL statements and multi-statement queries.
    pub async fn simple_query(&mut self, sql: &str) -> Result<Vec<CommandResult>> {
        frontend::query(self.conn.write_buf(), sql);
        self.conn.send().await?;

        let mut results = Vec::new();

        loop {
            match self.conn.recv().await? {
                BackendMessage::CommandComplete { tag } => {
                    results.push(row::parse_command_tag(&tag));
                }
                BackendMessage::ReadyForQuery { transaction_status } => {
                    self.transaction_status = transaction_status;
                    break;
                }
                BackendMessage::ErrorResponse { fields } => {
                    // Drain until ReadyForQuery
                    self.drain_until_ready().await.ok();
                    return Err(Error::server(
                        fields.severity,
                        fields.code,
                        fields.message,
                        fields.detail,
                        fields.hint,
                        fields.position,
                    ));
                }
                _ => {}
            }
        }

        Ok(results)
    }

    /// Create a pipeline batch for executing multiple queries in a single round-trip.
    ///
    /// Use `execute_pipeline()` to send the batch.
    pub fn pipeline(&self) -> PipelineBatch {
        PipelineBatch::new()
    }

    /// Execute a pipeline batch, returning results for each query.
    pub async fn execute_pipeline(
        &mut self,
        batch: PipelineBatch,
    ) -> Result<Vec<pipeline::QueryResult>> {
        batch.execute(&mut self.conn).await
    }

    /// Begin a transaction with default settings.
    pub async fn begin(&mut self) -> Result<()> {
        self.begin_with(TransactionConfig::new()).await
    }

    /// Begin a transaction with custom settings.
    pub async fn begin_with(&mut self, config: TransactionConfig) -> Result<()> {
        self.simple_query(&config.begin_sql()).await?;
        Ok(())
    }

    /// Commit the current transaction.
    pub async fn commit(&mut self) -> Result<()> {
        self.simple_query("COMMIT").await?;
        Ok(())
    }

    /// Rollback the current transaction.
    pub async fn rollback(&mut self) -> Result<()> {
        self.simple_query("ROLLBACK").await?;
        Ok(())
    }

    /// Create a savepoint.
    pub async fn savepoint(&mut self, name: &str) -> Result<()> {
        self.simple_query(&format!("SAVEPOINT {}", notify::quote_identifier(name)))
            .await?;
        Ok(())
    }

    /// Rollback to a savepoint.
    pub async fn rollback_to(&mut self, name: &str) -> Result<()> {
        self.simple_query(&format!(
            "ROLLBACK TO SAVEPOINT {}",
            notify::quote_identifier(name)
        ))
        .await?;
        Ok(())
    }

    /// Start a COPY IN operation for bulk data loading.
    pub async fn copy_in(&mut self, sql: &str) -> Result<copy::CopyIn<'_>> {
        let (format, col_count) = copy::start_copy_in(&mut self.conn, sql).await?;
        Ok(copy::CopyIn::new(&mut self.conn, format, col_count))
    }

    /// Start a COPY OUT operation for bulk data export.
    pub async fn copy_out(&mut self, sql: &str) -> Result<copy::CopyOut<'_>> {
        let format = copy::start_copy_out(&mut self.conn, sql).await?;
        Ok(copy::CopyOut::new(&mut self.conn, format))
    }

    /// Subscribe to LISTEN/NOTIFY on a channel.
    pub async fn listen(&mut self, channel: &str) -> Result<()> {
        notify::listen(&mut self.conn, channel).await
    }

    /// Unsubscribe from a channel.
    pub async fn unlisten(&mut self, channel: &str) -> Result<()> {
        notify::unlisten(&mut self.conn, channel).await
    }

    /// Unsubscribe from all channels.
    pub async fn unlisten_all(&mut self) -> Result<()> {
        notify::unlisten_all(&mut self.conn).await
    }

    /// Send a notification on a channel.
    pub async fn notify(&mut self, channel: &str, payload: &str) -> Result<()> {
        notify::notify(&mut self.conn, channel, payload).await
    }

    /// Prepare a statement on the server using extended query protocol.
    ///
    /// Returns a `Statement` with parameter types and column descriptions.
    pub async fn prepare(&mut self, sql: &str) -> Result<Statement> {
        let stmt_name = format!("_sentinel_p{}", self.process_id);

        frontend::parse(self.conn.write_buf(), &stmt_name, sql, &[]);
        frontend::describe_statement(self.conn.write_buf(), &stmt_name);
        frontend::sync(self.conn.write_buf());
        self.conn.send().await?;

        // ParseComplete
        match self.conn.recv().await? {
            BackendMessage::ParseComplete => {}
            BackendMessage::ErrorResponse { fields } => {
                self.drain_until_ready().await.ok();
                return Err(Error::server(
                    fields.severity,
                    fields.code,
                    fields.message,
                    fields.detail,
                    fields.hint,
                    fields.position,
                ));
            }
            other => {
                return Err(Error::protocol(format!(
                    "expected ParseComplete, got {other:?}"
                )))
            }
        }

        // ParameterDescription
        let param_oids = match self.conn.recv().await? {
            BackendMessage::ParameterDescription { oids } => {
                oids.into_iter().map(Oid::from).collect()
            }
            other => {
                return Err(Error::protocol(format!(
                    "expected ParameterDescription, got {other:?}"
                )))
            }
        };

        // RowDescription or NoData
        let columns = match self.conn.recv().await? {
            BackendMessage::RowDescription { fields } => Some(fields),
            BackendMessage::NoData => None,
            other => {
                return Err(Error::protocol(format!(
                    "expected RowDescription/NoData, got {other:?}"
                )))
            }
        };

        // ReadyForQuery
        self.drain_until_ready().await?;

        Ok(Statement::new(
            stmt_name,
            sql.to_string(),
            param_oids,
            columns,
        ))
    }

    /// Register a prepared statement in the Tier 1 cache.
    pub fn register_statement(&mut self, name: &str, statement: Statement) {
        self.stmt_cache.register(name, statement);
    }

    /// Get statement cache metrics.
    pub fn cache_metrics(&self) -> &CacheMetrics {
        self.stmt_cache.metrics()
    }

    /// Returns `true` if the connection is using TLS.
    pub fn is_tls(&self) -> bool {
        self.conn.is_tls()
    }

    /// The server process ID for this connection.
    pub fn process_id(&self) -> i32 {
        self.process_id
    }

    /// Current transaction status.
    pub fn transaction_status(&self) -> TransactionStatus {
        self.transaction_status
    }

    /// Wait for the next LISTEN/NOTIFY notification.
    ///
    /// Blocks until a notification arrives on any subscribed channel.
    pub async fn wait_for_notification(&mut self) -> Result<Notification> {
        notify::wait_for_notification(&mut self.conn).await
    }

    /// Close the connection gracefully.
    pub async fn close(self) -> Result<()> {
        self.conn.close().await
    }

    // ── Internal ─────────────────────────────────────

    async fn query_internal(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<pipeline::QueryResult> {
        // Encode parameters
        let param_types: Vec<u32> = params.iter().map(|p| p.oid().0).collect();
        let mut encoded_params: Vec<Option<Vec<u8>>> = Vec::with_capacity(params.len());

        for param in params {
            let mut buf = BytesMut::new();
            param.to_sql(&mut buf)?;
            encoded_params.push(Some(buf.to_vec()));
        }

        // Use pipeline for single query (same protocol, consistent code path)
        let mut batch = PipelineBatch::new();
        batch.add(sql.to_string(), param_types, encoded_params);

        let mut results = batch.execute(&mut self.conn).await?;

        results
            .pop()
            .ok_or_else(|| Error::protocol("pipeline returned no results"))
    }

    async fn drain_until_ready(&mut self) -> Result<()> {
        loop {
            if let BackendMessage::ReadyForQuery { transaction_status } = self.conn.recv().await? {
                self.transaction_status = transaction_status;
                return Ok(());
            }
        }
    }
}

// Make quote_identifier accessible for savepoint
