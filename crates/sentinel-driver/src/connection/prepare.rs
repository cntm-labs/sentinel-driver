use super::{frontend, BackendMessage, CacheMetrics, Connection, Error, Oid, Result, Statement};

impl Connection {
    /// Prepare a statement on the server using extended query protocol.
    ///
    /// Returns a `Statement` with parameter types and column descriptions.
    ///
    /// Emits `PrepareStart` before the wire round-trip and `PrepareFinish` on
    /// success. `cache_hit` is always `false` today because `prepare()` always
    /// sends Parse; a future task that wires automatic cache lookup will toggle
    /// that field without breaking the API.
    pub async fn prepare(&mut self, sql: &str) -> Result<Statement> {
        self.instr().on_event(&crate::Event::PrepareStart {
            name: "", // server-side name not yet assigned at this point
            sql,
        });
        let started = std::time::Instant::now();
        let res = self.prepare_inner(sql).await;
        let duration = started.elapsed();
        if let Ok(stmt) = &res {
            // Collect param OIDs as raw u32 for the event (Oid is Vec<Oid>
            // internally; Option B — one allocation on the prepare path, which
            // is already a full network round-trip).
            let raw_oids: Vec<u32> = stmt.param_types().iter().map(|o| o.0).collect();
            self.instr().on_event(&crate::Event::PrepareFinish {
                name: stmt.name(),
                param_oids: &raw_oids,
                col_count: stmt.column_count() as u16,
                duration,
                cache_hit: false, // current prepare() never consults the cache
            });
        }
        // On error we don't emit PrepareFinish — the prepare failed; consumers
        // will see ExecuteFinish with the same Err if a downstream query was
        // attempted, or no event at all if the caller just discards the error.
        // (Matches the design doc's borrowing model: events are best-effort.)
        res
    }

    /// Inner prepare implementation — sends Parse/Describe/Sync and awaits the
    /// server responses. Called exclusively by `prepare()`.
    async fn prepare_inner(&mut self, sql: &str) -> Result<Statement> {
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
