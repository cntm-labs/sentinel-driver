use bytes::BytesMut;

use sentinel_driver::types::{FromSql, ToSql};

/// Helper: encode then decode, verify roundtrip.
fn roundtrip<T: ToSql + FromSql + std::fmt::Debug + PartialEq>(val: &T) {
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    let decoded = T::from_sql(&buf).unwrap();
    assert_eq!(&decoded, val);
}

#[test]
fn test_roundtrip_bool() {
    roundtrip(&true);
    roundtrip(&false);
}

#[test]
fn test_roundtrip_i16() {
    roundtrip(&0i16);
    roundtrip(&i16::MIN);
    roundtrip(&i16::MAX);
    roundtrip(&-42i16);
}

#[test]
fn test_roundtrip_i32() {
    roundtrip(&0i32);
    roundtrip(&i32::MIN);
    roundtrip(&i32::MAX);
    roundtrip(&42i32);
}

#[test]
fn test_roundtrip_i64() {
    roundtrip(&0i64);
    roundtrip(&i64::MIN);
    roundtrip(&i64::MAX);
}

#[test]
fn test_roundtrip_f32() {
    roundtrip(&0.0f32);
    roundtrip(&3.14f32);
    roundtrip(&-1.0f32);
}

#[test]
fn test_roundtrip_f64() {
    roundtrip(&0.0f64);
    roundtrip(&std::f64::consts::PI);
    roundtrip(&-1.0f64);
}

#[test]
fn test_roundtrip_string() {
    roundtrip(&String::from("hello world"));
    roundtrip(&String::from(""));
    roundtrip(&String::from("日本語テスト"));
}

#[test]
fn test_roundtrip_bytes() {
    roundtrip(&vec![0xDE, 0xAD, 0xBE, 0xEF]);
    roundtrip(&vec![]);
}

#[test]
fn test_roundtrip_uuid() {
    let id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    roundtrip(&id);
    roundtrip(&uuid::Uuid::nil());
}

#[test]
fn test_roundtrip_naive_date() {
    let date = chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
    roundtrip(&date);

    let date = chrono::NaiveDate::from_ymd_opt(2026, 4, 3).unwrap();
    roundtrip(&date);

    let date = chrono::NaiveDate::from_ymd_opt(1999, 12, 31).unwrap();
    roundtrip(&date);
}

#[test]
fn test_roundtrip_naive_datetime() {
    let dt = chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    roundtrip(&dt);

    let dt = chrono::NaiveDate::from_ymd_opt(2026, 4, 3)
        .unwrap()
        .and_hms_micro_opt(12, 30, 45, 123456)
        .unwrap();
    roundtrip(&dt);
}

#[test]
fn test_roundtrip_datetime_utc() {
    let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap();
    roundtrip(&dt);

    let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(1712150400, 500_000_000).unwrap();
    roundtrip(&dt);
}

#[test]
fn test_roundtrip_naive_time() {
    let t = chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap();
    roundtrip(&t);

    let t = chrono::NaiveTime::from_hms_micro_opt(23, 59, 59, 999999).unwrap();
    roundtrip(&t);
}

#[test]
fn test_decode_wrong_size() {
    assert!(i32::from_sql(&[0, 0]).is_err());
    assert!(bool::from_sql(&[]).is_err());
    assert!(uuid::Uuid::from_sql(&[0; 15]).is_err());
}

#[test]
fn test_option_from_sql() {
    let result: Option<i32> = FromSql::from_sql_nullable(None).unwrap();
    assert_eq!(result, None);

    let buf = 42i32.to_be_bytes();
    let result: Option<i32> = FromSql::from_sql_nullable(Some(&buf)).unwrap();
    assert_eq!(result, Some(42));
}
