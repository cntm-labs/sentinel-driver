use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use bytes::BytesMut;

use crate::error::{Error, Result};
use crate::protocol::frontend;

/// Token for cancelling a running query from any task or thread.
///
/// Obtained via [`Connection::cancel_token()`]. Cheaply cloneable — holds
/// only the host, port, and backend key data needed to send a CancelRequest.
///
/// # Example
///
/// ```rust,no_run
/// # async fn example(conn: &mut sentinel_driver::Connection) -> sentinel_driver::Result<()> {
/// let token = conn.cancel_token();
///
/// // Spawn a task that cancels after 5 seconds
/// tokio::spawn(async move {
///     tokio::time::sleep(std::time::Duration::from_secs(5)).await;
///     token.cancel().await.ok();
/// });
///
/// // This query will be cancelled if it takes more than 5 seconds
/// let rows = conn.query("SELECT pg_sleep(60)", &[]).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct CancelToken {
    host: String,
    port: u16,
    process_id: i32,
    secret_key: i32,
}

impl CancelToken {
    /// Create a new cancel token.
    ///
    /// Typically obtained via `Connection::cancel_token()` rather than
    /// constructed directly.
    pub fn new(host: impl Into<String>, port: u16, process_id: i32, secret_key: i32) -> Self {
        Self {
            host: host.into(),
            port,
            process_id,
            secret_key,
        }
    }

    /// Send a cancel request to the PostgreSQL server.
    ///
    /// Opens a new TCP connection, sends the 16-byte CancelRequest message,
    /// and closes the connection. This is best-effort — the server may or
    /// may not cancel the running query.
    ///
    /// Always uses plain TCP (no TLS) per PostgreSQL protocol convention.
    pub async fn cancel(&self) -> Result<()> {
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = TcpStream::connect(&addr).await.map_err(Error::Io)?;

        let mut buf = BytesMut::with_capacity(16);
        frontend::cancel_request(&mut buf, self.process_id, self.secret_key);

        stream.write_all(&buf).await.map_err(Error::Io)?;
        stream.shutdown().await.map_err(Error::Io)?;

        Ok(())
    }
}
