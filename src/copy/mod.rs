pub mod binary;
pub mod text;

use crate::connection::stream::PgConnection;
use crate::error::{Error, Result};
use crate::protocol::backend::{BackendMessage, CopyFormat};
use crate::protocol::frontend;
use crate::row::parse_command_tag;

/// A COPY IN operation — streaming data to the server.
///
/// Created by sending a `COPY ... FROM STDIN` query.
/// Write rows via `write_raw()` or the format-specific helpers,
/// then call `finish()` to complete.
pub struct CopyIn<'a> {
    conn: &'a mut PgConnection,
    format: CopyFormat,
    column_count: usize,
    finished: bool,
}

impl<'a> CopyIn<'a> {
    pub(crate) fn new(conn: &'a mut PgConnection, format: CopyFormat, column_count: usize) -> Self {
        Self {
            conn,
            format,
            column_count,
            finished: false,
        }
    }

    /// The COPY format (Text or Binary).
    pub fn format(&self) -> CopyFormat {
        self.format
    }

    /// Number of columns expected per row.
    pub fn column_count(&self) -> usize {
        self.column_count
    }

    /// Write raw COPY data. The data must be in the correct format
    /// (text or binary) as negotiated with the server.
    pub async fn write_raw(&mut self, data: &[u8]) -> Result<()> {
        frontend::copy_data(self.conn.write_buf(), data);
        self.conn.send().await
    }

    /// Finish the COPY operation and return the number of rows inserted.
    pub async fn finish(mut self) -> Result<u64> {
        self.finished = true;

        frontend::copy_done(self.conn.write_buf());
        self.conn.send().await?;

        // Expect CommandComplete then ReadyForQuery
        let rows = loop {
            match self.conn.recv().await? {
                BackendMessage::CommandComplete { tag } => {
                    break parse_command_tag(&tag).rows_affected;
                }
                BackendMessage::ErrorResponse { fields } => {
                    // Drain until ReadyForQuery
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
                _ => continue,
            }
        };

        // Wait for ReadyForQuery
        drain_until_ready(self.conn).await?;

        Ok(rows)
    }

    /// Abort the COPY operation with an error message.
    pub async fn abort(mut self, message: &str) -> Result<()> {
        self.finished = true;

        frontend::copy_fail(self.conn.write_buf(), message);
        self.conn.send().await?;

        // Server will send ErrorResponse + ReadyForQuery
        drain_until_ready(self.conn).await.ok();

        Ok(())
    }
}

impl<'a> Drop for CopyIn<'a> {
    fn drop(&mut self) {
        if !self.finished {
            // Can't do async in drop — just write CopyFail to buffer.
            // The next operation on the connection will flush it.
            frontend::copy_fail(
                self.conn.write_buf(),
                "COPY IN aborted: dropped without finish",
            );
        }
    }
}

/// A COPY OUT operation — streaming data from the server.
///
/// Created by sending a `COPY ... TO STDOUT` query.
/// Read rows via `read_raw()` until it returns `None`.
pub struct CopyOut<'a> {
    conn: &'a mut PgConnection,
    format: CopyFormat,
    done: bool,
}

impl<'a> CopyOut<'a> {
    pub(crate) fn new(conn: &'a mut PgConnection, format: CopyFormat) -> Self {
        Self {
            conn,
            format,
            done: false,
        }
    }

    /// The COPY format (Text or Binary).
    pub fn format(&self) -> CopyFormat {
        self.format
    }

    /// Read the next chunk of COPY data.
    ///
    /// Returns `None` when the COPY operation is complete.
    pub async fn read_raw(&mut self) -> Result<Option<bytes::Bytes>> {
        if self.done {
            return Ok(None);
        }

        loop {
            match self.conn.recv().await? {
                BackendMessage::CopyData { data } => {
                    return Ok(Some(data));
                }
                BackendMessage::CopyDone => {
                    self.done = true;
                    // Expect CommandComplete + ReadyForQuery
                    drain_until_ready(self.conn).await?;
                    return Ok(None);
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
                _ => continue,
            }
        }
    }
}

/// Start a COPY IN operation by sending the COPY query.
pub(crate) async fn start_copy_in(
    conn: &mut PgConnection,
    sql: &str,
) -> Result<(CopyFormat, usize)> {
    frontend::query(conn.write_buf(), sql);
    conn.send().await?;

    loop {
        match conn.recv().await? {
            BackendMessage::CopyInResponse {
                format,
                column_formats,
            } => {
                return Ok((format, column_formats.len()));
            }
            BackendMessage::ErrorResponse { fields } => {
                drain_until_ready(conn).await.ok();
                return Err(Error::server(
                    fields.severity,
                    fields.code,
                    fields.message,
                    fields.detail,
                    fields.hint,
                    fields.position,
                ));
            }
            _ => continue,
        }
    }
}

/// Start a COPY OUT operation by sending the COPY query.
pub(crate) async fn start_copy_out(conn: &mut PgConnection, sql: &str) -> Result<CopyFormat> {
    frontend::query(conn.write_buf(), sql);
    conn.send().await?;

    loop {
        match conn.recv().await? {
            BackendMessage::CopyOutResponse { format, .. } => {
                return Ok(format);
            }
            BackendMessage::ErrorResponse { fields } => {
                drain_until_ready(conn).await.ok();
                return Err(Error::server(
                    fields.severity,
                    fields.code,
                    fields.message,
                    fields.detail,
                    fields.hint,
                    fields.position,
                ));
            }
            _ => continue,
        }
    }
}

/// Drain messages until ReadyForQuery.
async fn drain_until_ready(conn: &mut PgConnection) -> Result<()> {
    loop {
        match conn.recv().await? {
            BackendMessage::ReadyForQuery { .. } => return Ok(()),
            _ => continue,
        }
    }
}
