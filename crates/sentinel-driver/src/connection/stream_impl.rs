use std::sync::Arc;

use super::{frontend, BackendMessage, BytesMut, Connection, Error, Result, RowDescription, ToSql};
use crate::stream::RowStream;

impl Connection {
    /// Execute a streaming query that returns rows one at a time.
    ///
    /// Unlike [`query()`](Connection::query) which materializes all rows
    /// in memory, this returns a [`RowStream`] that yields rows lazily.
    /// Ideal for large result sets.
    ///
    /// The stream holds an exclusive borrow of the connection — no other
    /// queries can run until the stream is dropped or fully consumed.
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
    pub async fn query_stream(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<RowStream<'_>> {
        // Encode parameters
        let param_types: Vec<u32> = params.iter().map(|p| p.oid().0).collect();
        let mut encoded_params: Vec<Option<&[u8]>> = Vec::with_capacity(params.len());
        let mut param_bufs: Vec<BytesMut> = Vec::with_capacity(params.len());

        for param in params {
            let mut buf = BytesMut::new();
            param.to_sql(&mut buf)?;
            param_bufs.push(buf);
        }
        for buf in &param_bufs {
            encoded_params.push(Some(buf.as_ref()));
        }

        // Send Parse + Bind + Describe + Execute + Sync
        frontend::parse(self.conn.write_buf(), "", sql, &param_types);
        frontend::bind(self.conn.write_buf(), "", "", &encoded_params, &[]);
        frontend::describe_portal(self.conn.write_buf(), "");
        frontend::execute(self.conn.write_buf(), "", 0);
        frontend::sync(self.conn.write_buf());
        self.conn.send().await?;

        // Read ParseComplete
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
                )));
            }
        }

        // Read BindComplete
        match self.conn.recv().await? {
            BackendMessage::BindComplete => {}
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
                    "expected BindComplete, got {other:?}"
                )));
            }
        }

        // Read RowDescription (required for streaming — NoData means no rows to stream)
        let description = match self.conn.recv().await? {
            BackendMessage::RowDescription { fields } => Arc::new(RowDescription::new(fields)),
            BackendMessage::NoData => {
                // Non-SELECT query — drain remaining and return error
                self.drain_until_ready().await.ok();
                return Err(Error::protocol(
                    "query_stream requires a query that returns rows".to_string(),
                ));
            }
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
                    "expected RowDescription, got {other:?}"
                )));
            }
        };

        Ok(RowStream::new(&mut self.conn, description))
    }
}
