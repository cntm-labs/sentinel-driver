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
fn test_roundtrip_timestamp_infinity() {
    roundtrip(&chrono::NaiveDateTime::MAX);
    roundtrip(&chrono::NaiveDateTime::MIN);
}

#[test]
fn test_roundtrip_timestamptz_infinity() {
    roundtrip(&chrono::NaiveDateTime::MAX.and_utc());
    roundtrip(&chrono::NaiveDateTime::MIN.and_utc());
}

#[test]
fn test_roundtrip_date_infinity() {
    roundtrip(&chrono::NaiveDate::MAX);
    roundtrip(&chrono::NaiveDate::MIN);
}

#[test]
fn test_decode_timestamp_positive_infinity() {
    let decoded = chrono::NaiveDateTime::from_sql(&i64::MAX.to_be_bytes()).unwrap();
    assert_eq!(decoded, chrono::NaiveDateTime::MAX);
}

#[test]
fn test_decode_timestamp_negative_infinity() {
    let decoded = chrono::NaiveDateTime::from_sql(&i64::MIN.to_be_bytes()).unwrap();
    assert_eq!(decoded, chrono::NaiveDateTime::MIN);
}

#[test]
fn test_decode_date_positive_infinity() {
    let decoded = chrono::NaiveDate::from_sql(&i32::MAX.to_be_bytes()).unwrap();
    assert_eq!(decoded, chrono::NaiveDate::MAX);
}

#[test]
fn test_decode_date_negative_infinity() {
    let decoded = chrono::NaiveDate::from_sql(&i32::MIN.to_be_bytes()).unwrap();
    assert_eq!(decoded, chrono::NaiveDate::MIN);
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
fn test_roundtrip_vec_naive_datetime() {
    let dt1 = chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let dt2 = chrono::NaiveDate::from_ymd_opt(2026, 4, 15)
        .unwrap()
        .and_hms_micro_opt(12, 30, 45, 123456)
        .unwrap();
    roundtrip(&vec![dt1, dt2]);
    roundtrip(&Vec::<chrono::NaiveDateTime>::new());
}

#[test]
fn test_roundtrip_vec_datetime_utc() {
    let dt1 = chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap();
    let dt2 = chrono::DateTime::<chrono::Utc>::from_timestamp(1712150400, 500_000_000).unwrap();
    roundtrip(&vec![dt1, dt2]);
}

#[test]
fn test_roundtrip_vec_naive_date() {
    let d1 = chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
    let d2 = chrono::NaiveDate::from_ymd_opt(2026, 4, 15).unwrap();
    roundtrip(&vec![d1, d2]);
}

#[test]
fn test_roundtrip_vec_naive_time() {
    let t1 = chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap();
    let t2 = chrono::NaiveTime::from_hms_micro_opt(23, 59, 59, 999999).unwrap();
    roundtrip(&vec![t1, t2]);
}

#[test]
fn test_roundtrip_vec_bytea() {
    let v: Vec<Vec<u8>> = vec![vec![0xDE, 0xAD], vec![0xBE, 0xEF]];
    roundtrip(&v);
    roundtrip(&Vec::<Vec<u8>>::new());
}

#[test]
fn test_roundtrip_vec_point() {
    use sentinel_driver::types::geometric::PgPoint;
    let v = vec![PgPoint { x: 1.0, y: 2.0 }, PgPoint { x: -3.5, y: 4.5 }];
    roundtrip(&v);
}

#[test]
fn test_roundtrip_vec_circle() {
    use sentinel_driver::types::geometric::{PgCircle, PgPoint};
    let v = vec![PgCircle {
        center: PgPoint { x: 0.0, y: 0.0 },
        radius: 5.0,
    }];
    roundtrip(&v);
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
    assert_eq!(
        <Vec<chrono::NaiveDateTime> as FromSql>::oid(),
        Oid::TIMESTAMP_ARRAY
    );
    assert_eq!(
        <Vec<chrono::DateTime<chrono::Utc>> as FromSql>::oid(),
        Oid::TIMESTAMPTZ_ARRAY
    );
    assert_eq!(<Vec<chrono::NaiveDate> as FromSql>::oid(), Oid::DATE_ARRAY);
    assert_eq!(<Vec<chrono::NaiveTime> as FromSql>::oid(), Oid::TIME_ARRAY);
    assert_eq!(<Vec<Vec<u8>> as FromSql>::oid(), Oid::BYTEA_ARRAY);
    assert_eq!(
        <Vec<sentinel_driver::types::geometric::PgPoint> as FromSql>::oid(),
        Oid::POINT_ARRAY
    );
    assert_eq!(
        <Vec<sentinel_driver::types::geometric::PgCircle> as FromSql>::oid(),
        Oid::CIRCLE_ARRAY
    );
}

#[test]
fn test_option_from_sql() {
    let result: Option<i32> = FromSql::from_sql_nullable(None).unwrap();
    assert_eq!(result, None);

    let buf = 42i32.to_be_bytes();
    let result: Option<i32> = FromSql::from_sql_nullable(Some(&buf)).unwrap();
    assert_eq!(result, Some(42));
}

// ---------------------------------------------------------------------------
// Nullable array decoding — `Vec<Option<T>>` (issue #33)
// ---------------------------------------------------------------------------

/// Build a one-dimensional PG binary-array body for the given element OID.
///
/// Each element is either `Some(bytes)` (a non-NULL element body) or
/// `None` (SQL NULL, encoded as a per-element length of `-1`).
fn build_pg_array(elem_oid: u32, elems: &[Option<&[u8]>]) -> Vec<u8> {
    let mut out = Vec::new();
    let has_null = elems.iter().any(Option::is_none) as i32;
    out.extend_from_slice(&1i32.to_be_bytes()); // ndim
    out.extend_from_slice(&has_null.to_be_bytes());
    out.extend_from_slice(&elem_oid.to_be_bytes());
    out.extend_from_slice(&(elems.len() as i32).to_be_bytes()); // dim_len
    out.extend_from_slice(&1i32.to_be_bytes()); // dim_lbound
    for e in elems {
        match e {
            Some(body) => {
                out.extend_from_slice(&(body.len() as i32).to_be_bytes());
                out.extend_from_slice(body);
            }
            None => {
                out.extend_from_slice(&(-1i32).to_be_bytes());
            }
        }
    }
    out
}

#[test]
fn test_decode_vec_option_i32_with_null() {
    let one = 1i32.to_be_bytes();
    let three = 3i32.to_be_bytes();
    let buf = build_pg_array(
        sentinel_driver::Oid::INT4.0,
        &[Some(&one), None, Some(&three)],
    );
    let got = <Vec<Option<i32>> as FromSql>::from_sql(&buf).unwrap();
    assert_eq!(got, vec![Some(1), None, Some(3)]);
}

#[test]
fn test_decode_vec_option_i32_no_nulls() {
    let a = 7i32.to_be_bytes();
    let b = 8i32.to_be_bytes();
    let buf = build_pg_array(sentinel_driver::Oid::INT4.0, &[Some(&a), Some(&b)]);
    let got = <Vec<Option<i32>> as FromSql>::from_sql(&buf).unwrap();
    assert_eq!(got, vec![Some(7), Some(8)]);
}

#[test]
fn test_decode_vec_option_i32_all_null() {
    let buf = build_pg_array(sentinel_driver::Oid::INT4.0, &[None, None, None]);
    let got = <Vec<Option<i32>> as FromSql>::from_sql(&buf).unwrap();
    assert_eq!(got, vec![None, None, None]);
}

#[test]
fn test_decode_vec_option_empty() {
    // ndim=0 — short-circuit path returns empty Vec without touching the
    // header beyond the first 12 bytes.
    let mut buf = Vec::new();
    buf.extend_from_slice(&0i32.to_be_bytes()); // ndim = 0
    buf.extend_from_slice(&0i32.to_be_bytes()); // has_null
    buf.extend_from_slice(&sentinel_driver::Oid::INT4.0.to_be_bytes());
    let got = <Vec<Option<i32>> as FromSql>::from_sql(&buf).unwrap();
    assert!(got.is_empty());
}

#[test]
fn test_decode_vec_i32_rejects_null_with_helpful_message() {
    let one = 1i32.to_be_bytes();
    let buf = build_pg_array(sentinel_driver::Oid::INT4.0, &[Some(&one), None]);
    let err = <Vec<i32> as FromSql>::from_sql(&buf).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("NULL"), "expected NULL error, got: {msg}");
    assert!(
        msg.contains("Vec<Option<T>>"),
        "expected suggestion to use Vec<Option<T>>, got: {msg}"
    );
}

#[test]
fn test_decode_vec_option_string_with_null() {
    let hello = b"hello";
    let world = b"world";
    let buf = build_pg_array(
        sentinel_driver::Oid::TEXT.0,
        &[Some(hello), None, Some(world)],
    );
    let got = <Vec<Option<String>> as FromSql>::from_sql(&buf).unwrap();
    assert_eq!(
        got,
        vec![Some(String::from("hello")), None, Some(String::from("world"))]
    );
}

#[test]
fn test_decode_vec_option_bool_with_null() {
    let t = [1u8];
    let f = [0u8];
    let buf = build_pg_array(
        sentinel_driver::Oid::BOOL.0,
        &[Some(&t), None, Some(&f)],
    );
    let got = <Vec<Option<bool>> as FromSql>::from_sql(&buf).unwrap();
    assert_eq!(got, vec![Some(true), None, Some(false)]);
}

#[test]
fn test_decode_vec_option_bytea_with_null() {
    let blob = &[0xde, 0xad, 0xbe, 0xef][..];
    let buf = build_pg_array(sentinel_driver::Oid::BYTEA.0, &[Some(blob), None]);
    let got = <Vec<Option<Vec<u8>>> as FromSql>::from_sql(&buf).unwrap();
    assert_eq!(got, vec![Some(blob.to_vec()), None]);
}

#[test]
fn test_decode_vec_option_oid_matches_array_oid() {
    // Vec<Option<i32>> must advertise the same array OID as Vec<i32> so
    // server-side type checks accept either at a parameter slot.
    assert_eq!(
        <Vec<Option<i32>> as FromSql>::oid(),
        <Vec<i32> as FromSql>::oid()
    );
    assert_eq!(
        <Vec<Option<String>> as FromSql>::oid(),
        <Vec<String> as FromSql>::oid()
    );
    assert_eq!(
        <Vec<Option<Vec<u8>>> as FromSql>::oid(),
        <Vec<Vec<u8>> as FromSql>::oid()
    );
}
