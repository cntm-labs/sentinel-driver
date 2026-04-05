use bytes::{BufMut, BytesMut};

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
    roundtrip(&vec![0xDE_u8, 0xAD, 0xBE, 0xEF]);
    roundtrip(&Vec::<u8>::new());
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
fn test_roundtrip_vec_i32() {
    roundtrip(&vec![1i32, 2, 3]);
    roundtrip(&vec![i32::MIN, 0, i32::MAX]);
}

#[test]
fn test_roundtrip_vec_empty_i32() {
    roundtrip(&Vec::<i32>::new());
}

#[test]
fn test_roundtrip_vec_i16() {
    roundtrip(&vec![1i16, -1, 0]);
}

#[test]
fn test_roundtrip_vec_i64() {
    roundtrip(&vec![1i64, i64::MAX]);
}

#[test]
fn test_roundtrip_vec_f32() {
    roundtrip(&vec![1.0f32, 3.14, -0.5]);
}

#[test]
fn test_roundtrip_vec_f64() {
    roundtrip(&vec![std::f64::consts::PI, 0.0]);
}

#[test]
fn test_roundtrip_vec_bool() {
    roundtrip(&vec![true, false, true]);
}

#[test]
fn test_roundtrip_vec_string() {
    roundtrip(&vec![String::from("hello"), String::from("world")]);
    roundtrip(&vec![String::from("")]);
}

#[test]
fn test_roundtrip_vec_uuid() {
    let id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    roundtrip(&vec![id, uuid::Uuid::nil()]);
}

#[test]
fn test_decode_array_multidim_rejected() {
    let mut buf = BytesMut::new();
    buf.put_i32(2); // ndim = 2 (not supported)
    buf.put_i32(0);
    buf.put_u32(sentinel_driver::types::Oid::INT4.0);
    buf.put_i32(2);
    buf.put_i32(1);
    buf.put_i32(2);
    buf.put_i32(1);

    let result = Vec::<i32>::from_sql(&buf);
    assert!(result.is_err());
}

#[test]
fn test_decode_array_header_too_short() {
    // Less than 12 bytes
    let buf = [0u8; 8];
    let result = Vec::<i32>::from_sql(&buf);
    assert!(result.is_err());
}

#[test]
fn test_decode_array_wrong_element_oid() {
    use sentinel_driver::types::Oid;

    let mut buf = BytesMut::new();
    buf.put_i32(1); // ndim = 1
    buf.put_i32(0); // has_null = 0
    buf.put_u32(Oid::TEXT.0); // wrong OID for Vec<i32>
    buf.put_i32(1); // dim_len = 1
    buf.put_i32(1); // dim_lbound = 1
    buf.put_i32(4); // elem_len = 4
    buf.put_i32(42); // element data

    let result = Vec::<i32>::from_sql(&buf);
    assert!(result.is_err());
}

#[test]
fn test_decode_array_dimension_header_too_short() {
    use sentinel_driver::types::Oid;

    // 12 bytes header but missing dimension info (need 20 for ndim=1)
    let mut buf = BytesMut::new();
    buf.put_i32(1); // ndim = 1
    buf.put_i32(0); // has_null = 0
    buf.put_u32(Oid::INT4.0);
    // Missing dim_len and dim_lbound

    let result = Vec::<i32>::from_sql(&buf);
    assert!(result.is_err());
}

#[test]
fn test_decode_array_element_data_truncated() {
    use sentinel_driver::types::Oid;

    let mut buf = BytesMut::new();
    buf.put_i32(1); // ndim = 1
    buf.put_i32(0); // has_null = 0
    buf.put_u32(Oid::INT4.0);
    buf.put_i32(1); // dim_len = 1
    buf.put_i32(1); // dim_lbound = 1
    buf.put_i32(4); // elem_len = 4
    buf.put_i16(0); // only 2 bytes instead of 4

    let result = Vec::<i32>::from_sql(&buf);
    assert!(result.is_err());
}

#[test]
fn test_decode_array_null_element_rejected() {
    use sentinel_driver::types::Oid;

    let mut buf = BytesMut::new();
    buf.put_i32(1); // ndim = 1
    buf.put_i32(1); // has_null = 1
    buf.put_u32(Oid::INT4.0);
    buf.put_i32(1); // dim_len = 1
    buf.put_i32(1); // dim_lbound = 1
    buf.put_i32(-1); // NULL element (len = -1)

    let result = Vec::<i32>::from_sql(&buf);
    assert!(result.is_err());
}

#[test]
fn test_decode_array_unexpected_end_of_data() {
    use sentinel_driver::types::Oid;

    let mut buf = BytesMut::new();
    buf.put_i32(1); // ndim = 1
    buf.put_i32(0); // has_null = 0
    buf.put_u32(Oid::INT4.0);
    buf.put_i32(2); // dim_len = 2 (claims 2 elements)
    buf.put_i32(1); // dim_lbound = 1
                    // No element data at all

    let result = Vec::<i32>::from_sql(&buf);
    assert!(result.is_err());
}

#[test]
fn test_array_from_sql_oid() {
    use sentinel_driver::types::Oid;

    assert_eq!(<Vec<bool> as FromSql>::oid(), Oid::BOOL_ARRAY);
    assert_eq!(<Vec<i16> as FromSql>::oid(), Oid::INT2_ARRAY);
    assert_eq!(<Vec<i32> as FromSql>::oid(), Oid::INT4_ARRAY);
    assert_eq!(<Vec<i64> as FromSql>::oid(), Oid::INT8_ARRAY);
    assert_eq!(<Vec<f32> as FromSql>::oid(), Oid::FLOAT4_ARRAY);
    assert_eq!(<Vec<f64> as FromSql>::oid(), Oid::FLOAT8_ARRAY);
    assert_eq!(<Vec<String> as FromSql>::oid(), Oid::TEXT_ARRAY);
    assert_eq!(<Vec<uuid::Uuid> as FromSql>::oid(), Oid::UUID_ARRAY);
}

#[test]
fn test_option_from_sql() {
    let result: Option<i32> = FromSql::from_sql_nullable(None).unwrap();
    assert_eq!(result, None);

    let buf = 42i32.to_be_bytes();
    let result: Option<i32> = FromSql::from_sql_nullable(Some(&buf)).unwrap();
    assert_eq!(result, Some(42));
}
