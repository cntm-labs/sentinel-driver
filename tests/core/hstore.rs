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

#[test]
fn test_hstore_decode_negative_count() {
    let data = (-1i32).to_be_bytes();
    let result: sentinel_driver::Result<HashMap<String, Option<String>>> = FromSql::from_sql(&data);
    assert!(result.is_err());
}

#[test]
fn test_hstore_decode_truncated_key_length() {
    // count=1 but no key length bytes after
    let mut data = Vec::new();
    data.extend_from_slice(&1i32.to_be_bytes()); // count = 1
    let result: sentinel_driver::Result<HashMap<String, Option<String>>> = FromSql::from_sql(&data);
    assert!(result.is_err());
}

#[test]
fn test_hstore_decode_truncated_key_data() {
    let mut data = Vec::new();
    data.extend_from_slice(&1i32.to_be_bytes()); // count = 1
    data.extend_from_slice(&10i32.to_be_bytes()); // key_len = 10
    data.extend_from_slice(b"short"); // only 5 bytes, need 10
    let result: sentinel_driver::Result<HashMap<String, Option<String>>> = FromSql::from_sql(&data);
    assert!(result.is_err());
}

#[test]
fn test_hstore_decode_truncated_value_length() {
    let mut data = Vec::new();
    data.extend_from_slice(&1i32.to_be_bytes()); // count = 1
    data.extend_from_slice(&3i32.to_be_bytes()); // key_len = 3
    data.extend_from_slice(b"key"); // key data
                                    // missing value length
    let result: sentinel_driver::Result<HashMap<String, Option<String>>> = FromSql::from_sql(&data);
    assert!(result.is_err());
}

#[test]
fn test_hstore_decode_truncated_value_data() {
    let mut data = Vec::new();
    data.extend_from_slice(&1i32.to_be_bytes()); // count = 1
    data.extend_from_slice(&3i32.to_be_bytes()); // key_len = 3
    data.extend_from_slice(b"key"); // key data
    data.extend_from_slice(&10i32.to_be_bytes()); // val_len = 10
    data.extend_from_slice(b"short"); // only 5 bytes
    let result: sentinel_driver::Result<HashMap<String, Option<String>>> = FromSql::from_sql(&data);
    assert!(result.is_err());
}

#[test]
fn test_hstore_oid() {
    let map: HashMap<String, Option<String>> = HashMap::new();
    assert_eq!(map.oid(), sentinel_driver::Oid::TEXT);
}

#[test]
fn test_hstore_from_sql_oid() {
    assert_eq!(
        <HashMap<String, Option<String>> as FromSql>::oid(),
        sentinel_driver::Oid::TEXT
    );
}

#[test]
fn test_hstore_decode_negative_key_length() {
    let mut data = Vec::new();
    data.extend_from_slice(&1i32.to_be_bytes()); // count = 1
    data.extend_from_slice(&(-1i32).to_be_bytes()); // key_len = -1 (invalid)
    let result: sentinel_driver::Result<HashMap<String, Option<String>>> = FromSql::from_sql(&data);
    assert!(result.is_err());
}

#[test]
fn test_hstore_decode_invalid_utf8_key() {
    let mut data = Vec::new();
    data.extend_from_slice(&1i32.to_be_bytes()); // count = 1
    data.extend_from_slice(&2i32.to_be_bytes()); // key_len = 2
    data.extend_from_slice(&[0xFF, 0xFE]); // invalid UTF-8
    data.extend_from_slice(&(-1i32).to_be_bytes()); // val = NULL
    let result: sentinel_driver::Result<HashMap<String, Option<String>>> = FromSql::from_sql(&data);
    assert!(result.is_err());
}

#[test]
fn test_hstore_decode_invalid_utf8_value() {
    let mut data = Vec::new();
    data.extend_from_slice(&1i32.to_be_bytes()); // count = 1
    data.extend_from_slice(&3i32.to_be_bytes()); // key_len = 3
    data.extend_from_slice(b"key"); // valid key
    data.extend_from_slice(&2i32.to_be_bytes()); // val_len = 2
    data.extend_from_slice(&[0xFF, 0xFE]); // invalid UTF-8 value
    let result: sentinel_driver::Result<HashMap<String, Option<String>>> = FromSql::from_sql(&data);
    assert!(result.is_err());
}
