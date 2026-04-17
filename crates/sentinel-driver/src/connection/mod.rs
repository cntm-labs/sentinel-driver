pub mod startup;
pub mod stream;

mod client;
mod copy_impl;
mod notify_impl;
mod pipeline_impl;
mod portal_impl;
mod prepare;
mod query;
mod stream_impl;
mod transaction_impl;

use std::time::Duration;

use crate::cache::{CacheMetrics, StatementCache};
use crate::cancel::CancelToken;
use crate::config::Config;
use crate::copy;
use crate::error::{Error, Result};
use crate::notify::{self, Notification};
use crate::pipeline::{self, batch::PipelineBatch};
use crate::protocol::backend::{BackendMessage, TransactionStatus};
use crate::protocol::frontend;
use crate::row::{Row, RowDescription};
use crate::statement::Statement;
use crate::transaction::TransactionConfig;
use crate::types::{Oid, ToSql};

use bytes::BytesMut;
use stream::PgConnection;

/// A high-level connection to PostgreSQL.
///
/// Wraps the low-level `PgConnection` with statement caching,
/// convenient query methods, and transaction support.
pub struct Connection {
    pub(crate) conn: PgConnection,
    pub(crate) config: Config,
    pub(crate) connected_host: String,
    pub(crate) connected_port: u16,
    pub(crate) process_id: i32,
    pub(crate) secret_key: i32,
    pub(crate) transaction_status: TransactionStatus,
    pub(crate) stmt_cache: StatementCache,
    pub(crate) query_timeout: Option<Duration>,
    pub(crate) is_broken: bool,
}
