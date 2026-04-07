use crate::connection::stream::PgConnection;
use crate::error::Result;
use crate::pipeline::{encode_pipeline, read_pipeline_responses, PipelineQuery, QueryResult};

/// A batch of queries to be pipelined together.
///
/// Collects queries and executes them in a single pipeline round-trip.
///
/// ```rust,no_run
/// # async fn example(conn: &mut sentinel_driver::pipeline::batch::PipelineBatch) {
/// // All queries sent in one round-trip:
/// // conn.pipeline(|p| {
/// //     p.query("SELECT * FROM users WHERE active = $1", &[&true]);
/// //     p.query("SELECT * FROM posts WHERE published = $1", &[&true]);
/// // }).await?;
/// # }
/// ```
pub struct PipelineBatch {
    queries: Vec<PipelineQuery>,
}

impl PipelineBatch {
    pub fn new() -> Self {
        Self {
            queries: Vec::new(),
        }
    }

    /// Add a query to the pipeline batch.
    ///
    /// `param_types` are the OIDs of the parameters (0 = let server infer).
    /// `params` are pre-encoded binary values (None = NULL).
    pub fn add(
        &mut self,
        sql: impl Into<String>,
        param_types: Vec<u32>,
        params: Vec<Option<Vec<u8>>>,
    ) {
        self.queries.push(PipelineQuery {
            sql: sql.into(),
            param_types,
            params,
        });
    }

    /// Number of queries in the batch.
    pub fn len(&self) -> usize {
        self.queries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queries.is_empty()
    }

    /// Execute the pipeline batch, returning results for each query.
    ///
    /// All queries are sent to the server in a single write, then all
    /// responses are read in order. This reduces N round-trips to 1.
    pub(crate) async fn execute(self, conn: &mut PgConnection) -> Result<Vec<QueryResult>> {
        if self.queries.is_empty() {
            return Ok(Vec::new());
        }

        let count = self.queries.len();

        // Encode all queries into the write buffer
        encode_pipeline(conn.write_buf(), &self.queries);

        // Send everything in one flush
        conn.send().await?;

        // Read all responses
        read_pipeline_responses(conn, count).await
    }
}

impl Default for PipelineBatch {
    fn default() -> Self {
        Self::new()
    }
}
