use std::fmt;

/// Result type alias for sentinel-driver operations.
pub type Result<T> = std::result::Result<T, Error>;

/// All possible errors returned by `sentinel-driver`.
///
/// # Stability contract
///
/// This enum is `#[non_exhaustive]` as of v1.1.0. New variants may be
/// added in any future minor release without a major version bump, in
/// line with the additive-only stability policy in `GOVERNANCE.md`.
///
/// What this means for downstream code:
///
/// - `?` propagation and `From`/`Into` conversion are unaffected.
/// - Manual exhaustive `match` arms must include a wildcard `_ =>` arm
///   to keep compiling against future minor releases. This is the only
///   migration required by the v1.0 → v1.1 transition.
///
/// ```ignore
/// // OK — has a wildcard arm.
/// match err {
///     Error::ConnectionClosed => /* ... */,
///     Error::TransactionCompleted => /* ... */,
///     _ => /* ... */,
/// }
/// ```
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// I/O error from TCP/TLS stream.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// PostgreSQL protocol error (unexpected message, malformed packet, etc.).
    #[error("protocol error: {0}")]
    Protocol(String),

    /// Error returned by the PostgreSQL server.
    #[error("{0}")]
    Server(Box<ServerError>),

    /// Authentication failure.
    #[error("authentication failed: {0}")]
    Auth(String),

    /// TLS/SSL negotiation error.
    #[error("tls error: {0}")]
    Tls(String),

    /// Connection pool error.
    #[error("pool error: {0}")]
    Pool(String),

    /// Invalid configuration.
    #[error("config error: {0}")]
    Config(String),

    /// Type encoding error (Rust → PG).
    #[error("encode error: {0}")]
    Encode(String),

    /// Type decoding error (PG → Rust).
    #[error("decode error: {0}")]
    Decode(String),

    /// Column not found by name.
    #[error("column not found: {0}")]
    ColumnNotFound(String),

    /// Column index out of bounds.
    #[error("column index {index} out of bounds (row has {count} columns)")]
    ColumnIndex { index: usize, count: usize },

    /// Unexpected null value.
    #[error("unexpected null in column {0}")]
    UnexpectedNull(usize),

    /// Timeout (connect, query, pool checkout).
    #[error("timeout: {0}")]
    Timeout(String),

    /// Connection is closed or broken.
    #[error("connection closed")]
    ConnectionClosed,

    /// COPY protocol error.
    #[error("copy error: {0}")]
    Copy(String),

    /// Transaction already completed (committed or rolled back).
    #[error("transaction already completed")]
    TransactionCompleted,

    /// All configured hosts failed to connect.
    #[error("all hosts failed: {0}")]
    AllHostsFailed(String),

    /// Connected server does not match required session attributes.
    #[error("wrong session attributes: {0}")]
    WrongSessionAttrs(String),
}

/// PostgreSQL server error details.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ServerError {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub detail: Option<String>,
    pub hint: Option<String>,
    pub position: Option<u32>,
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} (SQLSTATE {})",
            self.severity, self.message, self.code
        )
    }
}

impl Error {
    /// Returns the SQLSTATE code if this is a server error.
    pub fn code(&self) -> Option<&str> {
        match self {
            Error::Server(e) => Some(&e.code),
            _ => None,
        }
    }

    /// Returns the server error details if this is a server error.
    pub fn server_error(&self) -> Option<&ServerError> {
        match self {
            Error::Server(e) => Some(e),
            _ => None,
        }
    }

    /// Returns `true` if this error represents a unique violation (SQLSTATE 23505).
    pub fn is_unique_violation(&self) -> bool {
        self.code() == Some("23505")
    }

    /// Returns `true` if this error represents a foreign key violation (SQLSTATE 23503).
    pub fn is_foreign_key_violation(&self) -> bool {
        self.code() == Some("23503")
    }

    /// Returns `true` if the connection should be considered broken.
    pub fn is_fatal(&self) -> bool {
        matches!(self, Error::Io(_) | Error::ConnectionClosed | Error::Tls(_))
    }
}

impl Error {
    /// Create a protocol error from a string.
    pub(crate) fn protocol(msg: impl Into<String>) -> Self {
        Error::Protocol(msg.into())
    }

    /// Create a server error from ErrorResponse fields.
    pub(crate) fn server(
        severity: String,
        code: String,
        message: String,
        detail: Option<String>,
        hint: Option<String>,
        position: Option<u32>,
    ) -> Self {
        Error::Server(Box::new(ServerError {
            severity,
            code,
            message,
            detail,
            hint,
            position,
        }))
    }
}

/// Severity level from PostgreSQL ErrorResponse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Severity {
    Error,
    Fatal,
    Panic,
    Warning,
    Notice,
    Debug,
    Info,
    Log,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "ERROR"),
            Severity::Fatal => write!(f, "FATAL"),
            Severity::Panic => write!(f, "PANIC"),
            Severity::Warning => write!(f, "WARNING"),
            Severity::Notice => write!(f, "NOTICE"),
            Severity::Debug => write!(f, "DEBUG"),
            Severity::Info => write!(f, "INFO"),
            Severity::Log => write!(f, "LOG"),
        }
    }
}
