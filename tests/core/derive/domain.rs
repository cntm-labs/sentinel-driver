use bytes::BytesMut;
use sentinel_driver::types::{FromSql, Oid, ToSql};

/// Domain type wrapping String — models `CREATE DOMAIN email AS text`.
#[derive(Debug, PartialEq, sentinel_driver::ToSql, sentinel_driver::FromSql)]
struct Email(String);

/// Domain type wrapping i32 — models `CREATE DOMAIN user_id AS integer`.
#[derive(Debug, PartialEq, sentinel_driver::ToSql, sentinel_driver::FromSql)]
struct UserId(i32);

/// Domain type wrapping f64 — models `CREATE DOMAIN temperature AS float8`.
#[derive(Debug, PartialEq, sentinel_driver::ToSql, sentinel_driver::FromSql)]
struct Temperature(f64);

/// Domain type wrapping bool.
#[derive(Debug, PartialEq, sentinel_driver::ToSql, sentinel_driver::FromSql)]
struct IsActive(bool);

// ── Roundtrip tests ─────────────────────────────────

#[test]
fn test_domain_email_roundtrip() {
    let val = Email("user@example.com".into());
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = Email::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_domain_user_id_roundtrip() {
    let val = UserId(42);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = UserId::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_domain_temperature_roundtrip() {
    let val = Temperature(36.6);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = Temperature::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_domain_bool_roundtrip() {
    let val = IsActive(true);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = IsActive::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

// ── OID delegation tests ────────────────────────────

#[test]
fn test_domain_email_oid() {
    let val = Email("test@test.com".into());
    assert_eq!(val.oid(), Oid::TEXT);
    assert_eq!(<Email as FromSql>::oid(), Oid::TEXT);
}

#[test]
fn test_domain_user_id_oid() {
    let val = UserId(1);
    assert_eq!(val.oid(), Oid::INT4);
    assert_eq!(<UserId as FromSql>::oid(), Oid::INT4);
}

#[test]
fn test_domain_temperature_oid() {
    let val = Temperature(0.0);
    assert_eq!(val.oid(), Oid::FLOAT8);
    assert_eq!(<Temperature as FromSql>::oid(), Oid::FLOAT8);
}

// ── Wire format tests ───────────────────────────────

#[test]
fn test_domain_email_wire_format() {
    let val = Email("hi".into());
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    // Should encode same as raw String
    assert_eq!(&buf[..], b"hi");
}

#[test]
fn test_domain_user_id_wire_format() {
    let val = UserId(7);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    assert_eq!(&buf[..], &7i32.to_be_bytes());
}

// ── Edge cases ──────────────────────────────────────

#[test]
fn test_domain_email_empty_string() {
    let val = Email(String::new());
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = Email::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_domain_user_id_negative() {
    let val = UserId(-1);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = UserId::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_domain_user_id_zero() {
    let val = UserId(0);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = UserId::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}
