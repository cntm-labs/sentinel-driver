use bytes::BytesMut;

use sentinel_driver::types::json::Json;
use sentinel_driver::types::{FromSql, Oid, ToSql};

#[test]
fn test_json_oid() {
    let val = Json(serde_json::json!({"key": "value"}));
    assert_eq!(val.oid(), Oid::JSONB);
    assert_eq!(<Json<serde_json::Value> as FromSql>::oid(), Oid::JSONB);
}

#[test]
fn test_json_roundtrip_object() {
    let val = Json(serde_json::json!({"name": "Alice", "age": 30}));
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();

    // First byte should be JSONB version = 1
    assert_eq!(buf[0], 1);

    let decoded: Json<serde_json::Value> = Json::from_sql(&buf).unwrap();
    assert_eq!(decoded.0["name"], "Alice");
    assert_eq!(decoded.0["age"], 30);
}

#[test]
fn test_json_roundtrip_array() {
    let val = Json(serde_json::json!([1, 2, 3]));
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    let decoded: Json<Vec<i32>> = Json::from_sql(&buf).unwrap();
    assert_eq!(decoded.0, vec![1, 2, 3]);
}

#[test]
fn test_json_roundtrip_string() {
    let val = Json("hello".to_string());
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    let decoded: Json<String> = Json::from_sql(&buf).unwrap();
    assert_eq!(decoded.0, "hello");
}

#[test]
fn test_json_roundtrip_struct() {
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct User {
        name: String,
        active: bool,
    }

    let val = Json(User {
        name: "Bob".into(),
        active: true,
    });
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    let decoded: Json<User> = Json::from_sql(&buf).unwrap();
    assert_eq!(decoded.0, val.0);
}

#[test]
fn test_json_decode_without_version_byte() {
    // Some servers may send JSON without the JSONB version prefix
    let raw = br#"{"key":"value"}"#;
    let decoded: Json<serde_json::Value> = Json::from_sql(raw).unwrap();
    assert_eq!(decoded.0["key"], "value");
}

#[test]
fn test_json_encode_wire_format() {
    let val = Json(42i32);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    // JSONB version 1 + "42"
    assert_eq!(&buf[..], &[1, b'4', b'2']);
}

#[test]
fn test_json_null_value() {
    let val = Json(serde_json::Value::Null);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    let decoded: Json<serde_json::Value> = Json::from_sql(&buf).unwrap();
    assert!(decoded.0.is_null());
}
