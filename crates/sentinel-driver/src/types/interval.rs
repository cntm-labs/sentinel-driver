use bytes::{BufMut, BytesMut};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

/// PostgreSQL INTERVAL type.
///
/// Stored as three components matching PG's internal representation:
/// - `months` -- number of months (years x 12 + months)
/// - `days` -- number of days (not normalized to months)
/// - `microseconds` -- time component in microseconds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PgInterval {
    pub months: i32,
    pub days: i32,
    pub microseconds: i64,
}

impl ToSql for PgInterval {
    fn oid(&self) -> Oid {
        Oid::INTERVAL
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_i64(self.microseconds);
        buf.put_i32(self.days);
        buf.put_i32(self.months);
        Ok(())
    }
}

impl FromSql for PgInterval {
    fn oid() -> Oid {
        Oid::INTERVAL
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        if buf.len() != 16 {
            return Err(Error::Decode(format!(
                "interval: expected 16 bytes, got {}",
                buf.len()
            )));
        }

        let microseconds = i64::from_be_bytes([
            buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
        ]);
        let days = i32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);
        let months = i32::from_be_bytes([buf[12], buf[13], buf[14], buf[15]]);

        Ok(PgInterval {
            months,
            days,
            microseconds,
        })
    }
}
