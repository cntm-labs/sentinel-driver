use std::sync::Arc;

use crate::connection::stream::PgConnection;
use crate::error::{Error, Result};
use crate::protocol::backend::BackendMessage;
use crate::row::{Row, RowDescription};

/// A streaming row-by-row iterator over query results.
///
/// Created by [`Connection::query_stream()`]. Yields rows one at a time
/// via [`next()`](RowStream::next), avoiding full materialization of
/// large result sets in memory.
///
/// The stream holds an exclusive borrow of the connection — no other
/// queries can run until the stream is dropped or fully consumed.
///
/// # Example
///
/// ```rust,no_run
/// # async fn example(conn: &mut sentinel_driver::Connection) -> sentinel_driver::Result<()> {
/// let mut stream = conn.query_stream("SELECT * FROM users", &[]).await?;
/// while let Some(row) = stream.next().await? {
///     let name: String = row.get(0);
/// }
/// # Ok(())
/// # }
/// ```
pub struct RowStream<'a> {
    conn: &'a mut PgConnection,
    description: Arc<RowDescription>,
    done: bool,
}

impl<'a> RowStream<'a> {
    pub(crate) fn new(conn: &'a mut PgConnection, description: Arc<RowDescription>) -> Self {
        Self {
            conn,
            description,
            done: false,
        }
    }

    /// Fetch the next row from the stream.
    ///
    /// Returns `Ok(Some(row))` for each row, `Ok(None)` when the query
    /// is complete, or `Err` on server/protocol error.
    pub async fn next(&mut self) -> Result<Option<Row>> {
        if self.done {
            return Ok(None);
        }

        match self.conn.recv().await? {
            BackendMessage::DataRow { columns } => {
                Ok(Some(Row::new(columns, Arc::clone(&self.description))))
            }
            BackendMessage::CommandComplete { .. } => {
                self.done = true;
                // Read ReadyForQuery to leave connection in clean state
                drain_until_ready(self.conn).await?;
                Ok(None)
            }
            BackendMessage::EmptyQueryResponse => {
                self.done = true;
                drain_until_ready(self.conn).await?;
                Ok(None)
            }
            BackendMessage::ErrorResponse { fields } => {
                self.done = true;
                drain_until_ready(self.conn).await.ok();
                Err(Error::server(
                    fields.severity,
                    fields.code,
                    fields.message,
                    fields.detail,
                    fields.hint,
                    fields.position,
                ))
            }
            other => {
                self.done = true;
                Err(Error::protocol(format!(
                    "unexpected message in row stream: {other:?}"
                )))
            }
        }
    }

    /// Close the stream early, draining any remaining server messages.
    ///
    /// Call this instead of dropping when you want to reuse the connection
    /// for subsequent queries after only partially consuming the stream.
    pub async fn close(mut self) -> Result<()> {
        if self.done {
            return Ok(());
        }

        // Drain remaining DataRows, CommandComplete, and ReadyForQuery
        loop {
            match self.conn.recv().await? {
                BackendMessage::CommandComplete { .. } => {
                    self.done = true;
                    drain_until_ready(self.conn).await?;
                    return Ok(());
                }
                BackendMessage::ErrorResponse { fields } => {
                    self.done = true;
                    drain_until_ready(self.conn).await.ok();
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
    }

    /// The row description for this stream's columns.
    pub fn description(&self) -> &RowDescription {
        &self.description
    }

    /// Returns `true` if the stream has been fully consumed or closed.
    pub fn is_done(&self) -> bool {
        self.done
    }
}

/// Drain messages until ReadyForQuery.
async fn drain_until_ready(conn: &mut PgConnection) -> Result<()> {
    loop {
        if let BackendMessage::ReadyForQuery { .. } = conn.recv().await? {
            return Ok(());
        }
    }
}
