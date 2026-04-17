use super::{
    pipeline, startup, BackendMessage, BytesMut, CancelToken, Config, Connection, Duration, Error,
    PgConnection, PipelineBatch, Result, StatementCache, ToSql, TransactionStatus,
};
use crate::config::{LoadBalanceHosts, TargetSessionAttrs};

impl Connection {
    /// Connect to PostgreSQL and perform the startup handshake.
    ///
    /// With multiple hosts configured, tries each host in order (or shuffled
    /// if `load_balance_hosts=random`) until one succeeds and matches the
    /// required `target_session_attrs`.
    pub async fn connect(config: Config) -> Result<Self> {
        let mut hosts: Vec<(String, u16)> = config.hosts().to_vec();

        if hosts.is_empty() {
            hosts.push(("localhost".to_string(), 5432));
        }

        if config.load_balance_hosts() == LoadBalanceHosts::Random {
            use rand::seq::SliceRandom;
            use rand::thread_rng;
            hosts.shuffle(&mut thread_rng());
        }

        let mut last_error: Option<Error> = None;

        for (host, port) in &hosts {
            match Self::try_connect_host(&config, host, *port).await {
                Ok(conn) => return Ok(conn),
                Err(e) => {
                    tracing::debug!(host = %host, port = %port, error = %e, "host failed");
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| Error::AllHostsFailed("no hosts configured".to_string())))
    }

    /// Try connecting to a single host, performing startup and session attrs check.
    async fn try_connect_host(config: &Config, host: &str, port: u16) -> Result<Self> {
        let mut conn = PgConnection::connect_host(config, host, port).await?;
        let result = startup::startup(&mut conn, config).await?;

        // Check target_session_attrs after successful auth
        if config.target_session_attrs() != TargetSessionAttrs::Any {
            startup::check_session_attrs(&mut conn, config.target_session_attrs()).await?;
        }

        let query_timeout = config.statement_timeout();

        Ok(Self {
            conn,
            config: config.clone(),
            connected_host: host.to_string(),
            connected_port: port,
            process_id: result.process_id,
            secret_key: result.secret_key,
            transaction_status: result.transaction_status,
            stmt_cache: StatementCache::new(),
            query_timeout,
            is_broken: false,
        })
    }

    /// Close the connection gracefully.
    pub async fn close(self) -> Result<()> {
        self.conn.close().await
    }

    /// Get a cancel token for this connection.
    ///
    /// The token can be cloned and sent to another task to cancel a
    /// running query. See [`CancelToken`] for details.
    pub fn cancel_token(&self) -> CancelToken {
        CancelToken::new(
            &self.connected_host,
            self.connected_port,
            self.process_id,
            self.secret_key,
        )
    }

    /// Returns `true` if the connection is using TLS.
    /// Returns the configuration used for this connection.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns the host this connection is connected to.
    pub fn connected_host(&self) -> &str {
        &self.connected_host
    }

    /// Returns the port this connection is connected to.
    pub fn connected_port(&self) -> u16 {
        self.connected_port
    }

    pub fn is_tls(&self) -> bool {
        self.conn.is_tls()
    }

    /// Returns `true` if connected via Unix domain socket.
    #[cfg(unix)]
    pub fn is_unix(&self) -> bool {
        self.conn.is_unix()
    }

    /// The server process ID for this connection.
    pub fn process_id(&self) -> i32 {
        self.process_id
    }

    /// Returns the configured query timeout, if any.
    pub fn query_timeout(&self) -> Option<Duration> {
        self.query_timeout
    }

    /// Returns `true` if the connection has been marked broken by a timeout.
    ///
    /// A broken connection should be discarded — the server state is
    /// indeterminate after a cancelled query.
    pub fn is_broken(&self) -> bool {
        self.is_broken
    }

    /// Current transaction status.
    pub fn transaction_status(&self) -> TransactionStatus {
        self.transaction_status
    }

    /// Access the underlying PgConnection mutably.
    pub(crate) fn pg_connection_mut(&mut self) -> &mut PgConnection {
        &mut self.conn
    }

    // ── Internal ─────────────────────────────────────

    pub(crate) async fn query_internal(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<pipeline::QueryResult> {
        // Encode parameters
        let param_types: Vec<u32> = params.iter().map(|p| p.oid().0).collect();
        let mut encoded_params: Vec<Option<Vec<u8>>> = Vec::with_capacity(params.len());

        for param in params {
            if param.is_null() {
                encoded_params.push(None);
            } else {
                let mut buf = BytesMut::new();
                param.to_sql(&mut buf)?;
                encoded_params.push(Some(buf.to_vec()));
            }
        }

        // Use pipeline for single query (same protocol, consistent code path)
        let mut batch = PipelineBatch::new();
        batch.add(sql.to_string(), param_types, encoded_params);

        let mut results = batch.execute(&mut self.conn).await?;

        results
            .pop()
            .ok_or_else(|| Error::protocol("pipeline returned no results"))
    }

    pub(crate) async fn drain_until_ready(&mut self) -> Result<()> {
        loop {
            if let BackendMessage::ReadyForQuery { transaction_status } = self.conn.recv().await? {
                self.transaction_status = transaction_status;
                return Ok(());
            }
        }
    }
}
