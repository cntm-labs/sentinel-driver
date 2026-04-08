use bytes::BytesMut;

use sentinel_driver::types::{encode_param, encode_param_nullable, FromSql, Oid, ToSql};

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

#[test]
fn test_option_some_to_sql() {
    let val: Option<i32> = Some(42);
    assert_eq!(val.oid(), Oid::INT4);

    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    assert_eq!(&buf[..], &42i32.to_be_bytes());
}

#[test]
fn test_option_none_to_sql() {
    let val: Option<i32> = None;
    assert_eq!(val.oid(), Oid::TEXT); // defaults to TEXT for NULL

    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    assert!(buf.is_empty());
}

#[test]
fn test_option_some_from_sql() {
    let data = 42i32.to_be_bytes();
    let val: Option<i32> = FromSql::from_sql(&data).unwrap();
    assert_eq!(val, Some(42));
}

#[test]
fn test_option_none_from_sql_nullable() {
    let val: Option<i32> = FromSql::from_sql_nullable(None).unwrap();
    assert_eq!(val, None);
}

#[test]
fn test_option_some_from_sql_nullable() {
    let data = 42i32.to_be_bytes();
    let val: Option<i32> = FromSql::from_sql_nullable(Some(&data)).unwrap();
    assert_eq!(val, Some(42));
}

#[test]
fn test_from_sql_nullable_null_error() {
    let result: sentinel_driver::Result<i32> = FromSql::from_sql_nullable(None);
    assert!(result.is_err());
}

#[test]
fn test_to_sql_vec() {
    let vec = 42i32.to_sql_vec().unwrap();
    assert_eq!(vec, 42i32.to_be_bytes().to_vec());
}

#[test]
fn test_encode_param() {
    let vec = encode_param(&42i32).unwrap();
    assert_eq!(vec, 42i32.to_be_bytes().to_vec());
}

#[test]
fn test_encode_param_nullable_some() {
    let val: Option<i32> = Some(42);
    let result = encode_param_nullable(&val).unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap(), 42i32.to_be_bytes().to_vec());
}

#[test]
fn test_encode_param_nullable_none() {
    let val: Option<i32> = None;
    let result = encode_param_nullable(&val).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_from_sql_nullable_some_value() {
    // Covers the default from_sql_nullable Some branch on the base FromSql trait
    let data = 42i32.to_be_bytes();
    let val: i32 = FromSql::from_sql_nullable(Some(&data[..])).unwrap();
    assert_eq!(val, 42);
}

#[test]
fn test_option_to_sql_vec_some() {
    let val: Option<i32> = Some(99);
    let vec = val.to_sql_vec().unwrap();
    assert_eq!(vec, 99i32.to_be_bytes().to_vec());
}

#[test]
fn test_option_to_sql_vec_none() {
    let val: Option<i32> = None;
    let vec = val.to_sql_vec().unwrap();
    assert!(vec.is_empty());
}

#[test]
fn test_option_from_sql_oid() {
    // Covers Option<T>::FromSql::oid() (lines 47-49)
    assert_eq!(<Option<i32> as FromSql>::oid(), Oid::INT4);
    assert_eq!(<Option<String> as FromSql>::oid(), Oid::TEXT);
}

#[test]
fn test_is_null_default_false() {
    let val = 42i32;
    assert!(!val.is_null());
}

#[test]
fn test_is_null_option_none() {
    let val: Option<i32> = None;
    assert!(val.is_null());
}

#[test]
fn test_is_null_option_some() {
    let val: Option<i32> = Some(42);
    assert!(!val.is_null());
}

#[test]
fn test_null_param_encodes_as_none() {
    // Simulate the fixed query_internal encoding logic
    let params: Vec<&(dyn sentinel_driver::ToSql + Sync)> = vec![&None::<i32>];
    let mut encoded: Vec<Option<Vec<u8>>> = Vec::new();

    for param in &params {
        if param.is_null() {
            encoded.push(None);
        } else {
            let mut buf = BytesMut::new();
            param.to_sql(&mut buf).unwrap();
            encoded.push(Some(buf.to_vec()));
        }
    }

    assert_eq!(encoded.len(), 1);
    assert!(
        encoded[0].is_none(),
        "NULL param must encode as None, not Some(empty)"
    );
}

#[test]
fn test_non_null_param_encodes_as_some() {
    let params: Vec<&(dyn sentinel_driver::ToSql + Sync)> = vec![&42i32];
    let mut encoded: Vec<Option<Vec<u8>>> = Vec::new();

    for param in &params {
        if param.is_null() {
            encoded.push(None);
        } else {
            let mut buf = BytesMut::new();
            param.to_sql(&mut buf).unwrap();
            encoded.push(Some(buf.to_vec()));
        }
    }

    assert_eq!(encoded.len(), 1);
    assert!(encoded[0].is_some());
    assert_eq!(encoded[0].as_ref().unwrap(), &42i32.to_be_bytes());
}
