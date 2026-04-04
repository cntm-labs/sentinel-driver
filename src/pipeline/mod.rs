pub mod batch;

use std::sync::Arc;

use bytes::BytesMut;

use crate::error::{Error, Result};
use crate::protocol::backend::BackendMessage;
use crate::protocol::frontend;
use crate::row::{CommandResult, Row, RowDescription, parse_command_tag};

/// A single query in a pipeline with its bound parameters.
#[derive(Debug)]
pub(crate) struct PipelineQuery {
    pub sql: String,
    pub param_types: Vec<u32>,
    pub params: Vec<Option<Vec<u8>>>,
}

/// Result of a single query within a pipeline.
#[derive(Debug)]
pub enum QueryResult {
    /// Query returned rows.
    Rows(Vec<Row>),
    /// Query returned a command result (INSERT/UPDATE/DELETE/etc).
    Command(CommandResult),
}

impl QueryResult {
    /// Get rows if this is a row-returning query.
    pub fn into_rows(self) -> Result<Vec<Row>> {
        match self {
            QueryResult::Rows(rows) => Ok(rows),
            QueryResult::Command(_) => Err(Error::Protocol(
                "expected rows but got command result".to_string(),
            )),
        }
    }

    /// Get command result if this is a non-row query.
    pub fn into_command(self) -> Result<CommandResult> {
        match self {
            QueryResult::Command(r) => Ok(r),
            QueryResult::Rows(_) => Err(Error::Protocol(
                "expected command result but got rows".to_string(),
            )),
        }
    }
}

/// Encode a pipeline of queries into the write buffer.
///
/// Each query gets: Parse (unnamed) → Bind → Describe → Execute
/// A single Sync is appended at the end (single pipeline barrier).
pub(crate) fn encode_pipeline(buf: &mut BytesMut, queries: &[PipelineQuery]) {
    for q in queries {
        // Parse with unnamed statement ("")
        let oids: Vec<u32> = q.param_types.clone();
        frontend::parse(buf, "", &q.sql, &oids);

        // Bind with unnamed portal and statement
        let param_refs: Vec<Option<&[u8]>> = q
            .params
            .iter()
            .map(|p| p.as_deref())
            .collect();
        frontend::bind(buf, "", "", &param_refs, &[]);

        // Describe portal to get RowDescription (if SELECT)
        frontend::describe_portal(buf, "");

        // Execute with no row limit
        frontend::execute(buf, "", 0);
    }

    // Single Sync at the end — acts as pipeline barrier
    frontend::sync(buf);
}

/// Read pipeline responses for `count` queries.
///
/// Expected sequence per query:
/// - ParseComplete
/// - BindComplete
/// - RowDescription (or NoData for non-SELECT)
/// - DataRow* + CommandComplete (or just CommandComplete)
///
/// Finally: ReadyForQuery after the Sync.
pub(crate) async fn read_pipeline_responses(
    conn: &mut crate::connection::stream::PgConnection,
    count: usize,
) -> Result<Vec<QueryResult>> {
    let mut results = Vec::with_capacity(count);

    for _ in 0..count {
        // ParseComplete
        expect_message(conn, "ParseComplete", |m| {
            matches!(m, BackendMessage::ParseComplete)
        })
        .await?;

        // BindComplete
        expect_message(conn, "BindComplete", |m| {
            matches!(m, BackendMessage::BindComplete)
        })
        .await?;

        // RowDescription or NoData
        let msg = conn.recv().await?;
        let description = match msg {
            BackendMessage::RowDescription { fields } => {
                Some(Arc::new(RowDescription::new(fields)))
            }
            BackendMessage::NoData => None,
            BackendMessage::ErrorResponse { fields } => {
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
                    "expected RowDescription or NoData, got {other:?}"
                )));
            }
        };

        // Read DataRows + CommandComplete
        let result = read_query_result(conn, description).await?;
        results.push(result);
    }

    // ReadyForQuery after Sync
    let msg = conn.recv().await?;
    match msg {
        BackendMessage::ReadyForQuery { .. } => {}
        BackendMessage::ErrorResponse { fields } => {
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
                "expected ReadyForQuery, got {other:?}"
            )));
        }
    }

    Ok(results)
}

/// Read DataRows and CommandComplete for a single query in the pipeline.
async fn read_query_result(
    conn: &mut crate::connection::stream::PgConnection,
    description: Option<Arc<RowDescription>>,
) -> Result<QueryResult> {
    let mut rows = Vec::new();

    loop {
        let msg = conn.recv().await?;
        match msg {
            BackendMessage::DataRow { columns } => {
                let desc = description.as_ref().ok_or_else(|| {
                    Error::protocol("received DataRow without RowDescription")
                })?;
                rows.push(Row::new(columns, Arc::clone(desc)));
            }
            BackendMessage::CommandComplete { tag } => {
                if rows.is_empty() {
                    return Ok(QueryResult::Command(parse_command_tag(&tag)));
                } else {
                    return Ok(QueryResult::Rows(rows));
                }
            }
            BackendMessage::EmptyQueryResponse => {
                return Ok(QueryResult::Command(CommandResult {
                    command: String::new(),
                    rows_affected: 0,
                }));
            }
            BackendMessage::ErrorResponse { fields } => {
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
                    "unexpected message in query result: {other:?}"
                )));
            }
        }
    }
}

async fn expect_message(
    conn: &mut crate::connection::stream::PgConnection,
    expected: &str,
    check: impl FnOnce(&BackendMessage) -> bool,
) -> Result<()> {
    let msg = conn.recv().await?;
    if check(&msg) {
        Ok(())
    } else if let BackendMessage::ErrorResponse { fields } = msg {
        Err(Error::server(
            fields.severity,
            fields.code,
            fields.message,
            fields.detail,
            fields.hint,
            fields.position,
        ))
    } else {
        Err(Error::protocol(format!(
            "expected {expected}, got {msg:?}"
        )))
    }
}
