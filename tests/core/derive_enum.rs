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
