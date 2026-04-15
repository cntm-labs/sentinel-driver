use bytes::{BufMut, BytesMut};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

/// PostgreSQL TIMETZ (TIME WITH TIME ZONE) type.
///
/// Wire format: i64 microseconds since midnight + i32 UTC offset in seconds (negated).
/// PostgreSQL stores the offset with west-positive convention, so UTC+7 is stored as -25200.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PgTimeTz {
    pub time: chrono::NaiveTime,
    pub offset_seconds: i32,
}

impl ToSql for PgTimeTz {
    fn oid(&self) -> Oid {
        Oid::TIMETZ
    }

    #[allow(clippy::expect_used)]
    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        let midnight = chrono::NaiveTime::from_hms_opt(0, 0, 0).expect("midnight is valid");
        let us = self
            .time
            .signed_duration_since(midnight)
            .num_microseconds()
            .unwrap_or(0);
        buf.put_i64(us);
        // PG stores offset negated (west-positive)
        buf.put_i32(-self.offset_seconds);
        Ok(())
    }
}

impl FromSql for PgTimeTz {
    fn oid() -> Oid {
        Oid::TIMETZ
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        if buf.len() != 12 {
            return Err(Error::Decode(format!(
                "timetz: expected 12 bytes, got {}",
                buf.len()
            )));
        }
        let us = i64::from_be_bytes([
            buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
        ]);
        let pg_offset = i32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);

        let secs = (us / 1_000_000) as u32;
        let micro = (us % 1_000_000) as u32;
        let time = chrono::NaiveTime::from_num_seconds_from_midnight_opt(secs, micro * 1000)
            .ok_or_else(|| {
                Error::Decode(format!("timetz: time out of range: {us} microseconds"))
            })?;

        Ok(PgTimeTz {
            time,
            // Un-negate to get standard east-positive offset
            offset_seconds: -pg_offset,
        })
    }
}
