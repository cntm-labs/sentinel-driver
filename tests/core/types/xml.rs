use bytes::BytesMut;
use sentinel_driver::types::xml::PgXml;
use sentinel_driver::types::{FromSql, Oid, ToSql};

#[test]
fn test_xml_roundtrip() {
    let val = PgXml("<root><item>hello</item></root>".to_string());
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgXml::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_xml_oid() {
    assert_eq!(PgXml(String::new()).oid(), Oid::XML);
    assert_eq!(<PgXml as FromSql>::oid(), Oid::XML);
}

#[test]
fn test_xml_decode_empty() {
    let decoded = PgXml::from_sql(&[]).ok();
    assert_eq!(decoded, Some(PgXml(String::new())));
}
