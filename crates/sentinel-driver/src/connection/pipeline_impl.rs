use super::{pipeline, Connection, PipelineBatch, Result};

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
        let batch_len = batch.len();
        self.instr()
            .on_event(&crate::Event::PipelineStart { batch_len });
        let started = std::time::Instant::now();
        let res = batch.execute(&mut self.conn).await;
        let total_duration = started.elapsed();
        self.instr().on_event(&crate::Event::PipelineFlush {
            batch_len,
            total_duration,
        });
        res
    }
}
