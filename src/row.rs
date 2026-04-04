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
    pub(crate) fn new(fields: Vec<FieldDescription>) -> Self {
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
    pub(crate) fn new(columns: DataRowColumns, description: Arc<RowDescription>) -> Self {
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
        self.try_get(idx).unwrap_or_else(|e| {
            panic!("error getting column {idx}: {e}")
        })
    }

    /// Get a typed column value by name.
    ///
    /// # Panics
    ///
    /// Panics if the column name doesn't exist or the value cannot be decoded.
    pub fn get_by_name<T: FromSql>(&self, name: &str) -> T {
        self.try_get_by_name(name).unwrap_or_else(|e| {
            panic!("error getting column '{name}': {e}")
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::backend;

    fn make_test_description() -> Arc<RowDescription> {
        Arc::new(RowDescription::new(vec![
            FieldDescription {
                name: "id".to_string(),
                table_oid: 0,
                column_id: 0,
                type_oid: 23, // int4
                type_size: 4,
                type_modifier: -1,
                format: 1,
            },
            FieldDescription {
                name: "name".to_string(),
                table_oid: 0,
                column_id: 1,
                type_oid: 25, // text
                type_size: -1,
                type_modifier: -1,
                format: 1,
            },
            FieldDescription {
                name: "score".to_string(),
                table_oid: 0,
                column_id: 2,
                type_oid: 23, // int4
                type_size: 4,
                type_modifier: -1,
                format: 1,
            },
        ]))
    }

    fn make_data_row(body: bytes::Bytes) -> DataRowColumns {
        // Decode a DataRow message to get DataRowColumns
        match backend::decode(b'D', body).unwrap() {
            backend::BackendMessage::DataRow { columns } => columns,
            _ => panic!("expected DataRow"),
        }
    }

    fn build_data_row_bytes(columns: &[Option<&[u8]>]) -> bytes::Bytes {
        use bytes::BufMut;
        let mut buf = bytes::BytesMut::new();
        buf.put_i16(columns.len() as i16);
        for col in columns {
            match col {
                Some(data) => {
                    buf.put_i32(data.len() as i32);
                    buf.put_slice(data);
                }
                None => {
                    buf.put_i32(-1);
                }
            }
        }
        buf.freeze()
    }

    #[test]
    fn test_row_get_by_index() {
        let desc = make_test_description();
        let data = build_data_row_bytes(&[
            Some(&42i32.to_be_bytes()),
            Some(b"Alice"),
            Some(&100i32.to_be_bytes()),
        ]);
        let columns = make_data_row(data);
        let row = Row::new(columns, desc);

        let id: i32 = row.get(0);
        assert_eq!(id, 42);

        let name: String = row.get(1);
        assert_eq!(name, "Alice");

        let score: i32 = row.get(2);
        assert_eq!(score, 100);
    }

    #[test]
    fn test_row_get_by_name() {
        let desc = make_test_description();
        let data = build_data_row_bytes(&[
            Some(&7i32.to_be_bytes()),
            Some(b"Bob"),
            Some(&99i32.to_be_bytes()),
        ]);
        let columns = make_data_row(data);
        let row = Row::new(columns, desc);

        let name: String = row.get_by_name("name");
        assert_eq!(name, "Bob");

        let id: i32 = row.get_by_name("id");
        assert_eq!(id, 7);
    }

    #[test]
    fn test_row_null_handling() {
        let desc = make_test_description();
        let data = build_data_row_bytes(&[
            Some(&1i32.to_be_bytes()),
            None, // name is NULL
            Some(&50i32.to_be_bytes()),
        ]);
        let columns = make_data_row(data);
        let row = Row::new(columns, desc);

        assert!(row.is_null(1));
        assert!(!row.is_null(0));

        let name: Option<String> = row.try_get(1).unwrap();
        assert_eq!(name, None);
    }

    #[test]
    fn test_row_index_out_of_bounds() {
        let desc = make_test_description();
        let data = build_data_row_bytes(&[
            Some(&1i32.to_be_bytes()),
            Some(b"X"),
            Some(&0i32.to_be_bytes()),
        ]);
        let columns = make_data_row(data);
        let row = Row::new(columns, desc);

        assert!(row.try_get::<i32>(10).is_err());
    }

    #[test]
    fn test_row_column_not_found() {
        let desc = make_test_description();
        let data = build_data_row_bytes(&[
            Some(&1i32.to_be_bytes()),
            Some(b"X"),
            Some(&0i32.to_be_bytes()),
        ]);
        let columns = make_data_row(data);
        let row = Row::new(columns, desc);

        assert!(row.try_get_by_name::<String>("nonexistent").is_err());
    }

    #[test]
    fn test_parse_command_tag() {
        let r = parse_command_tag("INSERT 0 5");
        assert_eq!(r.command, "INSERT");
        assert_eq!(r.rows_affected, 5);

        let r = parse_command_tag("UPDATE 3");
        assert_eq!(r.command, "UPDATE");
        assert_eq!(r.rows_affected, 3);

        let r = parse_command_tag("DELETE 0");
        assert_eq!(r.command, "DELETE");
        assert_eq!(r.rows_affected, 0);

        let r = parse_command_tag("SELECT 100");
        assert_eq!(r.command, "SELECT");
        assert_eq!(r.rows_affected, 100);

        let r = parse_command_tag("CREATE TABLE");
        assert_eq!(r.command, "CREATE");
        assert_eq!(r.rows_affected, 0);
    }

    #[test]
    fn test_row_description() {
        let desc = make_test_description();
        assert_eq!(desc.len(), 3);
        assert_eq!(desc.column_index("name"), Some(1));
        assert_eq!(desc.column_index("nonexistent"), None);
        assert_eq!(desc.field(0).unwrap().name, "id");
    }
}
