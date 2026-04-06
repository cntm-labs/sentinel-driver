use bytes::BytesMut;
use sentinel_driver::types::range::{PgRange, RangeBound};
use sentinel_driver::types::{Oid, ToSql};

#[test]
fn test_range_empty_i32() {
    let range: PgRange<i32> = PgRange::empty(Oid::INT4RANGE, Oid::INT4);
    let mut buf = BytesMut::new();
    range.to_sql(&mut buf).ok();
    let decoded = PgRange::<i32>::from_sql_with_oids(&buf, Oid::INT4RANGE, Oid::INT4).ok();
    assert!(decoded.is_some());
    assert!(decoded.as_ref().map_or(false, |r| r.is_empty));
}

#[test]
fn test_range_inclusive_i32() {
    let range = PgRange {
        lower: RangeBound::Inclusive(1i32),
        upper: RangeBound::Inclusive(10i32),
        is_empty: false,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    let mut buf = BytesMut::new();
    range.to_sql(&mut buf).ok();
    let decoded = PgRange::<i32>::from_sql_with_oids(&buf, Oid::INT4RANGE, Oid::INT4).ok();
    let decoded = decoded.as_ref();
    assert_eq!(decoded.map(|r| &r.lower), Some(&RangeBound::Inclusive(1)));
    assert_eq!(decoded.map(|r| &r.upper), Some(&RangeBound::Inclusive(10)));
}

#[test]
fn test_range_exclusive_i64() {
    let range = PgRange {
        lower: RangeBound::Exclusive(0i64),
        upper: RangeBound::Exclusive(100i64),
        is_empty: false,
        range_oid: Oid::INT8RANGE,
        element_oid: Oid::INT8,
    };
    let mut buf = BytesMut::new();
    range.to_sql(&mut buf).ok();
    let decoded = PgRange::<i64>::from_sql_with_oids(&buf, Oid::INT8RANGE, Oid::INT8).ok();
    let decoded = decoded.as_ref();
    assert_eq!(decoded.map(|r| &r.lower), Some(&RangeBound::Exclusive(0)));
    assert_eq!(decoded.map(|r| &r.upper), Some(&RangeBound::Exclusive(100)));
}

#[test]
fn test_range_unbounded_lower() {
    let range = PgRange {
        lower: RangeBound::<i32>::Unbounded,
        upper: RangeBound::Inclusive(50),
        is_empty: false,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    let mut buf = BytesMut::new();
    range.to_sql(&mut buf).ok();
    let decoded = PgRange::<i32>::from_sql_with_oids(&buf, Oid::INT4RANGE, Oid::INT4).ok();
    let decoded = decoded.as_ref();
    assert_eq!(
        decoded.map(|r| &r.lower),
        Some(&RangeBound::<i32>::Unbounded)
    );
    assert_eq!(decoded.map(|r| &r.upper), Some(&RangeBound::Inclusive(50)));
}

#[test]
fn test_range_unbounded_both() {
    let range = PgRange {
        lower: RangeBound::<i32>::Unbounded,
        upper: RangeBound::<i32>::Unbounded,
        is_empty: false,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    let mut buf = BytesMut::new();
    range.to_sql(&mut buf).ok();
    let decoded = PgRange::<i32>::from_sql_with_oids(&buf, Oid::INT4RANGE, Oid::INT4).ok();
    let decoded = decoded.as_ref();
    assert_eq!(
        decoded.map(|r| &r.lower),
        Some(&RangeBound::<i32>::Unbounded)
    );
    assert_eq!(
        decoded.map(|r| &r.upper),
        Some(&RangeBound::<i32>::Unbounded)
    );
}

#[test]
fn test_range_wire_format_empty() {
    let range: PgRange<i32> = PgRange::empty(Oid::INT4RANGE, Oid::INT4);
    let mut buf = BytesMut::new();
    range.to_sql(&mut buf).ok();
    assert_eq!(buf.len(), 1);
    assert_eq!(buf[0], 0x01); // RANGE_EMPTY flag
}

#[test]
fn test_range_oid() {
    let range = PgRange {
        lower: RangeBound::Inclusive(1i32),
        upper: RangeBound::Inclusive(10i32),
        is_empty: false,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    assert_eq!(range.oid(), Oid::INT4RANGE);
}

#[test]
fn test_range_mixed_inclusive_exclusive() {
    let range = PgRange {
        lower: RangeBound::Inclusive(1i32),
        upper: RangeBound::Exclusive(10i32),
        is_empty: false,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    let mut buf = BytesMut::new();
    range.to_sql(&mut buf).ok();
    let decoded = PgRange::<i32>::from_sql_with_oids(&buf, Oid::INT4RANGE, Oid::INT4).ok();
    let decoded = decoded.as_ref();
    assert_eq!(decoded.map(|r| &r.lower), Some(&RangeBound::Inclusive(1)));
    assert_eq!(decoded.map(|r| &r.upper), Some(&RangeBound::Exclusive(10)));
}

#[test]
fn test_range_exclusive_inclusive() {
    let range = PgRange {
        lower: RangeBound::Exclusive(0i32),
        upper: RangeBound::Inclusive(100i32),
        is_empty: false,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    let mut buf = BytesMut::new();
    range.to_sql(&mut buf).ok();
    let decoded = PgRange::<i32>::from_sql_with_oids(&buf, Oid::INT4RANGE, Oid::INT4).ok();
    let decoded = decoded.as_ref();
    assert_eq!(decoded.map(|r| &r.lower), Some(&RangeBound::Exclusive(0)));
    assert_eq!(decoded.map(|r| &r.upper), Some(&RangeBound::Inclusive(100)));
}

#[test]
fn test_range_unbounded_upper() {
    let range = PgRange {
        lower: RangeBound::Exclusive(5i32),
        upper: RangeBound::<i32>::Unbounded,
        is_empty: false,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    let mut buf = BytesMut::new();
    range.to_sql(&mut buf).ok();
    let decoded = PgRange::<i32>::from_sql_with_oids(&buf, Oid::INT4RANGE, Oid::INT4).ok();
    let decoded = decoded.as_ref();
    assert_eq!(decoded.map(|r| &r.lower), Some(&RangeBound::Exclusive(5)));
    assert_eq!(
        decoded.map(|r| &r.upper),
        Some(&RangeBound::<i32>::Unbounded)
    );
}

#[test]
fn test_range_decode_empty_buffer() {
    let buf: &[u8] = &[];
    assert!(PgRange::<i32>::from_sql_with_oids(buf, Oid::INT4RANGE, Oid::INT4).is_err());
}

#[test]
fn test_range_decode_truncated_lower_bound() {
    // flags=0x02 (LB_INC) but no length field follows
    let buf = [0x02];
    assert!(PgRange::<i32>::from_sql_with_oids(&buf, Oid::INT4RANGE, Oid::INT4).is_err());
}

#[test]
fn test_range_decode_truncated_lower_data() {
    // flags=0x02 (LB_INC), len=4, but only 2 bytes of data
    let buf = [0x02, 0, 0, 0, 4, 0, 0];
    assert!(PgRange::<i32>::from_sql_with_oids(&buf, Oid::INT4RANGE, Oid::INT4).is_err());
}

#[test]
fn test_range_decode_truncated_upper_bound() {
    // flags=0x06 (LB_INC|UB_INC), valid lower bound but truncated upper
    let mut buf = vec![0x06]; // flags
    buf.extend_from_slice(&4i32.to_be_bytes()); // lower len
    buf.extend_from_slice(&1i32.to_be_bytes()); // lower val = 1
                                                // no upper bound data
    assert!(PgRange::<i32>::from_sql_with_oids(&buf, Oid::INT4RANGE, Oid::INT4).is_err());
}

#[test]
fn test_range_decode_truncated_upper_data() {
    // flags=0x06 (LB_INC|UB_INC), valid lower bound but truncated upper data
    let mut buf = vec![0x06]; // flags
    buf.extend_from_slice(&4i32.to_be_bytes()); // lower len
    buf.extend_from_slice(&1i32.to_be_bytes()); // lower val = 1
    buf.extend_from_slice(&4i32.to_be_bytes()); // upper len = 4
    buf.extend_from_slice(&[0, 0]); // only 2 bytes of upper data
    assert!(PgRange::<i32>::from_sql_with_oids(&buf, Oid::INT4RANGE, Oid::INT4).is_err());
}
