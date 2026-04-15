use std::sync::Arc;

use sentinel_driver::protocol::backend::{self, BackendMessage, DataRowColumns, FieldDescription};
use sentinel_driver::row::{
    parse_command_tag, Row, RowDescription, SimpleQueryMessage, SimpleQueryRow,
};

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
    match backend::decode(b'D', body).unwrap() {
        BackendMessage::DataRow { columns } => columns,
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

// ── SimpleQueryRow tests ──────────────────────────

#[test]
fn test_simple_query_row_get() {
    let row = SimpleQueryRow::new(vec![
        Some("42".to_string()),
        Some("hello".to_string()),
        None,
    ]);

    assert_eq!(row.get(0), Some("42"));
    assert_eq!(row.get(1), Some("hello"));
    assert_eq!(row.get(2), None);
    assert_eq!(row.get(99), None);
}

#[test]
fn test_simple_query_row_try_get() {
    let row = SimpleQueryRow::new(vec![Some("value".to_string()), None]);

    assert_eq!(row.try_get(0).unwrap(), "value");
    assert!(row.try_get(1).is_err()); // NULL
    assert!(row.try_get(5).is_err()); // out of bounds
}

#[test]
fn test_simple_query_row_len() {
    let row = SimpleQueryRow::new(vec![Some("a".to_string()), Some("b".to_string())]);
    assert_eq!(row.len(), 2);
    assert!(!row.is_empty());

    let empty = SimpleQueryRow::new(vec![]);
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
}

#[test]
fn test_simple_query_message_variants() {
    let row_msg = SimpleQueryMessage::Row(SimpleQueryRow::new(vec![Some("1".to_string())]));
    let cmd_msg = SimpleQueryMessage::CommandComplete(5);

    match row_msg {
        SimpleQueryMessage::Row(r) => assert_eq!(r.get(0), Some("1")),
        _ => panic!("expected Row"),
    }

    match cmd_msg {
        SimpleQueryMessage::CommandComplete(n) => assert_eq!(n, 5),
        _ => panic!("expected CommandComplete"),
    }
}
