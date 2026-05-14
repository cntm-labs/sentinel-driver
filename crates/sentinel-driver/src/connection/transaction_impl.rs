use super::{notify, Connection, Result, TransactionConfig};

impl Connection {
    /// Begin a transaction with default settings.
    pub async fn begin(&mut self) -> Result<()> {
        self.begin_with(TransactionConfig::new()).await
    }

    /// Begin a transaction with custom settings.
    pub async fn begin_with(&mut self, config: TransactionConfig) -> Result<()> {
        self.instr().on_event(&crate::Event::TxBegin {
            isolation: config.isolation,
        });
        self.simple_query(&config.begin_sql()).await?;
        Ok(())
    }

    /// Commit the current transaction.
    pub async fn commit(&mut self) -> Result<()> {
        let started = std::time::Instant::now();
        let res = self.simple_query("COMMIT").await;
        let duration = started.elapsed();
        self.instr().on_event(&crate::Event::TxCommit { duration });
        res.map(|_| ())
    }

    /// Rollback the current transaction.
    pub async fn rollback(&mut self) -> Result<()> {
        let started = std::time::Instant::now();
        let res = self.simple_query("ROLLBACK").await;
        let duration = started.elapsed();
        self.instr().on_event(&crate::Event::TxRollback {
            duration,
            reason: crate::RollbackReason::Explicit,
        });
        res.map(|_| ())
    }

    /// Create a savepoint.
    pub async fn savepoint(&mut self, name: &str) -> Result<()> {
        self.simple_query(&format!("SAVEPOINT {}", notify::quote_identifier(name)))
            .await?;
        Ok(())
    }

    /// Rollback to a savepoint.
    pub async fn rollback_to(&mut self, name: &str) -> Result<()> {
        self.simple_query(&format!(
            "ROLLBACK TO SAVEPOINT {}",
            notify::quote_identifier(name)
        ))
        .await?;
        Ok(())
    }
}
