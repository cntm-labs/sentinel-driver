use bytes::{BufMut, BytesMut};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

/// PostgreSQL BIT / VARBIT type.
///
/// Stores a fixed- or variable-length bit string.
/// `data` holds raw bytes (MSB-first, padded with trailing zeros in the last byte).
/// `bit_length` is the actual number of significant bits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PgBit {
    pub data: Vec<u8>,
    pub bit_length: i32,
}

impl PgBit {
    /// Create from a slice of bools (true = 1, false = 0).
    pub fn from_bools(bits: &[bool]) -> Self {
        let bit_length = bits.len() as i32;
        let byte_count = bits.len().div_ceil(8);
        let mut data = vec![0u8; byte_count];

        for (i, &bit) in bits.iter().enumerate() {
            if bit {
                data[i / 8] |= 1 << (7 - (i % 8));
            }
        }

        PgBit { data, bit_length }
    }

    /// Get the bit at the given index (0-based, MSB-first).
    pub fn get(&self, index: usize) -> Option<bool> {
        if index >= self.bit_length as usize {
            return None;
        }
        Some(self.data[index / 8] & (1 << (7 - (index % 8))) != 0)
    }

    /// Number of bits.
    pub fn len(&self) -> usize {
        self.bit_length as usize
    }

    /// True if zero bits.
    pub fn is_empty(&self) -> bool {
        self.bit_length == 0
    }
}

impl ToSql for PgBit {
    fn oid(&self) -> Oid {
        Oid::VARBIT
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_i32(self.bit_length);
        buf.put_slice(&self.data);
        Ok(())
    }
}

impl FromSql for PgBit {
    fn oid() -> Oid {
        Oid::VARBIT
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        if buf.len() < 4 {
            return Err(Error::Decode(format!(
                "bit: expected at least 4 bytes, got {}",
                buf.len()
            )));
        }

        let bit_length = i32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);

        if bit_length < 0 {
            return Err(Error::Decode(format!(
                "bit: negative bit length {bit_length}"
            )));
        }

        let byte_count = (bit_length as usize).div_ceil(8);

        if buf.len() < 4 + byte_count {
            return Err(Error::Decode(format!(
                "bit: expected {} data bytes for {} bits, got {}",
                byte_count,
                bit_length,
                buf.len() - 4
            )));
        }

        let data = buf[4..4 + byte_count].to_vec();

        Ok(PgBit { data, bit_length })
    }
}
