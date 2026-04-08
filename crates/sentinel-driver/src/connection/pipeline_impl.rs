use super::{Connection, PipelineBatch, Result, pipeline};

impl Connection {
    /// Create a pipeline batch for executing multiple queries in a single round-trip.
    ///
    /// Use `execute_pipeline()` to send the batch.
    pub fn pipeline(&self) -> PipelineBatch {
        PipelineBatch::new()
    }

    /// Execute a pipeline batch, returning results for each query.
    pub async fn execute_pipeline(
        &mut self,
        batch: PipelineBatch,
    ) -> Result<Vec<pipeline::QueryResult>> {
        batch.execute(&mut self.conn).await
    }
}
