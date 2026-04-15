use bytes::BytesMut;

use sentinel_driver::types::timetz::PgTimeTz;
use sentinel_driver::types::{FromSql, Oid, ToSql};

#[test]
fn test_timetz_oid() {
    let val = PgTimeTz {
        time: chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        offset_seconds: 0,
    };
    assert_eq!(val.oid(), Oid::TIMETZ);
    assert_eq!(<PgTimeTz as FromSql>::oid(), Oid::TIMETZ);
}

#[test]
fn test_timetz_roundtrip_utc() {
    let val = PgTimeTz {
        time: chrono::NaiveTime::from_hms_micro_opt(14, 30, 45, 123456).unwrap(),
        offset_seconds: 0,
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    assert_eq!(buf.len(), 12); // i64 + i32
    let decoded = PgTimeTz::from_sql(&buf).unwrap();
    assert_eq!(decoded, val);
}

#[test]
fn test_timetz_roundtrip_positive_offset() {
    // UTC+7 (Bangkok)
    let val = PgTimeTz {
        time: chrono::NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
        offset_seconds: 25200, // +7 hours
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    let decoded = PgTimeTz::from_sql(&buf).unwrap();
    assert_eq!(decoded, val);
}

#[test]
fn test_timetz_roundtrip_negative_offset() {
    // UTC-5 (Eastern)
    let val = PgTimeTz {
        time: chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
        offset_seconds: -18000, // -5 hours
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    let decoded = PgTimeTz::from_sql(&buf).unwrap();
    assert_eq!(decoded, val);
}

#[test]
fn test_timetz_roundtrip_midnight() {
    let val = PgTimeTz {
        time: chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        offset_seconds: 0,
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    let decoded = PgTimeTz::from_sql(&buf).unwrap();
    assert_eq!(decoded, val);
}

#[test]
fn test_timetz_wire_format() {
    let val = PgTimeTz {
        time: chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        offset_seconds: 3600, // UTC+1
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();

    // Time = 0 microseconds
    assert_eq!(&buf[0..8], &0i64.to_be_bytes());
    // Offset = -3600 (negated)
    assert_eq!(&buf[8..12], &(-3600i32).to_be_bytes());
}

#[test]
fn test_timetz_decode_wrong_size() {
    let buf = [0u8; 8]; // too short
    assert!(PgTimeTz::from_sql(&buf).is_err());
}

#[test]
fn test_timetz_array_oid() {
    assert_eq!(<Vec<PgTimeTz> as FromSql>::oid(), Oid::TIMETZ_ARRAY);
}
