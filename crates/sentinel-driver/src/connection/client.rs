use super::{
    BackendMessage, BytesMut, CancelToken, Config, Connection, Duration, Error, PgConnection,
    PipelineBatch, Result, StatementCache, ToSql, TransactionStatus, pipeline, startup,
};

impl Connection {
    /// Connect to PostgreSQL and perform the startup handshake.
    pub async fn connect(config: Config) -> Result<Self> {
        let mut conn = PgConnection::connect(&config).await?;
        let result = startup::startup(&mut conn, &config).await?;
        let query_timeout = config.statement_timeout();

        Ok(Self {
            conn,
            config,
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
            self.config.host(),
            self.config.port(),
            self.process_id,
            self.secret_key,
        )
    }

    /// Returns `true` if the connection is using TLS.
    pub fn is_tls(&self) -> bool {
        self.conn.is_tls()
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
            let mut buf = BytesMut::new();
            param.to_sql(&mut buf)?;
            encoded_params.push(Some(buf.to_vec()));
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
