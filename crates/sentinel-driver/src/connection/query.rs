use super::{
    frontend, pipeline, BackendMessage, BytesMut, Connection, Duration, Error, Oid, PipelineBatch,
    Result, Row, ToSql,
};

use crate::row::{self, SimpleQueryMessage, SimpleQueryRow};

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
    /// Returns row data (in text format) and command completions. Useful
    /// for DDL statements, multi-statement queries, and queries where you
    /// don't need binary-decoded typed values.
    ///
    /// ```rust,no_run
    /// # async fn example(conn: &mut sentinel_driver::Connection) -> sentinel_driver::Result<()> {
    /// use sentinel_driver::SimpleQueryMessage;
    ///
    /// let messages = conn.simple_query("SELECT 1 AS n; SELECT 'hello' AS greeting").await?;
    /// for msg in &messages {
    ///     match msg {
    ///         SimpleQueryMessage::Row(row) => {
    ///             println!("value: {:?}", row.get(0));
    ///         }
    ///         SimpleQueryMessage::CommandComplete(n) => {
    ///             println!("rows: {n}");
    ///         }
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn simple_query(&mut self, sql: &str) -> Result<Vec<SimpleQueryMessage>> {
        frontend::query(self.conn.write_buf(), sql);
        self.conn.send().await?;

        let mut results = Vec::new();

        loop {
            match self.conn.recv().await? {
                BackendMessage::DataRow { columns } => {
                    // Extract text-format column values from DataRow
                    let mut text_columns = Vec::with_capacity(columns.len());
                    for i in 0..columns.len() {
                        let value = columns
                            .get(i)
                            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
                        text_columns.push(value);
                    }
                    results.push(SimpleQueryMessage::Row(SimpleQueryRow::new(text_columns)));
                }
                BackendMessage::CommandComplete { tag } => {
                    let parsed = row::parse_command_tag(&tag);
                    results.push(SimpleQueryMessage::CommandComplete(parsed.rows_affected));
                }
                BackendMessage::ReadyForQuery { transaction_status } => {
                    self.transaction_status = transaction_status;
                    break;
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
                _ => {}
            }
        }

        Ok(results)
    }

    // ── query_typed ────────────────────────────────────

    /// Execute a query with inline parameter types, skipping the prepare step.
    ///
    /// Instead of a separate Prepare round-trip, the parameter types are
    /// specified directly in the Parse message. This saves one round-trip
    /// compared to [`query()`](Self::query) at the cost of requiring the
    /// caller to specify types explicitly.
    ///
    /// ```rust,no_run
    /// # async fn example(conn: &mut sentinel_driver::Connection) -> sentinel_driver::Result<()> {
    /// use sentinel_driver::Oid;
    ///
    /// let rows = conn.query_typed(
    ///     "SELECT $1::int4 + $2::int4 AS sum",
    ///     &[(&1i32, Oid::INT4), (&2i32, Oid::INT4)],
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_typed(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), Oid)],
    ) -> Result<Vec<Row>> {
        let result = self.query_typed_internal(sql, params).await?;
        match result {
            pipeline::QueryResult::Rows(rows) => Ok(rows),
            pipeline::QueryResult::Command(_) => Ok(Vec::new()),
        }
    }

    /// Execute a typed query that returns a single row.
    pub async fn query_typed_one(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), Oid)],
    ) -> Result<Row> {
        let rows = self.query_typed(sql, params).await?;
        rows.into_iter()
            .next()
            .ok_or_else(|| Error::Protocol("query returned no rows".into()))
    }

    /// Execute a typed query that returns an optional single row.
    pub async fn query_typed_opt(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), Oid)],
    ) -> Result<Option<Row>> {
        let rows = self.query_typed(sql, params).await?;
        Ok(rows.into_iter().next())
    }

    /// Execute a typed non-SELECT query, returning rows affected.
    pub async fn execute_typed(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), Oid)],
    ) -> Result<u64> {
        let result = self.query_typed_internal(sql, params).await?;
        match result {
            pipeline::QueryResult::Command(r) => Ok(r.rows_affected),
            pipeline::QueryResult::Rows(_) => Ok(0),
        }
    }

    async fn query_typed_internal(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), Oid)],
    ) -> Result<pipeline::QueryResult> {
        let param_types: Vec<u32> = params.iter().map(|(_, oid)| oid.0).collect();
        let mut encoded_params: Vec<Option<Vec<u8>>> = Vec::with_capacity(params.len());

        for (value, _) in params {
            if value.is_null() {
                encoded_params.push(None);
            } else {
                let mut buf = BytesMut::new();
                value.to_sql(&mut buf)?;
                encoded_params.push(Some(buf.to_vec()));
            }
        }

        let mut batch = PipelineBatch::new();
        batch.add(sql.to_string(), param_types, encoded_params);

        let mut results = batch.execute(&mut self.conn).await?;
        results
            .pop()
            .ok_or_else(|| Error::protocol("pipeline returned no results"))
    }
}
