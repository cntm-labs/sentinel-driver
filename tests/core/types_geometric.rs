use bytes::BytesMut;
use sentinel_driver::types::geometric::{PgBox, PgCircle, PgLSeg, PgLine, PgPoint};
use sentinel_driver::types::{FromSql, Oid, ToSql};

// -- PgPoint --

#[test]
fn test_point_roundtrip() {
    let val = PgPoint { x: 1.5, y: -2.75 };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgPoint::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_point_wire_format() {
    let mut buf = BytesMut::new();
    PgPoint { x: 1.0, y: 2.0 }.to_sql(&mut buf).ok();
    assert_eq!(buf.len(), 16);
    assert_eq!(&buf[0..8], &1.0f64.to_be_bytes());
    assert_eq!(&buf[8..16], &2.0f64.to_be_bytes());
}

#[test]
fn test_point_oid() {
    assert_eq!(PgPoint { x: 0.0, y: 0.0 }.oid(), Oid::POINT);
}

// -- PgLine --

#[test]
fn test_line_roundtrip() {
    let val = PgLine {
        a: 1.0,
        b: -1.0,
        c: 0.0,
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgLine::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
    assert_eq!(buf.len(), 24);
}

// -- PgLSeg --

#[test]
fn test_lseg_roundtrip() {
    let val = PgLSeg {
        start: PgPoint { x: 0.0, y: 0.0 },
        end: PgPoint { x: 3.0, y: 4.0 },
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgLSeg::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
    assert_eq!(buf.len(), 32);
}

// -- PgBox --

#[test]
fn test_box_roundtrip() {
    let val = PgBox {
        upper_right: PgPoint { x: 10.0, y: 10.0 },
        lower_left: PgPoint { x: 0.0, y: 0.0 },
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgBox::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
    assert_eq!(buf.len(), 32);
}

// -- PgCircle --

#[test]
fn test_circle_roundtrip() {
    let val = PgCircle {
        center: PgPoint { x: 5.0, y: 5.0 },
        radius: 3.0,
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgCircle::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
    assert_eq!(buf.len(), 24);
}

// -- Error cases --

#[test]
fn test_point_decode_wrong_size() {
    let buf = [0u8; 10];
    assert!(PgPoint::from_sql(&buf).is_err());
}

#[test]
fn test_line_decode_wrong_size() {
    let buf = [0u8; 10];
    assert!(PgLine::from_sql(&buf).is_err());
}

#[test]
fn test_lseg_decode_wrong_size() {
    let buf = [0u8; 10];
    assert!(PgLSeg::from_sql(&buf).is_err());
}

#[test]
fn test_box_decode_wrong_size() {
    let buf = [0u8; 10];
    assert!(PgBox::from_sql(&buf).is_err());
}

#[test]
fn test_circle_decode_wrong_size() {
    let buf = [0u8; 10];
    assert!(PgCircle::from_sql(&buf).is_err());
}
