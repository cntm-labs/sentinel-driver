use bytes::BytesMut;

use sentinel_driver::types::ToSql;

#[test]
fn test_encode_bool() {
    let mut buf = BytesMut::new();
    true.to_sql(&mut buf).unwrap();
    assert_eq!(&buf[..], &[1]);

    buf.clear();
    false.to_sql(&mut buf).unwrap();
    assert_eq!(&buf[..], &[0]);
}

#[test]
fn test_encode_i32() {
    let mut buf = BytesMut::new();
    42i32.to_sql(&mut buf).unwrap();
    assert_eq!(&buf[..], &42i32.to_be_bytes());
}

#[test]
fn test_encode_i64() {
    let mut buf = BytesMut::new();
    123456789i64.to_sql(&mut buf).unwrap();
    assert_eq!(&buf[..], &123456789i64.to_be_bytes());
}

#[test]
fn test_encode_f64() {
    let mut buf = BytesMut::new();
    3.14f64.to_sql(&mut buf).unwrap();
    assert_eq!(&buf[..], &3.14f64.to_be_bytes());
}

#[test]
fn test_encode_str() {
    let mut buf = BytesMut::new();
    "hello".to_sql(&mut buf).unwrap();
    assert_eq!(&buf[..], b"hello");
}

#[test]
fn test_encode_string() {
    let mut buf = BytesMut::new();
    String::from("world").to_sql(&mut buf).unwrap();
    assert_eq!(&buf[..], b"world");
}

#[test]
fn test_encode_bytes() {
    let mut buf = BytesMut::new();
    let data: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF];
    data.to_sql(&mut buf).unwrap();
    assert_eq!(&buf[..], &[0xDE, 0xAD, 0xBE, 0xEF]);
}

#[test]
fn test_encode_uuid() {
    let id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let mut buf = BytesMut::new();
    id.to_sql(&mut buf).unwrap();
    assert_eq!(&buf[..], id.as_bytes());
    assert_eq!(buf.len(), 16);
}

#[test]
fn test_encode_naive_date() {
    // 2000-01-01 should encode as 0
    let date = chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
    let mut buf = BytesMut::new();
    date.to_sql(&mut buf).unwrap();
    assert_eq!(&buf[..], &0i32.to_be_bytes());

    // 2000-01-02 should encode as 1
    buf.clear();
    let date = chrono::NaiveDate::from_ymd_opt(2000, 1, 2).unwrap();
    date.to_sql(&mut buf).unwrap();
    assert_eq!(&buf[..], &1i32.to_be_bytes());
}

#[test]
fn test_encode_timestamp() {
    // PG epoch: 2000-01-01 00:00:00 should encode as 0
    let dt = chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let mut buf = BytesMut::new();
    dt.to_sql(&mut buf).unwrap();
    assert_eq!(&buf[..], &0i64.to_be_bytes());
}
