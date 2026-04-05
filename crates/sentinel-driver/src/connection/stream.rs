use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::BytesMut;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;

use crate::config::{Config, SslMode};
use crate::error::{Error, Result};
use crate::protocol::backend::BackendMessage;
use crate::protocol::codec;
use crate::protocol::frontend;
use crate::tls;

/// Unified stream that can be either plain TCP or TLS-wrapped TCP.
pub(crate) enum PgStream {
    Tcp(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
}

impl AsyncRead for PgStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            PgStream::Tcp(s) => Pin::new(s).poll_read(cx, buf),
            PgStream::Tls(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for PgStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            PgStream::Tcp(s) => Pin::new(s).poll_write(cx, buf),
            PgStream::Tls(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            PgStream::Tcp(s) => Pin::new(s).poll_flush(cx),
            PgStream::Tls(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            PgStream::Tcp(s) => Pin::new(s).poll_shutdown(cx),
            PgStream::Tls(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

/// A buffered connection to PostgreSQL, handling framing and TLS.
pub(crate) struct PgConnection {
    stream: PgStream,
    read_buf: BytesMut,
    write_buf: BytesMut,
}

impl PgConnection {
    /// Connect to PostgreSQL, performing TCP connection and optional TLS upgrade.
    pub async fn connect(config: &Config) -> Result<Self> {
        let addr = format!("{}:{}", config.host(), config.port());

        let tcp = tokio::time::timeout(config.connect_timeout(), TcpStream::connect(&addr))
            .await
            .map_err(|_| Error::Timeout(format!("connect timeout to {addr}")))?
            .map_err(Error::Io)?;

        tcp.set_nodelay(true).ok();

        let tls_config = tls::make_tls_connector(config.ssl_mode(), config.host())?;

        let stream = match tls_config {
            Some(tls_cfg) => {
                let mut tcp = tcp;
                // Send SSLRequest
                let mut ssl_buf = BytesMut::new();
                frontend::ssl_request(&mut ssl_buf);
                tcp.write_all(&ssl_buf).await.map_err(Error::Io)?;

                // Read single-byte response
                let mut response = [0u8; 1];
                tcp.read_exact(&mut response).await.map_err(Error::Io)?;

                match response[0] {
                    b'S' => {
                        // Server supports TLS — upgrade
                        let tls_stream = tls_cfg
                            .connector
                            .connect(tls_cfg.server_name, tcp)
                            .await
                            .map_err(|e| Error::Tls(format!("TLS handshake failed: {e}")))?;
                        PgStream::Tls(Box::new(tls_stream))
                    }
                    b'N' => {
                        // Server doesn't support TLS
                        match config.ssl_mode() {
                            SslMode::Prefer => PgStream::Tcp(tcp),
                            _ => {
                                return Err(Error::Tls("server does not support TLS".to_string()));
                            }
                        }
                    }
                    b => {
                        return Err(Error::protocol(format!(
                            "unexpected SSL response: 0x{b:02x}"
                        )));
                    }
                }
            }
            None => PgStream::Tcp(tcp),
        };

        Ok(Self {
            stream,
            read_buf: BytesMut::with_capacity(8192),
            write_buf: BytesMut::with_capacity(8192),
        })
    }

    /// Get a mutable reference to the write buffer for encoding messages.
    pub fn write_buf(&mut self) -> &mut BytesMut {
        &mut self.write_buf
    }

    /// Flush the write buffer to the stream.
    pub async fn flush(&mut self) -> Result<()> {
        if !self.write_buf.is_empty() {
            self.stream
                .write_all(&self.write_buf)
                .await
                .map_err(Error::Io)?;
            self.write_buf.clear();
        }
        self.stream.flush().await.map_err(Error::Io)?;
        Ok(())
    }

    /// Send the contents of the write buffer and flush.
    pub async fn send(&mut self) -> Result<()> {
        self.flush().await
    }

    /// Read the next backend message, reading more data from the stream as needed.
    pub async fn recv(&mut self) -> Result<BackendMessage> {
        loop {
            // Try to decode from existing buffer first.
            if let Some(msg) = codec::decode_message(&mut self.read_buf)? {
                return Ok(msg);
            }

            // Need more data from the stream.
            let n = self
                .stream
                .read_buf(&mut self.read_buf)
                .await
                .map_err(Error::Io)?;
            if n == 0 {
                return Err(Error::ConnectionClosed);
            }
        }
    }

    /// Send a raw buffer (for startup message which uses write_buf differently).
    pub async fn send_raw(&mut self, buf: &[u8]) -> Result<()> {
        self.stream.write_all(buf).await.map_err(Error::Io)?;
        self.stream.flush().await.map_err(Error::Io)?;
        Ok(())
    }

    /// Send a Terminate message and shut down the stream.
    pub async fn close(mut self) -> Result<()> {
        frontend::terminate(&mut self.write_buf);
        self.flush().await?;
        self.stream.shutdown().await.map_err(Error::Io)?;
        Ok(())
    }

    /// Returns `true` if the connection is using TLS.
    pub fn is_tls(&self) -> bool {
        matches!(self.stream, PgStream::Tls(_))
    }
}
