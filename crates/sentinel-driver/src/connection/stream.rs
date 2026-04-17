use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::BytesMut;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;

#[cfg(unix)]
use tokio::net::UnixStream;

use crate::config::{Config, SslMode};
use crate::error::{Error, Result};
use crate::protocol::backend::BackendMessage;
use crate::protocol::codec;
use crate::protocol::frontend;
use crate::tls;

/// Unified stream that can be plain TCP, TLS-wrapped TCP, or Unix domain socket.
pub(crate) enum PgStream {
    Tcp(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
    #[cfg(unix)]
    Unix(UnixStream),
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
            #[cfg(unix)]
            PgStream::Unix(s) => Pin::new(s).poll_read(cx, buf),
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
            #[cfg(unix)]
            PgStream::Unix(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            PgStream::Tcp(s) => Pin::new(s).poll_flush(cx),
            PgStream::Tls(s) => Pin::new(s).poll_flush(cx),
            #[cfg(unix)]
            PgStream::Unix(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            PgStream::Tcp(s) => Pin::new(s).poll_shutdown(cx),
            PgStream::Tls(s) => Pin::new(s).poll_shutdown(cx),
            #[cfg(unix)]
            PgStream::Unix(s) => Pin::new(s).poll_shutdown(cx),
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
    /// Connect to a single host, performing TCP (or Unix) connection and optional TLS upgrade.
    pub async fn connect_host(config: &Config, host: &str, port: u16) -> Result<Self> {
        let stream = if is_unix_socket(host) {
            Self::connect_unix(config, host, port).await?
        } else {
            Self::connect_tcp(config, host, port).await?
        };

        Ok(Self {
            stream,
            read_buf: BytesMut::with_capacity(8192),
            write_buf: BytesMut::with_capacity(8192),
        })
    }

    /// Connect via TCP with optional TLS upgrade.
    async fn connect_tcp(config: &Config, host: &str, port: u16) -> Result<PgStream> {
        let addr = format!("{host}:{port}");

        let tcp = tokio::time::timeout(config.connect_timeout(), TcpStream::connect(&addr))
            .await
            .map_err(|_| Error::Timeout(format!("connect timeout to {addr}")))?
            .map_err(Error::Io)?;

        tcp.set_nodelay(true).ok();

        let tls_config = tls::make_tls_connector(config)?;

        let stream = match tls_config {
            Some(tls_cfg) => {
                if config.ssl_direct() {
                    // Direct TLS (PG 17+): handshake immediately, skip SSLRequest
                    let tls_stream = tls_cfg
                        .connector
                        .connect(tls_cfg.server_name, tcp)
                        .await
                        .map_err(|e| Error::Tls(format!("TLS handshake failed: {e}")))?;
                    PgStream::Tls(Box::new(tls_stream))
                } else {
                    // Standard SSLRequest negotiation
                    let mut tcp = tcp;
                    let mut ssl_buf = BytesMut::new();
                    frontend::ssl_request(&mut ssl_buf);
                    tcp.write_all(&ssl_buf).await.map_err(Error::Io)?;

                    let mut response = [0u8; 1];
                    tcp.read_exact(&mut response).await.map_err(Error::Io)?;

                    match response[0] {
                        b'S' => {
                            let tls_stream = tls_cfg
                                .connector
                                .connect(tls_cfg.server_name, tcp)
                                .await
                                .map_err(|e| Error::Tls(format!("TLS handshake failed: {e}")))?;
                            PgStream::Tls(Box::new(tls_stream))
                        }
                        b'N' => match config.ssl_mode() {
                            SslMode::Prefer => PgStream::Tcp(tcp),
                            _ => {
                                return Err(Error::Tls("server does not support TLS".to_string()));
                            }
                        },
                        b => {
                            return Err(Error::protocol(format!(
                                "unexpected SSL response: 0x{b:02x}"
                            )));
                        }
                    }
                }
            }
            None => PgStream::Tcp(tcp),
        };

        Ok(stream)
    }

    /// Connect via Unix domain socket.
    #[cfg(unix)]
    async fn connect_unix(config: &Config, host: &str, port: u16) -> Result<PgStream> {
        let socket_path = format!("{host}/.s.PGSQL.{port}");

        let unix =
            tokio::time::timeout(config.connect_timeout(), UnixStream::connect(&socket_path))
                .await
                .map_err(|_| Error::Timeout(format!("connect timeout to {socket_path}")))?
                .map_err(Error::Io)?;

        Ok(PgStream::Unix(unix))
    }

    /// Unix socket connect is not available on non-Unix platforms.
    #[cfg(not(unix))]
    async fn connect_unix(_config: &Config, host: &str, _port: u16) -> Result<PgStream> {
        Err(Error::Config(format!(
            "Unix domain sockets are not supported on this platform: {host}"
        )))
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

    /// Returns `true` if connected via Unix domain socket.
    #[cfg(unix)]
    pub fn is_unix(&self) -> bool {
        matches!(self.stream, PgStream::Unix(_))
    }

    /// Extract the DER-encoded server certificate from the TLS session.
    ///
    /// Returns `None` if not using TLS or no peer certificates available.
    pub fn server_certificate_der(&self) -> Option<Vec<u8>> {
        match &self.stream {
            PgStream::Tls(tls_stream) => {
                let (_, conn) = tls_stream.get_ref();
                conn.peer_certificates()
                    .and_then(|certs| certs.first())
                    .map(|cert| cert.as_ref().to_vec())
            }
            PgStream::Tcp(_) => None,
            #[cfg(unix)]
            PgStream::Unix(_) => None,
        }
    }
}

/// Returns `true` if the host string represents a Unix domain socket path.
pub(crate) fn is_unix_socket(host: &str) -> bool {
    host.starts_with('/')
}
