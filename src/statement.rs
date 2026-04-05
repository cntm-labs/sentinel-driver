use std::sync::Arc;

use crate::protocol::backend::FieldDescription;
use crate::types::Oid;

/// A prepared statement with parameter types and result column descriptions.
#[derive(Debug, Clone)]
pub struct Statement {
    /// Server-assigned statement name (empty string = unnamed).
    name: String,
    /// SQL query text.
    sql: String,
    /// Parameter type OIDs (from ParameterDescription).
    param_types: Vec<Oid>,
    /// Result column descriptions (from RowDescription). `None` for non-SELECT.
    columns: Option<Arc<Vec<FieldDescription>>>,
}

impl Statement {
    pub fn new(
        name: String,
        sql: String,
        param_types: Vec<Oid>,
        columns: Option<Vec<FieldDescription>>,
    ) -> Self {
        Self {
            name,
            sql,
            param_types,
            columns: columns.map(Arc::new),
        }
    }

    /// The server-assigned statement name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The SQL query text.
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// Parameter type OIDs.
    pub fn param_types(&self) -> &[Oid] {
        &self.param_types
    }

    /// Number of parameters.
    pub fn param_count(&self) -> usize {
        self.param_types.len()
    }

    /// Result column descriptions. `None` for statements that don't return rows.
    pub fn columns(&self) -> Option<&[FieldDescription]> {
        self.columns.as_ref().map(|c| c.as_slice())
    }

    /// Number of result columns. 0 if the statement doesn't return rows.
    pub fn column_count(&self) -> usize {
        self.columns.as_ref().map_or(0, |c| c.len())
    }
}
