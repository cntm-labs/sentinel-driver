use bytes::BytesMut;

use sentinel_driver::types::network::PgMacAddr8;
use sentinel_driver::types::{FromSql, Oid, ToSql};

#[test]
fn test_macaddr8_oid() {
    let val = PgMacAddr8([0x08, 0x00, 0x2b, 0x01, 0x02, 0x03, 0x04, 0x05]);
    assert_eq!(val.oid(), Oid::MACADDR8);
    assert_eq!(<PgMacAddr8 as FromSql>::oid(), Oid::MACADDR8);
}

#[test]
fn test_macaddr8_roundtrip() {
    let val = PgMacAddr8([0x08, 0x00, 0x2b, 0x01, 0x02, 0x03, 0x04, 0x05]);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    assert_eq!(buf.len(), 8);
    let decoded = PgMacAddr8::from_sql(&buf).unwrap();
    assert_eq!(decoded, val);
}

#[test]
fn test_macaddr8_wire_format() {
    let val = PgMacAddr8([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22]);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    assert_eq!(&buf[..], &[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22]);
}

#[test]
fn test_macaddr8_decode_wrong_size() {
    let buf = [0u8; 6]; // 6 bytes, not 8
    assert!(PgMacAddr8::from_sql(&buf).is_err());
}

#[test]
fn test_macaddr8_all_zeros() {
    let val = PgMacAddr8([0; 8]);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    let decoded = PgMacAddr8::from_sql(&buf).unwrap();
    assert_eq!(decoded, val);
}

#[test]
fn test_macaddr8_array_oid() {
    assert_eq!(<Vec<PgMacAddr8> as FromSql>::oid(), Oid::MACADDR8_ARRAY);
}
