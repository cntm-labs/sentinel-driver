use bytes::{BufMut, BytesMut};

use crate::error::{Error, Result};
use crate::types::range::PgRange;
use crate::types::{FromSql, Oid, ToSql};

/// PostgreSQL multirange type (PG 14+).
///
/// A multirange is an ordered list of non-overlapping ranges.
/// Wire format: count(i32) + [length(i32) + range_bytes] per range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgMultirange<T> {
    pub ranges: Vec<PgRange<T>>,
    pub multirange_oid: Oid,
    pub range_oid: Oid,
    pub element_oid: Oid,
}

impl<T: ToSql> ToSql for PgMultirange<T> {
    fn oid(&self) -> Oid {
        self.multirange_oid
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_i32(self.ranges.len() as i32);

        for range in &self.ranges {
            let len_pos = buf.len();
            buf.put_i32(0); // placeholder for range length
            let data_start = buf.len();
            range.to_sql(buf)?;
            let data_len = (buf.len() - data_start) as i32;
            buf[len_pos..len_pos + 4].copy_from_slice(&data_len.to_be_bytes());
        }

        Ok(())
    }
}

impl<T: FromSql> PgMultirange<T> {
    /// Decode a multirange from binary format. Requires OIDs since generic
    /// types cannot determine them.
    pub fn from_sql_with_oids(
        buf: &[u8],
        multirange_oid: Oid,
        range_oid: Oid,
        element_oid: Oid,
    ) -> Result<Self> {
        if buf.len() < 4 {
            return Err(Error::Decode("multirange: header too short".into()));
        }

        let count = i32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        let mut offset = 4;
        let mut ranges = Vec::with_capacity(count);

        for _ in 0..count {
            if offset + 4 > buf.len() {
                return Err(Error::Decode("multirange: range length truncated".into()));
            }
            let range_len = i32::from_be_bytes([
                buf[offset],
                buf[offset + 1],
                buf[offset + 2],
                buf[offset + 3],
            ]) as usize;
            offset += 4;

            if offset + range_len > buf.len() {
                return Err(Error::Decode("multirange: range data truncated".into()));
            }

            let range = PgRange::from_sql_with_oids(
                &buf[offset..offset + range_len],
                range_oid,
                element_oid,
            )?;
            ranges.push(range);
            offset += range_len;
        }

        Ok(PgMultirange {
            ranges,
            multirange_oid,
            range_oid,
            element_oid,
        })
    }
}
