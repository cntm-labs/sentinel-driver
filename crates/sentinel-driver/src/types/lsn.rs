use bytes::{BufMut, BytesMut};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

/// PostgreSQL PG_LSN type -- a 64-bit Log Sequence Number.
///
/// Represents a position in the WAL (write-ahead log).
/// Wire format is 8 bytes, big-endian u64 (sent as i64 on the wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PgLsn(pub u64);

impl ToSql for PgLsn {
    fn oid(&self) -> Oid {
        Oid::PG_LSN
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_i64(self.0 as i64);
        Ok(())
    }
}

impl FromSql for PgLsn {
    fn oid() -> Oid {
        Oid::PG_LSN
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let arr: [u8; 8] = buf
            .try_into()
            .map_err(|_| Error::Decode(format!("pg_lsn: expected 8 bytes, got {}", buf.len())))?;
        Ok(PgLsn(i64::from_be_bytes(arr) as u64))
    }
}

impl std::fmt::Display for PgLsn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hi = (self.0 >> 32) as u32;
        let lo = self.0 as u32;
        write!(f, "{hi:X}/{lo:X}")
    }
}
