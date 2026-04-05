use bytes::{BufMut, BytesMut};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

const RANGE_EMPTY: u8 = 0x01;
const RANGE_LB_INC: u8 = 0x02;
const RANGE_UB_INC: u8 = 0x04;
const RANGE_LB_INF: u8 = 0x08;
const RANGE_UB_INF: u8 = 0x10;

/// A bound of a PostgreSQL range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeBound<T> {
    Inclusive(T),
    Exclusive(T),
    Unbounded,
}

/// PostgreSQL range type.
///
/// Generic over the element type `T`. The `range_oid` and `element_oid` must
/// be provided since Rust generics cannot map to PG range OIDs automatically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgRange<T> {
    pub lower: RangeBound<T>,
    pub upper: RangeBound<T>,
    pub is_empty: bool,
    pub range_oid: Oid,
    pub element_oid: Oid,
}

impl<T> PgRange<T> {
    /// Create an empty range.
    pub fn empty(range_oid: Oid, element_oid: Oid) -> Self {
        PgRange {
            lower: RangeBound::Unbounded,
            upper: RangeBound::Unbounded,
            is_empty: true,
            range_oid,
            element_oid,
        }
    }
}

impl<T: ToSql> ToSql for PgRange<T> {
    fn oid(&self) -> Oid {
        self.range_oid
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        if self.is_empty {
            buf.put_u8(RANGE_EMPTY);
            return Ok(());
        }

        let mut flags: u8 = 0;

        match &self.lower {
            RangeBound::Inclusive(_) => flags |= RANGE_LB_INC,
            RangeBound::Exclusive(_) => {}
            RangeBound::Unbounded => flags |= RANGE_LB_INF,
        }

        match &self.upper {
            RangeBound::Inclusive(_) => flags |= RANGE_UB_INC,
            RangeBound::Exclusive(_) => {}
            RangeBound::Unbounded => flags |= RANGE_UB_INF,
        }

        buf.put_u8(flags);

        // Encode lower bound
        match &self.lower {
            RangeBound::Inclusive(v) | RangeBound::Exclusive(v) => {
                let len_pos = buf.len();
                buf.put_i32(0); // placeholder
                let data_start = buf.len();
                v.to_sql(buf)?;
                let data_len = (buf.len() - data_start) as i32;
                buf[len_pos..len_pos + 4].copy_from_slice(&data_len.to_be_bytes());
            }
            RangeBound::Unbounded => {}
        }

        // Encode upper bound
        match &self.upper {
            RangeBound::Inclusive(v) | RangeBound::Exclusive(v) => {
                let len_pos = buf.len();
                buf.put_i32(0); // placeholder
                let data_start = buf.len();
                v.to_sql(buf)?;
                let data_len = (buf.len() - data_start) as i32;
                buf[len_pos..len_pos + 4].copy_from_slice(&data_len.to_be_bytes());
            }
            RangeBound::Unbounded => {}
        }

        Ok(())
    }
}

impl<T: FromSql> PgRange<T> {
    /// Decode a range from binary format. Requires OIDs since generic types
    /// cannot determine them.
    pub fn from_sql_with_oids(buf: &[u8], range_oid: Oid, element_oid: Oid) -> Result<Self> {
        if buf.is_empty() {
            return Err(Error::Decode("range: empty buffer".into()));
        }

        let flags = buf[0];

        if flags & RANGE_EMPTY != 0 {
            return Ok(PgRange::empty(range_oid, element_oid));
        }

        let mut offset = 1;

        let lower = if flags & RANGE_LB_INF != 0 {
            RangeBound::Unbounded
        } else {
            if offset + 4 > buf.len() {
                return Err(Error::Decode("range: lower bound truncated".into()));
            }
            let len = i32::from_be_bytes([
                buf[offset],
                buf[offset + 1],
                buf[offset + 2],
                buf[offset + 3],
            ]) as usize;
            offset += 4;
            if offset + len > buf.len() {
                return Err(Error::Decode(
                    "range: lower bound data truncated".into(),
                ));
            }
            let val = T::from_sql(&buf[offset..offset + len])?;
            offset += len;
            if flags & RANGE_LB_INC != 0 {
                RangeBound::Inclusive(val)
            } else {
                RangeBound::Exclusive(val)
            }
        };

        let upper = if flags & RANGE_UB_INF != 0 {
            RangeBound::Unbounded
        } else {
            if offset + 4 > buf.len() {
                return Err(Error::Decode("range: upper bound truncated".into()));
            }
            let len = i32::from_be_bytes([
                buf[offset],
                buf[offset + 1],
                buf[offset + 2],
                buf[offset + 3],
            ]) as usize;
            offset += 4;
            if offset + len > buf.len() {
                return Err(Error::Decode(
                    "range: upper bound data truncated".into(),
                ));
            }
            let val = T::from_sql(&buf[offset..offset + len])?;
            if flags & RANGE_UB_INC != 0 {
                RangeBound::Inclusive(val)
            } else {
                RangeBound::Exclusive(val)
            }
        };

        Ok(PgRange {
            lower,
            upper,
            is_empty: false,
            range_oid,
            element_oid,
        })
    }
}
