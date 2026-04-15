use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;

use crate::error::{Error, Result};
use crate::protocol::backend::{DataRowColumns, FieldDescription};
use crate::types::FromSql;

/// Shared column metadata for a result set.
///
/// Created once from RowDescription, shared across all rows via `Arc`.
#[derive(Debug, Clone)]
pub struct RowDescription {
    fields: Vec<FieldDescription>,
    name_index: HashMap<String, usize>,
}

impl RowDescription {
    pub fn new(fields: Vec<FieldDescription>) -> Self {
        let name_index = fields
            .iter()
            .enumerate()
            .map(|(i, f)| (f.name.clone(), i))
            .collect();

        Self { fields, name_index }
    }

    /// Number of columns.
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Get field description by index.
    pub fn field(&self, idx: usize) -> Option<&FieldDescription> {
        self.fields.get(idx)
    }

    /// Get column index by name.
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.name_index.get(name).copied()
    }

    /// Iterator over field descriptions.
    pub fn fields(&self) -> &[FieldDescription] {
        &self.fields
    }
}

/// A single row from a query result.
///
/// Provides zero-copy column access — data is decoded on demand from the
/// underlying `Bytes` buffer.
#[derive(Debug)]
pub struct Row {
    columns: DataRowColumns,
    description: Arc<RowDescription>,
}

impl Row {
    pub fn new(columns: DataRowColumns, description: Arc<RowDescription>) -> Self {
        Self {
            columns,
            description,
        }
    }

    /// Get a typed column value by index.
    ///
    /// # Panics
    ///
    /// Panics if the column index is out of bounds or the value cannot be decoded.
    /// Use [`try_get`](Self::try_get) for a non-panicking version.
    pub fn get<T: FromSql>(&self, idx: usize) -> T {
        self.try_get(idx)
            .unwrap_or_else(|e| panic!("error getting column {idx}: {e}"))
    }

    /// Get a typed column value by name.
    ///
    /// # Panics
    ///
    /// Panics if the column name doesn't exist or the value cannot be decoded.
    pub fn get_by_name<T: FromSql>(&self, name: &str) -> T {
        self.try_get_by_name(name)
            .unwrap_or_else(|e| panic!("error getting column '{name}': {e}"))
    }

    /// Try to get a typed column value by index.
    pub fn try_get<T: FromSql>(&self, idx: usize) -> Result<T> {
        if idx >= self.columns.len() {
            return Err(Error::ColumnIndex {
                index: idx,
                count: self.columns.len(),
            });
        }

        let raw = self.columns.get(idx);
        T::from_sql_nullable(raw.as_deref())
    }

    /// Try to get a typed column value by name.
    pub fn try_get_by_name<T: FromSql>(&self, name: &str) -> Result<T> {
        let idx = self
            .description
            .column_index(name)
            .ok_or_else(|| Error::ColumnNotFound(name.to_string()))?;
        self.try_get(idx)
    }

    /// Get raw bytes for a column. Returns `None` for NULL.
    pub fn get_raw(&self, idx: usize) -> Option<Bytes> {
        self.columns.get(idx)
    }

    /// Check if a column is NULL.
    pub fn is_null(&self, idx: usize) -> bool {
        self.columns.is_null(idx)
    }

    /// Number of columns.
    pub fn len(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    /// Get the row description (column metadata).
    pub fn description(&self) -> &RowDescription {
        &self.description
    }
}

/// Parse the command tag from CommandComplete to extract affected row count.
///
/// Tags look like: "INSERT 0 5", "UPDATE 3", "DELETE 1", "SELECT 10"
pub fn parse_command_tag(tag: &str) -> CommandResult {
    let parts: Vec<&str> = tag.split_whitespace().collect();
    let command = parts.first().copied().unwrap_or("");

    let rows_affected = match command {
        "INSERT" => parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0),
        "SELECT" | "UPDATE" | "DELETE" | "COPY" | "MERGE" | "MOVE" | "FETCH" => {
            parts.last().and_then(|s| s.parse().ok()).unwrap_or(0)
        }
        _ => 0,
    };

    CommandResult {
        command: command.to_string(),
        rows_affected,
    }
}

/// Result of a command execution (non-SELECT queries).
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub command: String,
    pub rows_affected: u64,
}

/// A message returned from the simple query protocol.
///
/// Simple queries can return a mix of row data and command completions
/// (e.g., a multi-statement query like `"SELECT 1; INSERT INTO ..."`).
#[derive(Debug, Clone)]
pub enum SimpleQueryMessage {
    /// A row of text-format data from a SELECT or RETURNING clause.
    Row(SimpleQueryRow),
    /// A command completed (INSERT, UPDATE, DELETE, etc.).
    CommandComplete(u64),
}

/// A single row from the simple query protocol.
///
/// All column values are in PostgreSQL text format. NULL values are
/// represented as `None`.
#[derive(Debug, Clone)]
pub struct SimpleQueryRow {
    columns: Vec<Option<String>>,
}

impl SimpleQueryRow {
    pub fn new(columns: Vec<Option<String>>) -> Self {
        Self { columns }
    }

    /// Get a column value by index. Returns `None` for NULL.
    pub fn get(&self, idx: usize) -> Option<&str> {
        self.columns.get(idx).and_then(|c| c.as_deref())
    }

    /// Get a column value by index, returning an error if the index is
    /// out of bounds or the value is NULL.
    pub fn try_get(&self, idx: usize) -> Result<&str> {
        if idx >= self.columns.len() {
            return Err(Error::ColumnIndex {
                index: idx,
                count: self.columns.len(),
            });
        }
        self.columns[idx]
            .as_deref()
            .ok_or_else(|| Error::Decode("unexpected NULL in simple query row".into()))
    }

    /// Number of columns in this row.
    pub fn len(&self) -> usize {
        self.columns.len()
    }

    /// Returns `true` if this row has no columns.
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }
}
