use bytes::BytesMut;
use sentinel_driver::types::{FromSql, Oid, ToSql};

// ── OffsetDateTime (TIMESTAMPTZ) ────────────────────

#[test]
fn test_time_offsetdatetime_roundtrip() {
    let val = time::macros::datetime!(2026-04-17 12:30:00 UTC);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = time::OffsetDateTime::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_time_offsetdatetime_epoch() {
    // PG epoch: 2000-01-01 00:00:00 UTC should encode as 0
    let val = time::macros::datetime!(2000-01-01 0:00:00 UTC);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    assert_eq!(&buf[..], &0i64.to_be_bytes());
}

#[test]
fn test_time_offsetdatetime_before_epoch() {
    // 1999-12-31 23:59:59 UTC = -1_000_000 microseconds from PG epoch
    let val = time::macros::datetime!(1999-12-31 23:59:59 UTC);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = time::OffsetDateTime::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_time_offsetdatetime_with_micros() {
    let val = time::macros::datetime!(2026-04-17 12:30:00.123456 UTC);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = time::OffsetDateTime::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_time_offsetdatetime_oid() {
    let val = time::macros::datetime!(2026-04-17 0:00:00 UTC);
    assert_eq!(val.oid(), Oid::TIMESTAMPTZ);
    assert_eq!(<time::OffsetDateTime as FromSql>::oid(), Oid::TIMESTAMPTZ);
}

// ── PrimitiveDateTime (TIMESTAMP) ───────────────────

#[test]
fn test_time_primitivedatetime_roundtrip() {
    let val = time::macros::datetime!(2026-04-17 12:30:00);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = time::PrimitiveDateTime::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_time_primitivedatetime_epoch() {
    let val = time::macros::datetime!(2000-01-01 0:00:00);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    assert_eq!(&buf[..], &0i64.to_be_bytes());
}

#[test]
fn test_time_primitivedatetime_with_micros() {
    let val = time::macros::datetime!(2026-04-17 12:30:00.654321);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = time::PrimitiveDateTime::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_time_primitivedatetime_oid() {
    let val = time::macros::datetime!(2026-04-17 0:00:00);
    assert_eq!(val.oid(), Oid::TIMESTAMP);
    assert_eq!(<time::PrimitiveDateTime as FromSql>::oid(), Oid::TIMESTAMP);
}

// ── Date (DATE) ─────────────────────────────────────

#[test]
fn test_time_date_roundtrip() {
    let val = time::macros::date!(2026 - 04 - 17);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = time::Date::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_time_date_epoch() {
    // PG epoch: 2000-01-01 should encode as 0 days
    let val = time::macros::date!(2000 - 01 - 01);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    assert_eq!(&buf[..], &0i32.to_be_bytes());
}

#[test]
fn test_time_date_day_after_epoch() {
    let val = time::macros::date!(2000 - 01 - 02);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    assert_eq!(&buf[..], &1i32.to_be_bytes());
}

#[test]
fn test_time_date_before_epoch() {
    let val = time::macros::date!(1999 - 12 - 31);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    assert_eq!(&buf[..], &(-1i32).to_be_bytes());
}

#[test]
fn test_time_date_oid() {
    let val = time::macros::date!(2026 - 04 - 17);
    assert_eq!(val.oid(), Oid::DATE);
    assert_eq!(<time::Date as FromSql>::oid(), Oid::DATE);
}

// ── Time (TIME) ─────────────────────────────────────

#[test]
fn test_time_time_roundtrip() {
    let val = time::macros::time!(12:30:45.123456);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = time::Time::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_time_time_midnight() {
    let val = time::macros::time!(0:00:00);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    assert_eq!(&buf[..], &0i64.to_be_bytes());
}

#[test]
fn test_time_time_max() {
    let val = time::macros::time!(23:59:59.999999);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = time::Time::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_time_time_oid() {
    let val = time::macros::time!(12:00:00);
    assert_eq!(val.oid(), Oid::TIME);
    assert_eq!(<time::Time as FromSql>::oid(), Oid::TIME);
}

// ── Cross-compatibility with chrono ─────────────────

#[test]
fn test_time_date_same_wire_as_chrono() {
    // Both time::Date and chrono::NaiveDate for 2026-04-17 should produce the same bytes
    let time_date = time::macros::date!(2026 - 04 - 17);
    let chrono_date = chrono::NaiveDate::from_ymd_opt(2026, 4, 17).unwrap();

    let mut buf_time = BytesMut::new();
    let mut buf_chrono = BytesMut::new();
    time_date.to_sql(&mut buf_time).ok();
    chrono_date.to_sql(&mut buf_chrono).ok();

    assert_eq!(&buf_time[..], &buf_chrono[..]);
}

#[test]
fn test_time_timestamp_same_wire_as_chrono() {
    let time_dt = time::macros::datetime!(2026-04-17 12:30:00);
    let chrono_dt = chrono::NaiveDate::from_ymd_opt(2026, 4, 17)
        .unwrap()
        .and_hms_opt(12, 30, 0)
        .unwrap();

    let mut buf_time = BytesMut::new();
    let mut buf_chrono = BytesMut::new();
    time_dt.to_sql(&mut buf_time).ok();
    chrono_dt.to_sql(&mut buf_chrono).ok();

    assert_eq!(&buf_time[..], &buf_chrono[..]);
}

#[test]
fn test_time_timestamptz_same_wire_as_chrono() {
    let time_dt = time::macros::datetime!(2026-04-17 12:30:00 UTC);
    let chrono_dt = chrono::NaiveDate::from_ymd_opt(2026, 4, 17)
        .unwrap()
        .and_hms_opt(12, 30, 0)
        .unwrap()
        .and_utc();

    let mut buf_time = BytesMut::new();
    let mut buf_chrono = BytesMut::new();
    time_dt.to_sql(&mut buf_time).ok();
    chrono_dt.to_sql(&mut buf_chrono).ok();

    assert_eq!(&buf_time[..], &buf_chrono[..]);
}
