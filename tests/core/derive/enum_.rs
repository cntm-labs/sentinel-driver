use bytes::BytesMut;
use sentinel_driver::types::{FromSql, ToSql};

#[derive(Debug, PartialEq, sentinel_driver::ToSql, sentinel_driver::FromSql)]
#[sentinel(rename_all = "lowercase")]
enum Mood {
    Happy,
    Sad,
    #[sentinel(rename = "meh")]
    Neutral,
}

#[test]
fn test_enum_to_sql() {
    let mut buf = BytesMut::new();
    Mood::Happy.to_sql(&mut buf).ok();
    assert_eq!(&buf[..], b"happy");

    buf.clear();
    Mood::Neutral.to_sql(&mut buf).ok();
    assert_eq!(&buf[..], b"meh");
}

#[test]
fn test_enum_from_sql() {
    let decoded = Mood::from_sql(b"happy").ok();
    assert_eq!(decoded, Some(Mood::Happy));

    let decoded = Mood::from_sql(b"meh").ok();
    assert_eq!(decoded, Some(Mood::Neutral));
}

#[test]
fn test_enum_from_sql_unknown() {
    assert!(Mood::from_sql(b"angry").is_err());
}

#[test]
fn test_enum_roundtrip() {
    let mut buf = BytesMut::new();
    Mood::Sad.to_sql(&mut buf).ok();
    let decoded = Mood::from_sql(&buf).ok();
    assert_eq!(decoded, Some(Mood::Sad));
}

// ── Integer-repr enum tests ──────────────────────────

#[derive(Debug, PartialEq, Clone, Copy, sentinel_driver::ToSql, sentinel_driver::FromSql)]
#[repr(i32)]
enum Status {
    Pending = 0,
    Active = 1,
    Suspended = 2,
}

#[test]
fn test_repr_enum_to_sql() {
    let mut buf = BytesMut::new();
    Status::Active.to_sql(&mut buf).ok();
    assert_eq!(&buf[..], &1i32.to_be_bytes());
}

#[test]
fn test_repr_enum_from_sql() {
    let decoded = Status::from_sql(&2i32.to_be_bytes()).ok();
    assert_eq!(decoded, Some(Status::Suspended));
}

#[test]
fn test_repr_enum_roundtrip() {
    let mut buf = BytesMut::new();
    Status::Pending.to_sql(&mut buf).ok();
    let decoded = Status::from_sql(&buf).ok();
    assert_eq!(decoded, Some(Status::Pending));
}

#[test]
fn test_repr_enum_unknown_discriminant() {
    assert!(Status::from_sql(&99i32.to_be_bytes()).is_err());
}

// ── allow_mismatch tests ─────────────────────────────

#[derive(Debug, PartialEq, sentinel_driver::FromSql)]
#[sentinel(rename_all = "lowercase", allow_mismatch)]
enum Color {
    Red,
    Blue,
}

#[test]
fn test_allow_mismatch_known_variant() {
    let decoded = Color::from_sql(b"red").ok();
    assert_eq!(decoded, Some(Color::Red));
}

#[test]
fn test_allow_mismatch_unknown_falls_back_to_first() {
    let decoded = Color::from_sql(b"green").ok();
    assert_eq!(decoded, Some(Color::Red));
}
