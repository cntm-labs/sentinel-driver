use std::collections::HashMap;

use bytes::BytesMut;
use sentinel_driver::types::{FromSql, ToSql};

#[test]
fn test_hstore_encode_empty() {
    let map: HashMap<String, Option<String>> = HashMap::new();
    let mut buf = BytesMut::new();
    map.to_sql(&mut buf).unwrap();
    // count = 0
    assert_eq!(&buf[..], &0i32.to_be_bytes());
}

#[test]
fn test_hstore_decode_empty() {
    let data = 0i32.to_be_bytes();
    let map: HashMap<String, Option<String>> = FromSql::from_sql(&data).unwrap();
    assert!(map.is_empty());
}

#[test]
fn test_hstore_encode_single_pair() {
    let mut map = HashMap::new();
    map.insert("key".to_string(), Some("val".to_string()));
    let mut buf = BytesMut::new();
    map.to_sql(&mut buf).unwrap();

    // Decode it back
    let decoded: HashMap<String, Option<String>> = FromSql::from_sql(&buf).unwrap();
    assert_eq!(decoded.get("key"), Some(&Some("val".to_string())));
}

#[test]
fn test_hstore_encode_null_value() {
    let mut map = HashMap::new();
    map.insert("key".to_string(), None);
    let mut buf = BytesMut::new();
    map.to_sql(&mut buf).unwrap();

    let decoded: HashMap<String, Option<String>> = FromSql::from_sql(&buf).unwrap();
    assert_eq!(decoded.get("key"), Some(&None));
}

#[test]
fn test_hstore_roundtrip_multiple() {
    let mut map = HashMap::new();
    map.insert("a".to_string(), Some("1".to_string()));
    map.insert("b".to_string(), Some("2".to_string()));
    map.insert("c".to_string(), None);

    let mut buf = BytesMut::new();
    map.to_sql(&mut buf).unwrap();

    let decoded: HashMap<String, Option<String>> = FromSql::from_sql(&buf).unwrap();
    assert_eq!(decoded.len(), 3);
    assert_eq!(decoded.get("a"), Some(&Some("1".to_string())));
    assert_eq!(decoded.get("b"), Some(&Some("2".to_string())));
    assert_eq!(decoded.get("c"), Some(&None));
}

#[test]
fn test_hstore_decode_truncated() {
    let result: sentinel_driver::Result<HashMap<String, Option<String>>> = FromSql::from_sql(&[0]);
    assert!(result.is_err());
}
