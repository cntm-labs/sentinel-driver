use super::{
    BackendMessage, CommandResult, Connection, Duration, Error, Result, Row, ToSql, frontend,
    pipeline, row,
};

impl Connection {
    /// Execute a query that returns rows.
    ///
    /// Parameters are encoded in binary format.
    ///
    /// ```rust,no_run
    /// # async fn example(conn: &mut sentinel_driver::Connection) -> sentinel_driver::Result<()> {
    /// let rows = conn.query("SELECT * FROM users WHERE id = $1", &[&42i32]).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        if let Some(timeout) = self.query_timeout {
            return self.query_with_timeout(sql, params, timeout).await;
        }

        let result = self.query_internal(sql, params).await?;
        match result {
            pipeline::QueryResult::Rows(rows) => Ok(rows),
            pipeline::QueryResult::Command(_) => Ok(Vec::new()),
        }
    }

    /// Execute a query that returns a single row.
    ///
    /// Returns an error if no rows are returned.
    pub async fn query_one(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        let rows = self.query(sql, params).await?;
        rows.into_iter()
            .next()
            .ok_or_else(|| Error::Protocol("query returned no rows".into()))
    }

    /// Execute a query that returns an optional single row.
    pub async fn query_opt(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>> {
        let rows = self.query(sql, params).await?;
        Ok(rows.into_iter().next())
    }

    /// Execute a non-SELECT query (INSERT, UPDATE, DELETE, etc.).
    ///
    /// Returns the number of rows affected.
    pub async fn execute(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        if let Some(timeout) = self.query_timeout {
            return self.execute_with_timeout(sql, params, timeout).await;
        }

        let result = self.query_internal(sql, params).await?;
        match result {
            pipeline::QueryResult::Command(r) => Ok(r.rows_affected),
            pipeline::QueryResult::Rows(_) => Ok(0),
        }
    }

    /// Execute a query with a timeout.
    ///
    /// If the query does not complete within `timeout`, a cancel request
    /// is sent to the server and the connection is marked as broken.
    pub async fn query_with_timeout(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        timeout: Duration,
    ) -> Result<Vec<Row>> {
        let cancel_token = self.cancel_token();

        match tokio::time::timeout(timeout, self.query_internal(sql, params)).await {
            Ok(result) => {
                let result = result?;
                match result {
                    pipeline::QueryResult::Rows(rows) => Ok(rows),
                    pipeline::QueryResult::Command(_) => Ok(Vec::new()),
                }
            }
            Err(_elapsed) => {
                self.is_broken = true;
                // Fire-and-forget cancel
                tokio::spawn(async move {
                    cancel_token.cancel().await.ok();
                });
                Err(Error::Timeout(format!(
                    "query timeout after {}ms",
                    timeout.as_millis()
                )))
            }
        }
    }

    /// Execute a non-SELECT query with a timeout.
    ///
    /// If the query does not complete within `timeout`, a cancel request
    /// is sent to the server and the connection is marked as broken.
    pub async fn execute_with_timeout(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
        timeout: Duration,
    ) -> Result<u64> {
        let cancel_token = self.cancel_token();

        match tokio::time::timeout(timeout, self.query_internal(sql, params)).await {
            Ok(result) => {
                let result = result?;
                match result {
                    pipeline::QueryResult::Command(r) => Ok(r.rows_affected),
                    pipeline::QueryResult::Rows(_) => Ok(0),
                }
            }
            Err(_elapsed) => {
                self.is_broken = true;
                tokio::spawn(async move {
                    cancel_token.cancel().await.ok();
                });
                Err(Error::Timeout(format!(
                    "query timeout after {}ms",
                    timeout.as_millis()
                )))
            }
        }
    }

    /// Execute a simple query (no parameters, text protocol).
    ///
    /// Useful for DDL statements and multi-statement queries.
    pub async fn simple_query(&mut self, sql: &str) -> Result<Vec<CommandResult>> {
        frontend::query(self.conn.write_buf(), sql);
        self.conn.send().await?;

        let mut results = Vec::new();

        loop {
            match self.conn.recv().await? {
                BackendMessage::CommandComplete { tag } => {
                    results.push(row::parse_command_tag(&tag));
                }
                BackendMessage::ReadyForQuery { transaction_status } => {
                    self.transaction_status = transaction_status;
                    break;
                }
                BackendMessage::ErrorResponse { fields } => {
                    // Drain until ReadyForQuery
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
                _ => {}
            }
        }

        Ok(results)
    }
}
