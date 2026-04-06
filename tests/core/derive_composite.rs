use bytes::BytesMut;
use sentinel_driver::types::{FromSql, ToSql};

#[derive(Debug, PartialEq, sentinel_driver::ToSql, sentinel_driver::FromSql)]
#[sentinel(type_name = "address")]
struct Address {
    street_number: i32,
    zip_code: i32,
}

#[test]
fn test_composite_roundtrip() {
    let val = Address {
        street_number: 123,
        zip_code: 90210,
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();

    let decoded = Address::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_composite_wire_format() {
    let val = Address {
        street_number: 1,
        zip_code: 2,
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();

    // field_count(4) + [oid(4) + len(4) + data(4)] * 2
    assert_eq!(buf.len(), 4 + (4 + 4 + 4) * 2);

    // field_count = 2
    assert_eq!(&buf[0..4], &2i32.to_be_bytes());
}
