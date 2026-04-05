use bytes::{BufMut, BytesMut};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

/// PostgreSQL POINT type -- 16 bytes (2 x f64).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PgPoint {
    pub x: f64,
    pub y: f64,
}

/// PostgreSQL LINE type -- 24 bytes (3 x f64), represents Ax + By + C = 0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PgLine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

/// PostgreSQL LSEG type -- 32 bytes (4 x f64), a line segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PgLSeg {
    pub start: PgPoint,
    pub end: PgPoint,
}

/// PostgreSQL BOX type -- 32 bytes (4 x f64), axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PgBox {
    pub upper_right: PgPoint,
    pub lower_left: PgPoint,
}

/// PostgreSQL CIRCLE type -- 24 bytes (2 x f64 center + f64 radius).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PgCircle {
    pub center: PgPoint,
    pub radius: f64,
}

// Helper to read f64 from a byte slice at an offset
fn read_f64(buf: &[u8], off: usize) -> f64 {
    f64::from_be_bytes([
        buf[off],
        buf[off + 1],
        buf[off + 2],
        buf[off + 3],
        buf[off + 4],
        buf[off + 5],
        buf[off + 6],
        buf[off + 7],
    ])
}

// -- PgPoint (16 bytes) --

impl ToSql for PgPoint {
    fn oid(&self) -> Oid {
        Oid::POINT
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_f64(self.x);
        buf.put_f64(self.y);
        Ok(())
    }
}

impl FromSql for PgPoint {
    fn oid() -> Oid {
        Oid::POINT
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        if buf.len() != 16 {
            return Err(Error::Decode(format!(
                "point: expected 16 bytes, got {}",
                buf.len()
            )));
        }
        Ok(PgPoint {
            x: read_f64(buf, 0),
            y: read_f64(buf, 8),
        })
    }
}

// -- PgLine (24 bytes) --

impl ToSql for PgLine {
    fn oid(&self) -> Oid {
        Oid::LINE
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_f64(self.a);
        buf.put_f64(self.b);
        buf.put_f64(self.c);
        Ok(())
    }
}

impl FromSql for PgLine {
    fn oid() -> Oid {
        Oid::LINE
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        if buf.len() != 24 {
            return Err(Error::Decode(format!(
                "line: expected 24 bytes, got {}",
                buf.len()
            )));
        }
        Ok(PgLine {
            a: read_f64(buf, 0),
            b: read_f64(buf, 8),
            c: read_f64(buf, 16),
        })
    }
}

// -- PgLSeg (32 bytes) --

impl ToSql for PgLSeg {
    fn oid(&self) -> Oid {
        Oid::LSEG
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_f64(self.start.x);
        buf.put_f64(self.start.y);
        buf.put_f64(self.end.x);
        buf.put_f64(self.end.y);
        Ok(())
    }
}

impl FromSql for PgLSeg {
    fn oid() -> Oid {
        Oid::LSEG
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        if buf.len() != 32 {
            return Err(Error::Decode(format!(
                "lseg: expected 32 bytes, got {}",
                buf.len()
            )));
        }
        Ok(PgLSeg {
            start: PgPoint {
                x: read_f64(buf, 0),
                y: read_f64(buf, 8),
            },
            end: PgPoint {
                x: read_f64(buf, 16),
                y: read_f64(buf, 24),
            },
        })
    }
}

// -- PgBox (32 bytes) --

impl ToSql for PgBox {
    fn oid(&self) -> Oid {
        Oid::PG_BOX
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_f64(self.upper_right.x);
        buf.put_f64(self.upper_right.y);
        buf.put_f64(self.lower_left.x);
        buf.put_f64(self.lower_left.y);
        Ok(())
    }
}

impl FromSql for PgBox {
    fn oid() -> Oid {
        Oid::PG_BOX
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        if buf.len() != 32 {
            return Err(Error::Decode(format!(
                "box: expected 32 bytes, got {}",
                buf.len()
            )));
        }
        Ok(PgBox {
            upper_right: PgPoint {
                x: read_f64(buf, 0),
                y: read_f64(buf, 8),
            },
            lower_left: PgPoint {
                x: read_f64(buf, 16),
                y: read_f64(buf, 24),
            },
        })
    }
}

// -- PgCircle (24 bytes) --

impl ToSql for PgCircle {
    fn oid(&self) -> Oid {
        Oid::CIRCLE
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_f64(self.center.x);
        buf.put_f64(self.center.y);
        buf.put_f64(self.radius);
        Ok(())
    }
}

impl FromSql for PgCircle {
    fn oid() -> Oid {
        Oid::CIRCLE
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        if buf.len() != 24 {
            return Err(Error::Decode(format!(
                "circle: expected 24 bytes, got {}",
                buf.len()
            )));
        }
        Ok(PgCircle {
            center: PgPoint {
                x: read_f64(buf, 0),
                y: read_f64(buf, 8),
            },
            radius: read_f64(buf, 16),
        })
    }
}
