use bytes::BytesMut;
use sentinel_driver::types::cube::PgCube;
use sentinel_driver::types::{FromSql, Oid, ToSql};

// ── Point roundtrips ────────────────────────────────

#[test]
fn test_cube_point_1d_roundtrip() {
    let val = PgCube::point(vec![1.0]);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgCube::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_cube_point_2d_roundtrip() {
    let val = PgCube::point(vec![1.0, 2.0]);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgCube::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_cube_point_3d_roundtrip() {
    let val = PgCube::point(vec![1.0, 2.0, 3.0]);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgCube::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

// ── Box roundtrips ──────────────────────────────────

#[test]
fn test_cube_box_2d_roundtrip() {
    let val = PgCube::cube(vec![0.0, 0.0, 1.0, 1.0], 2);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgCube::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_cube_box_3d_roundtrip() {
    let val = PgCube::cube(vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0], 3);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgCube::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

// ── OID ─────────────────────────────────────────────

#[test]
fn test_cube_oid() {
    let val = PgCube::point(vec![1.0]);
    assert_eq!(val.oid(), Oid::TEXT);
    assert_eq!(<PgCube as FromSql>::oid(), Oid::TEXT);
}

// ── Wire format ─────────────────────────────────────

#[test]
fn test_cube_point_wire_format() {
    let val = PgCube::point(vec![1.0, 2.0]);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();

    // ndim(4) + flags(4) + 2 * f64(8) = 24 bytes
    assert_eq!(buf.len(), 24);

    // ndim = 2
    assert_eq!(&buf[0..4], &2u32.to_be_bytes());
    // flags = 1 (is_point)
    assert_eq!(&buf[4..8], &1u32.to_be_bytes());
    // x = 1.0
    assert_eq!(&buf[8..16], &1.0f64.to_be_bytes());
    // y = 2.0
    assert_eq!(&buf[16..24], &2.0f64.to_be_bytes());
}

#[test]
fn test_cube_box_wire_format() {
    let val = PgCube::cube(vec![0.0, 0.0, 1.0, 1.0], 2);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();

    // ndim(4) + flags(4) + 4 * f64(8) = 40 bytes
    assert_eq!(buf.len(), 40);

    // ndim = 2
    assert_eq!(&buf[0..4], &2u32.to_be_bytes());
    // flags = 0 (not a point)
    assert_eq!(&buf[4..8], &0u32.to_be_bytes());
}

// ── Edge cases ──────────────────────────────────────

#[test]
fn test_cube_zero_dimensions() {
    let val = PgCube::point(vec![]);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgCube::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_cube_negative_coords() {
    let val = PgCube::point(vec![-1.5, -2.5, -3.5]);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgCube::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_cube_decode_too_short() {
    assert!(PgCube::from_sql(&[0u8; 4]).is_err());
}

#[test]
fn test_cube_decode_truncated_coords() {
    // ndim=2, flags=1 (point), but only 1 f64 instead of 2
    let mut buf = BytesMut::new();
    use bytes::BufMut;
    buf.put_u32(2); // ndim
    buf.put_u32(1); // flags (point)
    buf.put_f64(1.0); // only 1 coordinate, need 2
    assert!(PgCube::from_sql(&buf).is_err());
}

// ── Display ─────────────────────────────────────────

#[test]
fn test_cube_point_display() {
    let val = PgCube::point(vec![1.0, 2.0, 3.0]);
    assert_eq!(val.to_string(), "(1, 2, 3)");
}

#[test]
fn test_cube_box_display() {
    let val = PgCube::cube(vec![0.0, 0.0, 1.0, 1.0], 2);
    assert_eq!(val.to_string(), "(0, 0),(1, 1)");
}

#[test]
fn test_cube_empty_display() {
    let val = PgCube::point(vec![]);
    assert_eq!(val.to_string(), "()");
}
