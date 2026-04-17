use bytes::{BufMut, BytesMut};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

/// PostgreSQL CUBE type -- an n-dimensional point or box.
///
/// CUBE is a PostgreSQL extension type for multi-dimensional geometric data.
/// It represents either a point (all dimensions equal) or a box (two corners).
///
/// Binary wire format:
/// - `ndim` (u32): number of dimensions
/// - `flags` (u32): bit 0 = is_point
/// - coordinates: `ndim` f64 values for points, `ndim * 2` for boxes
///
/// Uses TEXT OID as carrier since CUBE is an extension type.
#[derive(Debug, Clone, PartialEq)]
pub struct PgCube {
    /// For a point: `[x, y, z, ...]` (ndim values).
    /// For a box: `[x1, y1, z1, ..., x2, y2, z2, ...]` (ndim * 2 values).
    pub coordinates: Vec<f64>,
    /// True if this represents a point, false if a box.
    pub is_point: bool,
}

const CUBE_IS_POINT: u32 = 1;

impl PgCube {
    /// Create a point with the given coordinates.
    pub fn point(coordinates: Vec<f64>) -> Self {
        PgCube {
            coordinates,
            is_point: true,
        }
    }

    /// Create a box with the given coordinates and number of dimensions.
    ///
    /// `coordinates` must have exactly `ndim * 2` values:
    /// the first `ndim` are the lower-left corner, the last `ndim` are the upper-right.
    pub fn cube(coordinates: Vec<f64>, ndim: usize) -> Self {
        debug_assert_eq!(
            coordinates.len(),
            ndim * 2,
            "box coordinates must be ndim * 2"
        );
        PgCube {
            coordinates,
            is_point: false,
        }
    }

    /// Number of dimensions.
    pub fn ndim(&self) -> usize {
        if self.is_point {
            self.coordinates.len()
        } else if self.coordinates.is_empty() {
            0
        } else {
            self.coordinates.len() / 2
        }
    }
}

impl ToSql for PgCube {
    fn oid(&self) -> Oid {
        Oid::TEXT
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        let ndim = self.ndim() as u32;
        let flags = if self.is_point { CUBE_IS_POINT } else { 0 };

        buf.put_u32(ndim);
        buf.put_u32(flags);

        for &coord in &self.coordinates {
            buf.put_f64(coord);
        }

        Ok(())
    }
}

impl FromSql for PgCube {
    fn oid() -> Oid {
        Oid::TEXT
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        if buf.len() < 8 {
            return Err(Error::Decode(format!(
                "cube: expected at least 8 bytes, got {}",
                buf.len()
            )));
        }

        let ndim = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        let flags = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let is_point = (flags & CUBE_IS_POINT) != 0;

        let num_coords = if is_point { ndim } else { ndim * 2 };
        let expected_len = 8 + num_coords * 8;

        if buf.len() < expected_len {
            return Err(Error::Decode(format!(
                "cube: expected {} bytes for {ndim}D {}, got {}",
                expected_len,
                if is_point { "point" } else { "box" },
                buf.len()
            )));
        }

        let mut coordinates = Vec::with_capacity(num_coords);
        let mut offset = 8;
        for _ in 0..num_coords {
            let val = f64::from_be_bytes([
                buf[offset],
                buf[offset + 1],
                buf[offset + 2],
                buf[offset + 3],
                buf[offset + 4],
                buf[offset + 5],
                buf[offset + 6],
                buf[offset + 7],
            ]);
            coordinates.push(val);
            offset += 8;
        }

        Ok(PgCube {
            coordinates,
            is_point,
        })
    }
}

impl std::fmt::Display for PgCube {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_point {
            write!(f, "(")?;
            for (i, coord) in self.coordinates.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{coord}")?;
            }
            write!(f, ")")
        } else {
            let ndim = self.ndim();
            let (lower, upper) = self.coordinates.split_at(ndim);
            write!(f, "(")?;
            for (i, coord) in lower.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{coord}")?;
            }
            write!(f, "),(")?;
            for (i, coord) in upper.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{coord}")?;
            }
            write!(f, ")")
        }
    }
}
