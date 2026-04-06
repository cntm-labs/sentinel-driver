use bytes::BytesMut;
use sentinel_driver::types::interval::PgInterval;
use sentinel_driver::types::{FromSql, ToSql};

fn roundtrip(val: &PgInterval) {
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgInterval::from_sql(&buf).ok();
    assert_eq!(decoded, Some(*val));
}

#[test]
fn test_interval_zero() {
    roundtrip(&PgInterval {
        months: 0,
        days: 0,
        microseconds: 0,
    });
}

#[test]
fn test_interval_one_month() {
    roundtrip(&PgInterval {
        months: 1,
        days: 0,
        microseconds: 0,
    });
}

#[test]
fn test_interval_complex() {
    // 1 year, 2 months, 3 days, 4 hours, 5 minutes, 6 seconds
    roundtrip(&PgInterval {
        months: 14,
        days: 3,
        microseconds: 14_706_000_000,
    });
}

#[test]
fn test_interval_negative() {
    roundtrip(&PgInterval {
        months: -6,
        days: -15,
        microseconds: -3_600_000_000,
    });
}

#[test]
fn test_interval_encode_wire_format() {
    let mut buf = BytesMut::new();
    let val = PgInterval {
        months: 2,
        days: 10,
        microseconds: 3_600_000_000, // 1 hour
    };
    val.to_sql(&mut buf).ok();
    // PG binary: microseconds(i64) + days(i32) + months(i32) = 16 bytes
    assert_eq!(buf.len(), 16);
    // microseconds in BE
    assert_eq!(&buf[0..8], &3_600_000_000i64.to_be_bytes());
    // days in BE
    assert_eq!(&buf[8..12], &10i32.to_be_bytes());
    // months in BE
    assert_eq!(&buf[12..16], &2i32.to_be_bytes());
}

#[test]
fn test_interval_decode_too_short() {
    let buf = [0u8; 10];
    assert!(PgInterval::from_sql(&buf).is_err());
}

#[test]
fn test_interval_oid() {
    use sentinel_driver::types::Oid;
    let val = PgInterval {
        months: 0,
        days: 0,
        microseconds: 0,
    };
    assert_eq!(val.oid(), Oid::INTERVAL);
    assert_eq!(<PgInterval as FromSql>::oid(), Oid::INTERVAL);
}
