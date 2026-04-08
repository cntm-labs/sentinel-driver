use std::time::Duration;

use crate::error::{Error, Result};

/// TLS mode for the connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SslMode {
    /// No TLS. Connections are unencrypted.
    Disable,
    /// Try TLS, fall back to plaintext if server doesn't support it.
    #[default]
    Prefer,
    /// Require TLS. Fail if server doesn't support it.
    Require,
    /// Require TLS and verify the server certificate.
    VerifyCa,
    /// Require TLS, verify certificate, and verify hostname matches.
    VerifyFull,
}

/// Connection configuration for sentinel-driver.
///
/// # Connection String
///
/// ```text
/// postgres://user:password@host:port/database?sslmode=prefer&application_name=myapp
/// ```
///
/// # Builder
///
/// ```rust,no_run
/// use sentinel_driver::Config;
///
/// let config = Config::builder()
///     .host("localhost")
///     .port(5432)
///     .database("mydb")
///     .user("postgres")
///     .password("secret")
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct Config {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) database: String,
    pub(crate) user: String,
    pub(crate) password: Option<String>,
    pub(crate) ssl_mode: SslMode,
    pub(crate) application_name: Option<String>,
    pub(crate) connect_timeout: Duration,
    pub(crate) statement_timeout: Option<Duration>,
    pub(crate) _keepalive: Option<Duration>,
    pub(crate) _keepalive_idle: Option<Duration>,
    pub(crate) _target_session_attrs: TargetSessionAttrs,
    pub(crate) _extra_float_digits: Option<i32>,
    /// Path to client certificate file for certificate authentication.
    pub(crate) ssl_client_cert: Option<std::path::PathBuf>,
    /// Path to client private key file for certificate authentication.
    pub(crate) ssl_client_key: Option<std::path::PathBuf>,
    /// Use direct TLS connection (PG 17+) — skip SSLRequest negotiation.
    pub(crate) ssl_direct: bool,
    /// Enable SCRAM-SHA-256 channel binding (SCRAM-PLUS) when TLS is active.
    pub(crate) channel_binding: ChannelBinding,
}

/// Channel binding preference for SCRAM authentication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChannelBinding {
    /// Use channel binding if available (default).
    #[default]
    Prefer,
    /// Require channel binding — fail if server doesn't support it.
    Require,
    /// Disable channel binding.
    Disable,
}

/// Target session attributes for connection validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TargetSessionAttrs {
    /// Any server is acceptable.
    #[default]
    Any,
    /// Only accept read-write servers (primary).
    ReadWrite,
    /// Only accept read-only servers (replica).
    ReadOnly,
}

impl Config {
    /// Parse a PostgreSQL connection string.
    ///
    /// Supported formats:
    /// - `postgres://user:password@host:port/database?param=value`
    /// - `postgresql://user:password@host:port/database?param=value`
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();

        let without_scheme = s
            .strip_prefix("postgres://")
            .or_else(|| s.strip_prefix("postgresql://"))
            .ok_or_else(|| {
                Error::Config(
                    "connection string must start with postgres:// or postgresql://".into(),
                )
            })?;

        let (userinfo, rest) = match without_scheme.split_once('@') {
            Some((ui, rest)) => (Some(ui), rest),
            None => (None, without_scheme),
        };

        let (user, password) = match userinfo {
            Some(ui) => match ui.split_once(':') {
                Some((u, p)) => (percent_decode(u)?, Some(percent_decode(p)?)),
                None => (percent_decode(ui)?, None),
            },
            None => (String::new(), None),
        };

        // Split host:port from database?params
        let (hostport, db_and_params) = match rest.split_once('/') {
            Some((hp, rest)) => (hp, Some(rest)),
            None => (rest, None),
        };

        let (host, port) = match hostport.rsplit_once(':') {
            Some((h, p)) => {
                let port: u16 = p
                    .parse()
                    .map_err(|_| Error::Config(format!("invalid port: {p}")))?;
                (h.to_string(), port)
            }
            None => (hostport.to_string(), 5432),
        };

        let (database, params_str) = match db_and_params {
            Some(dp) => match dp.split_once('?') {
                Some((db, params)) => (percent_decode(db)?, Some(params.to_string())),
                None => (percent_decode(dp)?, None),
            },
            None => (String::new(), None),
        };

        let mut config = ConfigBuilder::new()
            .host(host)
            .port(port)
            .database(database)
            .user(user);

        if let Some(pw) = password {
            config = config.password(pw);
        }

        // Parse query parameters
        if let Some(params) = params_str {
            for param in params.split('&') {
                let (key, value) = param
                    .split_once('=')
                    .ok_or_else(|| Error::Config(format!("invalid parameter: {param}")))?;
                let value = percent_decode(value)?;

                match key {
                    "sslmode" => {
                        config = config.ssl_mode(match value.as_str() {
                            "disable" => SslMode::Disable,
                            "prefer" => SslMode::Prefer,
                            "require" => SslMode::Require,
                            "verify-ca" => SslMode::VerifyCa,
                            "verify-full" => SslMode::VerifyFull,
                            _ => return Err(Error::Config(format!("invalid sslmode: {value}"))),
                        });
                    }
                    "application_name" => {
                        config = config.application_name(value);
                    }
                    "connect_timeout" => {
                        let secs: u64 = value.parse().map_err(|_| {
                            Error::Config(format!("invalid connect_timeout: {value}"))
                        })?;
                        config = config.connect_timeout(Duration::from_secs(secs));
                    }
                    "statement_timeout" => {
                        let secs: u64 = value.parse().map_err(|_| {
                            Error::Config(format!("invalid statement_timeout: {value}"))
                        })?;
                        config = config.statement_timeout(Duration::from_secs(secs));
                    }
                    "target_session_attrs" => {
                        config = config.target_session_attrs(match value.as_str() {
                            "any" => TargetSessionAttrs::Any,
                            "read-write" => TargetSessionAttrs::ReadWrite,
                            "read-only" => TargetSessionAttrs::ReadOnly,
                            _ => {
                                return Err(Error::Config(format!(
                                    "invalid target_session_attrs: {value}"
                                )))
                            }
                        });
                    }
                    _ => {
                        // Ignore unknown parameters for forward compatibility
                    }
                }
            }
        }

        Ok(config.build())
    }

    /// Create a new builder for `Config`.
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    // Accessor methods

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn database(&self) -> &str {
        &self.database
    }

    pub fn user(&self) -> &str {
        &self.user
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    pub fn ssl_mode(&self) -> SslMode {
        self.ssl_mode
    }

    pub fn application_name(&self) -> Option<&str> {
        self.application_name.as_deref()
    }

    pub fn connect_timeout(&self) -> Duration {
        self.connect_timeout
    }

    pub fn statement_timeout(&self) -> Option<Duration> {
        self.statement_timeout
    }

    /// Path to client certificate for certificate authentication.
    pub fn ssl_client_cert(&self) -> Option<&std::path::Path> {
        self.ssl_client_cert.as_deref()
    }

    /// Path to client private key for certificate authentication.
    pub fn ssl_client_key(&self) -> Option<&std::path::Path> {
        self.ssl_client_key.as_deref()
    }

    /// Whether direct TLS (PG 17+) is enabled.
    pub fn ssl_direct(&self) -> bool {
        self.ssl_direct
    }

    /// Channel binding preference for SCRAM authentication.
    pub fn channel_binding(&self) -> ChannelBinding {
        self.channel_binding
    }
}

/// Builder for [`Config`].
#[derive(Debug, Clone)]
pub struct ConfigBuilder {
    host: String,
    port: u16,
    database: String,
    user: String,
    password: Option<String>,
    ssl_mode: SslMode,
    application_name: Option<String>,
    connect_timeout: Duration,
    statement_timeout: Option<Duration>,
    keepalive: Option<Duration>,
    keepalive_idle: Option<Duration>,
    target_session_attrs: TargetSessionAttrs,
    extra_float_digits: Option<i32>,
}

impl ConfigBuilder {
    fn new() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            database: String::new(),
            user: String::new(),
            password: None,
            ssl_mode: SslMode::default(),
            application_name: None,
            connect_timeout: Duration::from_secs(10),
            statement_timeout: None,
            keepalive: Some(Duration::from_secs(60)),
            keepalive_idle: None,
            target_session_attrs: TargetSessionAttrs::default(),
            extra_float_digits: Some(3),
        }
    }

    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn database(mut self, database: impl Into<String>) -> Self {
        self.database = database.into();
        self
    }

    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = user.into();
        self
    }

    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    pub fn ssl_mode(mut self, ssl_mode: SslMode) -> Self {
        self.ssl_mode = ssl_mode;
        self
    }

    pub fn application_name(mut self, name: impl Into<String>) -> Self {
        self.application_name = Some(name.into());
        self
    }

    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    pub fn statement_timeout(mut self, timeout: Duration) -> Self {
        self.statement_timeout = Some(timeout);
        self
    }

    pub fn keepalive(mut self, interval: Duration) -> Self {
        self.keepalive = Some(interval);
        self
    }

    pub fn target_session_attrs(mut self, attrs: TargetSessionAttrs) -> Self {
        self.target_session_attrs = attrs;
        self
    }

    /// Build the final `Config`.
    pub fn build(self) -> Config {
        Config {
            host: self.host,
            port: self.port,
            database: self.database,
            user: self.user,
            password: self.password,
            ssl_mode: self.ssl_mode,
            application_name: self.application_name,
            connect_timeout: self.connect_timeout,
            statement_timeout: self.statement_timeout,
            _keepalive: self.keepalive,
            _keepalive_idle: self.keepalive_idle,
            _target_session_attrs: self.target_session_attrs,
            _extra_float_digits: self.extra_float_digits,
            ssl_client_cert: None,
            ssl_client_key: None,
            ssl_direct: false,
            channel_binding: ChannelBinding::default(),
        }
    }
}

/// Percent-decode a URL component.
fn percent_decode(s: &str) -> Result<String> {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.as_bytes().iter();

    while let Some(&b) = chars.next() {
        if b == b'%' {
            let hi = chars
                .next()
                .ok_or_else(|| Error::Config("incomplete percent encoding".into()))?;
            let lo = chars
                .next()
                .ok_or_else(|| Error::Config("incomplete percent encoding".into()))?;
            let byte = hex_digit(*hi)? << 4 | hex_digit(*lo)?;
            result.push(byte as char);
        } else {
            result.push(b as char);
        }
    }

    Ok(result)
}

fn hex_digit(b: u8) -> Result<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(Error::Config(format!("invalid hex digit: {}", b as char))),
    }
}
