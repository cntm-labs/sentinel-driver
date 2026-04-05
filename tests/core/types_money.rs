use bytes::BytesMut;
use sentinel_driver::types::money::PgMoney;
use sentinel_driver::types::{FromSql, Oid, ToSql};

fn roundtrip(val: &PgMoney) {
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgMoney::from_sql(&buf).ok();
    assert_eq!(decoded, Some(*val));
}

#[test]
fn test_money_zero() {
    roundtrip(&PgMoney(0));
}

#[test]
fn test_money_positive() {
    roundtrip(&PgMoney(12345)); // $123.45
}

#[test]
fn test_money_negative() {
    roundtrip(&PgMoney(-9999));
}

#[test]
fn test_money_large() {
    roundtrip(&PgMoney(i64::MAX));
}

#[test]
fn test_money_oid() {
    assert_eq!(PgMoney(0).oid(), Oid::MONEY);
    assert_eq!(<PgMoney as FromSql>::oid(), Oid::MONEY);
}

#[test]
fn test_money_wire_format() {
    let mut buf = BytesMut::new();
    PgMoney(12345).to_sql(&mut buf).ok();
    assert_eq!(buf.len(), 8);
    assert_eq!(&buf[..], &12345i64.to_be_bytes());
}
