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

// -- OID tests --

#[test]
fn test_line_oid() {
    let val = PgLine {
        a: 0.0,
        b: 0.0,
        c: 0.0,
    };
    assert_eq!(val.oid(), Oid::LINE);
    assert_eq!(<PgLine as FromSql>::oid(), Oid::LINE);
}

#[test]
fn test_lseg_oid() {
    let val = PgLSeg {
        start: PgPoint { x: 0.0, y: 0.0 },
        end: PgPoint { x: 0.0, y: 0.0 },
    };
    assert_eq!(val.oid(), Oid::LSEG);
    assert_eq!(<PgLSeg as FromSql>::oid(), Oid::LSEG);
}

#[test]
fn test_box_oid() {
    let val = PgBox {
        upper_right: PgPoint { x: 0.0, y: 0.0 },
        lower_left: PgPoint { x: 0.0, y: 0.0 },
    };
    assert_eq!(val.oid(), Oid::PG_BOX);
    assert_eq!(<PgBox as FromSql>::oid(), Oid::PG_BOX);
}

#[test]
fn test_circle_oid() {
    let val = PgCircle {
        center: PgPoint { x: 0.0, y: 0.0 },
        radius: 0.0,
    };
    assert_eq!(val.oid(), Oid::CIRCLE);
    assert_eq!(<PgCircle as FromSql>::oid(), Oid::CIRCLE);
}

// -- Wire format tests --

#[test]
fn test_line_wire_format() {
    let mut buf = BytesMut::new();
    PgLine {
        a: 1.0,
        b: 2.0,
        c: 3.0,
    }
    .to_sql(&mut buf)
    .ok();
    assert_eq!(buf.len(), 24);
    assert_eq!(&buf[0..8], &1.0f64.to_be_bytes());
    assert_eq!(&buf[8..16], &2.0f64.to_be_bytes());
    assert_eq!(&buf[16..24], &3.0f64.to_be_bytes());
}

#[test]
fn test_lseg_wire_format() {
    let mut buf = BytesMut::new();
    PgLSeg {
        start: PgPoint { x: 1.0, y: 2.0 },
        end: PgPoint { x: 3.0, y: 4.0 },
    }
    .to_sql(&mut buf)
    .ok();
    assert_eq!(buf.len(), 32);
    assert_eq!(&buf[0..8], &1.0f64.to_be_bytes());
    assert_eq!(&buf[24..32], &4.0f64.to_be_bytes());
}

#[test]
fn test_box_wire_format() {
    let mut buf = BytesMut::new();
    PgBox {
        upper_right: PgPoint { x: 10.0, y: 20.0 },
        lower_left: PgPoint { x: 1.0, y: 2.0 },
    }
    .to_sql(&mut buf)
    .ok();
    assert_eq!(buf.len(), 32);
    assert_eq!(&buf[0..8], &10.0f64.to_be_bytes());
    assert_eq!(&buf[8..16], &20.0f64.to_be_bytes());
    assert_eq!(&buf[16..24], &1.0f64.to_be_bytes());
    assert_eq!(&buf[24..32], &2.0f64.to_be_bytes());
}

#[test]
fn test_circle_wire_format() {
    let mut buf = BytesMut::new();
    PgCircle {
        center: PgPoint { x: 5.0, y: 6.0 },
        radius: 7.0,
    }
    .to_sql(&mut buf)
    .ok();
    assert_eq!(buf.len(), 24);
    assert_eq!(&buf[0..8], &5.0f64.to_be_bytes());
    assert_eq!(&buf[8..16], &6.0f64.to_be_bytes());
    assert_eq!(&buf[16..24], &7.0f64.to_be_bytes());
}

// -- Negative coordinate tests --

#[test]
fn test_point_negative_coords() {
    let val = PgPoint {
        x: -100.5,
        y: -200.75,
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgPoint::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_lseg_negative_coords() {
    let val = PgLSeg {
        start: PgPoint { x: -1.0, y: -2.0 },
        end: PgPoint { x: -3.0, y: -4.0 },
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgLSeg::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_box_negative_coords() {
    let val = PgBox {
        upper_right: PgPoint { x: 0.0, y: 0.0 },
        lower_left: PgPoint { x: -10.0, y: -10.0 },
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgBox::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

// -- FromSql OID static method tests --

#[test]
fn test_point_from_sql_oid() {
    assert_eq!(<PgPoint as FromSql>::oid(), Oid::POINT);
}

#[test]
fn test_circle_negative_radius() {
    let val = PgCircle {
        center: PgPoint { x: 0.0, y: 0.0 },
        radius: -1.0,
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgCircle::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_line_negative_coefficients() {
    let val = PgLine {
        a: -2.5,
        b: -3.5,
        c: -4.5,
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgLine::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}
