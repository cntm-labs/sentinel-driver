use bytes::{BufMut, BytesMut};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

/// PostgreSQL MONEY type -- stored as i64 cents.
///
/// The actual fractional digits depend on the server's `lc_monetary` setting,
/// but the wire format is always an i64. Most locales use 2 decimal places,
/// so a value of 12345 typically represents $123.45.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PgMoney(pub i64);

impl ToSql for PgMoney {
    fn oid(&self) -> Oid {
        Oid::MONEY
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_i64(self.0);
        Ok(())
    }
}

impl FromSql for PgMoney {
    fn oid() -> Oid {
        Oid::MONEY
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let arr: [u8; 8] = buf
            .try_into()
            .map_err(|_| Error::Decode(format!("money: expected 8 bytes, got {}", buf.len())))?;
        Ok(PgMoney(i64::from_be_bytes(arr)))
    }
}
