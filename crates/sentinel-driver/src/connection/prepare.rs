use super::{frontend, BackendMessage, CacheMetrics, Connection, Error, Oid, Result, Statement};

impl Connection {
    /// Prepare a statement on the server using extended query protocol.
    ///
    /// Returns a `Statement` with parameter types and column descriptions.
    pub async fn prepare(&mut self, sql: &str) -> Result<Statement> {
        let stmt_name = format!("_sentinel_p{}", self.process_id);

        frontend::parse(self.conn.write_buf(), &stmt_name, sql, &[]);
        frontend::describe_statement(self.conn.write_buf(), &stmt_name);
        frontend::sync(self.conn.write_buf());
        self.conn.send().await?;

        // ParseComplete
        match self.conn.recv().await? {
            BackendMessage::ParseComplete => {}
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
            other => {
                return Err(Error::protocol(format!(
                    "expected ParseComplete, got {other:?}"
                )))
            }
        }

        // ParameterDescription
        let param_oids = match self.conn.recv().await? {
            BackendMessage::ParameterDescription { oids } => {
                oids.into_iter().map(Oid::from).collect()
            }
            other => {
                return Err(Error::protocol(format!(
                    "expected ParameterDescription, got {other:?}"
                )))
            }
        };

        // RowDescription or NoData
        let columns = match self.conn.recv().await? {
            BackendMessage::RowDescription { fields } => Some(fields),
            BackendMessage::NoData => None,
            other => {
                return Err(Error::protocol(format!(
                    "expected RowDescription/NoData, got {other:?}"
                )))
            }
        };

        // ReadyForQuery
        self.drain_until_ready().await?;

        Ok(Statement::new(
            stmt_name,
            sql.to_string(),
            param_oids,
            columns,
        ))
    }

    /// Register a prepared statement in the Tier 1 cache.
    pub fn register_statement(&mut self, name: &str, statement: Statement) {
        self.stmt_cache.register(name, statement);
    }

    /// Get statement cache metrics.
    pub fn cache_metrics(&self) -> &CacheMetrics {
        self.stmt_cache.metrics()
    }
}
