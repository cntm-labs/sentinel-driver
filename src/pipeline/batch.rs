
use crate::error::Result;
use crate::pipeline::{PipelineQuery, QueryResult, encode_pipeline, read_pipeline_responses};
use crate::connection::stream::PgConnection;

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

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use crate::pipeline::encode_pipeline;

    #[test]
    fn test_batch_empty() {
        let batch = PipelineBatch::new();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
    }

    #[test]
    fn test_batch_add_queries() {
        let mut batch = PipelineBatch::new();
        batch.add("SELECT 1", vec![], vec![]);
        batch.add("SELECT 2", vec![], vec![]);
        assert_eq!(batch.len(), 2);
        assert!(!batch.is_empty());
    }

    #[test]
    fn test_encode_pipeline_single_query() {
        let queries = vec![PipelineQuery {
            sql: "SELECT 1".to_string(),
            param_types: vec![],
            params: vec![],
        }];

        let mut buf = BytesMut::new();
        encode_pipeline(&mut buf, &queries);

        // Should contain: Parse('P') + Bind('B') + Describe('D') + Execute('E') + Sync('S')
        let types: Vec<u8> = extract_message_types(&buf);
        assert_eq!(types, vec![b'P', b'B', b'D', b'E', b'S']);
    }

    #[test]
    fn test_encode_pipeline_multiple_queries() {
        let queries = vec![
            PipelineQuery {
                sql: "SELECT 1".to_string(),
                param_types: vec![],
                params: vec![],
            },
            PipelineQuery {
                sql: "SELECT 2".to_string(),
                param_types: vec![],
                params: vec![],
            },
            PipelineQuery {
                sql: "SELECT 3".to_string(),
                param_types: vec![],
                params: vec![],
            },
        ];

        let mut buf = BytesMut::new();
        encode_pipeline(&mut buf, &queries);

        // 3 queries × (P+B+D+E) + 1 Sync = 13 messages
        let types = extract_message_types(&buf);
        assert_eq!(types.len(), 13);

        // Verify pattern: P,B,D,E repeated 3x then S
        assert_eq!(types[0], b'P');
        assert_eq!(types[4], b'P');
        assert_eq!(types[8], b'P');
        assert_eq!(types[12], b'S'); // single Sync at end
    }

    #[test]
    fn test_encode_pipeline_with_params() {
        let queries = vec![PipelineQuery {
            sql: "SELECT * FROM users WHERE id = $1".to_string(),
            param_types: vec![23], // int4
            params: vec![Some(42i32.to_be_bytes().to_vec())],
        }];

        let mut buf = BytesMut::new();
        encode_pipeline(&mut buf, &queries);

        let types = extract_message_types(&buf);
        assert_eq!(types, vec![b'P', b'B', b'D', b'E', b'S']);

        // Verify the buffer contains the parameter data
        assert!(buf.len() > 30); // should be reasonably large with SQL + params
    }

    /// Extract message type bytes from an encoded buffer.
    fn extract_message_types(buf: &[u8]) -> Vec<u8> {
        let mut types = Vec::new();
        let mut pos = 0;

        while pos < buf.len() {
            let msg_type = buf[pos];
            types.push(msg_type);

            if pos + 5 > buf.len() {
                break;
            }
            let len = i32::from_be_bytes(buf[pos + 1..pos + 5].try_into().unwrap()) as usize;
            pos += 1 + len; // type byte + declared length (which includes its own 4 bytes)
        }

        types
    }
}
