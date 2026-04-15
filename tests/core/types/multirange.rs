use bytes::BytesMut;

use sentinel_driver::types::multirange::PgMultirange;
use sentinel_driver::types::range::{PgRange, RangeBound};
use sentinel_driver::types::{Oid, ToSql};

fn roundtrip_multirange_i32(val: &PgMultirange<i32>) -> PgMultirange<i32> {
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    PgMultirange::from_sql_with_oids(&buf, Oid::INT4MULTIRANGE, Oid::INT4RANGE, Oid::INT4).unwrap()
}

#[test]
fn test_multirange_oid() {
    let val: PgMultirange<i32> = PgMultirange {
        ranges: vec![],
        multirange_oid: Oid::INT4MULTIRANGE,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    assert_eq!(val.oid(), Oid::INT4MULTIRANGE);
}

#[test]
fn test_multirange_empty() {
    let val: PgMultirange<i32> = PgMultirange {
        ranges: vec![],
        multirange_oid: Oid::INT4MULTIRANGE,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    let decoded = roundtrip_multirange_i32(&val);
    assert!(decoded.ranges.is_empty());
}

#[test]
fn test_multirange_single_range() {
    let val: PgMultirange<i32> = PgMultirange {
        ranges: vec![PgRange {
            lower: RangeBound::Inclusive(1),
            upper: RangeBound::Exclusive(10),
            is_empty: false,
            range_oid: Oid::INT4RANGE,
            element_oid: Oid::INT4,
        }],
        multirange_oid: Oid::INT4MULTIRANGE,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    let decoded = roundtrip_multirange_i32(&val);
    assert_eq!(decoded.ranges.len(), 1);
    assert_eq!(decoded.ranges[0].lower, RangeBound::Inclusive(1));
    assert_eq!(decoded.ranges[0].upper, RangeBound::Exclusive(10));
}

#[test]
fn test_multirange_multiple_ranges() {
    let val: PgMultirange<i32> = PgMultirange {
        ranges: vec![
            PgRange {
                lower: RangeBound::Inclusive(1),
                upper: RangeBound::Exclusive(5),
                is_empty: false,
                range_oid: Oid::INT4RANGE,
                element_oid: Oid::INT4,
            },
            PgRange {
                lower: RangeBound::Inclusive(10),
                upper: RangeBound::Exclusive(20),
                is_empty: false,
                range_oid: Oid::INT4RANGE,
                element_oid: Oid::INT4,
            },
            PgRange {
                lower: RangeBound::Inclusive(100),
                upper: RangeBound::Unbounded,
                is_empty: false,
                range_oid: Oid::INT4RANGE,
                element_oid: Oid::INT4,
            },
        ],
        multirange_oid: Oid::INT4MULTIRANGE,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    let decoded = roundtrip_multirange_i32(&val);
    assert_eq!(decoded.ranges.len(), 3);
    assert_eq!(decoded.ranges[0].lower, RangeBound::Inclusive(1));
    assert_eq!(decoded.ranges[1].lower, RangeBound::Inclusive(10));
    assert_eq!(decoded.ranges[2].upper, RangeBound::Unbounded);
}

#[test]
fn test_multirange_with_empty_range() {
    let val: PgMultirange<i32> = PgMultirange {
        ranges: vec![PgRange::empty(Oid::INT4RANGE, Oid::INT4)],
        multirange_oid: Oid::INT4MULTIRANGE,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    let decoded = roundtrip_multirange_i32(&val);
    assert_eq!(decoded.ranges.len(), 1);
    assert!(decoded.ranges[0].is_empty);
}

#[test]
fn test_multirange_wire_format_empty() {
    let val: PgMultirange<i32> = PgMultirange {
        ranges: vec![],
        multirange_oid: Oid::INT4MULTIRANGE,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    // Just count = 0
    assert_eq!(&buf[..], &0i32.to_be_bytes());
}

#[test]
fn test_multirange_decode_header_too_short() {
    let buf = [0u8; 2];
    let result = PgMultirange::<i32>::from_sql_with_oids(
        &buf,
        Oid::INT4MULTIRANGE,
        Oid::INT4RANGE,
        Oid::INT4,
    );
    assert!(result.is_err());
}

#[test]
fn test_multirange_decode_range_truncated() {
    // count=1 but no range data
    let buf = 1i32.to_be_bytes();
    let result = PgMultirange::<i32>::from_sql_with_oids(
        &buf,
        Oid::INT4MULTIRANGE,
        Oid::INT4RANGE,
        Oid::INT4,
    );
    assert!(result.is_err());
}

#[test]
fn test_multirange_i64_roundtrip() {
    let val: PgMultirange<i64> = PgMultirange {
        ranges: vec![PgRange {
            lower: RangeBound::Inclusive(100),
            upper: RangeBound::Exclusive(200),
            is_empty: false,
            range_oid: Oid::INT8RANGE,
            element_oid: Oid::INT8,
        }],
        multirange_oid: Oid::INT8MULTIRANGE,
        range_oid: Oid::INT8RANGE,
        element_oid: Oid::INT8,
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    let decoded =
        PgMultirange::from_sql_with_oids(&buf, Oid::INT8MULTIRANGE, Oid::INT8RANGE, Oid::INT8)
            .unwrap();
    assert_eq!(decoded.ranges.len(), 1);
    assert_eq!(decoded.ranges[0].lower, RangeBound::Inclusive(100i64));
}
