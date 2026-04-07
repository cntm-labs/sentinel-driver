use bytes::BytesMut;
use sentinel_driver::types::lsn::PgLsn;
use sentinel_driver::types::{FromSql, Oid, ToSql};

#[test]
fn test_lsn_roundtrip() {
    let val = PgLsn(0x0000_0001_0000_0000);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgLsn::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_lsn_zero() {
    let val = PgLsn(0);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgLsn::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_lsn_oid() {
    assert_eq!(PgLsn(0).oid(), Oid::PG_LSN);
    assert_eq!(<PgLsn as FromSql>::oid(), Oid::PG_LSN);
}

#[test]
fn test_lsn_display() {
    let lsn = PgLsn(0x0000_0001_0000_00A0);
    let s = lsn.to_string();
    assert_eq!(s, "1/A0");
}

#[test]
fn test_lsn_wire_format() {
    let mut buf = BytesMut::new();
    PgLsn(42).to_sql(&mut buf).ok();
    assert_eq!(buf.len(), 8);
    assert_eq!(&buf[..], &42i64.to_be_bytes());
}

#[test]
fn test_lsn_decode_wrong_size() {
    // covers lsn.rs line 32: map_err for wrong byte count
    assert!(PgLsn::from_sql(&[0u8; 4]).is_err());
    assert!(PgLsn::from_sql(&[0u8; 10]).is_err());
}
