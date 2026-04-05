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
fn test_encode_vec_i32() {
    use sentinel_driver::types::Oid;

    let v: Vec<i32> = vec![1, 2, 3];
    let mut buf = BytesMut::new();
    v.to_sql(&mut buf).unwrap();

    // Verify OID
    assert_eq!(v.oid(), Oid::INT4_ARRAY);

    // Verify binary format header
    let ndim = i32::from_be_bytes(buf[0..4].try_into().unwrap());
    let has_null = i32::from_be_bytes(buf[4..8].try_into().unwrap());
    let elem_oid = u32::from_be_bytes(buf[8..12].try_into().unwrap());
    let dim_len = i32::from_be_bytes(buf[12..16].try_into().unwrap());
    let dim_lbound = i32::from_be_bytes(buf[16..20].try_into().unwrap());

    assert_eq!(ndim, 1);
    assert_eq!(has_null, 0);
    assert_eq!(elem_oid, Oid::INT4.0);
    assert_eq!(dim_len, 3);
    assert_eq!(dim_lbound, 1);
}

#[test]
fn test_encode_vec_empty() {
    let v: Vec<i32> = vec![];
    let mut buf = BytesMut::new();
    v.to_sql(&mut buf).unwrap();

    let ndim = i32::from_be_bytes(buf[0..4].try_into().unwrap());
    assert_eq!(ndim, 0);
    // Empty array: ndim=0, has_null=0, elem_oid
    assert_eq!(buf.len(), 12);
}

#[test]
fn test_encode_vec_string() {
    use sentinel_driver::types::Oid;

    let v: Vec<String> = vec!["hello".into(), "world".into()];
    let mut buf = BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(v.oid(), Oid::TEXT_ARRAY);
}

#[test]
fn test_encode_vec_bool() {
    use sentinel_driver::types::Oid;

    let v: Vec<bool> = vec![true, false, true];
    let mut buf = BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(v.oid(), Oid::BOOL_ARRAY);
}

#[test]
fn test_encode_vec_str_ref() {
    use sentinel_driver::types::Oid;

    let v: Vec<&str> = vec!["hello", "world"];
    let mut buf = BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(v.oid(), Oid::TEXT_ARRAY);

    // Verify header
    let ndim = i32::from_be_bytes(buf[0..4].try_into().unwrap());
    assert_eq!(ndim, 1);
    let dim_len = i32::from_be_bytes(buf[12..16].try_into().unwrap());
    assert_eq!(dim_len, 2);
}

#[test]
fn test_encode_vec_i16() {
    use sentinel_driver::types::Oid;

    let v: Vec<i16> = vec![1, -1, 0];
    let mut buf = BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(v.oid(), Oid::INT2_ARRAY);
}

#[test]
fn test_encode_vec_i64() {
    use sentinel_driver::types::Oid;

    let v: Vec<i64> = vec![1, i64::MAX];
    let mut buf = BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(v.oid(), Oid::INT8_ARRAY);
}

#[test]
fn test_encode_vec_f32() {
    use sentinel_driver::types::Oid;

    let v: Vec<f32> = vec![1.0, 3.14, -0.5];
    let mut buf = BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(v.oid(), Oid::FLOAT4_ARRAY);
}

#[test]
fn test_encode_vec_f64() {
    use sentinel_driver::types::Oid;

    let v: Vec<f64> = vec![std::f64::consts::PI, 0.0];
    let mut buf = BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(v.oid(), Oid::FLOAT8_ARRAY);
}

#[test]
fn test_encode_vec_uuid() {
    use sentinel_driver::types::Oid;

    let id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let v: Vec<uuid::Uuid> = vec![id, uuid::Uuid::nil()];
    let mut buf = BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(v.oid(), Oid::UUID_ARRAY);
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
