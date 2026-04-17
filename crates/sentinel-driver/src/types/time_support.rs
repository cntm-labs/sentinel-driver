//! Feature-gated `time` crate support for PostgreSQL date/time types.
//!
//! Enabled with the `with-time` feature. Provides an alternative to chrono
//! for date/time encoding and decoding.
//!
//! | Rust type                  | PG type      | OID  |
//! |----------------------------|-------------|------|
//! | `time::OffsetDateTime`     | TIMESTAMPTZ | 1184 |
//! | `time::PrimitiveDateTime`  | TIMESTAMP   | 1114 |
//! | `time::Date`               | DATE        | 1082 |
//! | `time::Time`               | TIME        | 1083 |

use bytes::{BufMut, BytesMut};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

/// PG epoch offset in microseconds from Unix epoch.
/// Unix epoch: 1970-01-01, PG epoch: 2000-01-01.
/// Difference: 10957 days * 86400 s/day * 1_000_000 us/s = 946_684_800_000_000 us.
const PG_EPOCH_OFFSET_US: i64 = 946_684_800_000_000;

/// PG epoch as Julian day number (2000-01-01).
const PG_EPOCH_JULIAN_DAY: i32 = 2_451_545;

// ── OffsetDateTime → TIMESTAMPTZ ────────────────────

impl ToSql for time::OffsetDateTime {
    fn oid(&self) -> Oid {
        Oid::TIMESTAMPTZ
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        let unix_us = self.unix_timestamp() * 1_000_000 + i64::from(self.microsecond());
        let pg_us = unix_us - PG_EPOCH_OFFSET_US;
        buf.put_i64(pg_us);
        Ok(())
    }
}

impl FromSql for time::OffsetDateTime {
    fn oid() -> Oid {
        Oid::TIMESTAMPTZ
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let pg_us = i64::from_sql(buf)?;
        let unix_us = pg_us + PG_EPOCH_OFFSET_US;
        let secs = unix_us.div_euclid(1_000_000);
        let nanos = (unix_us.rem_euclid(1_000_000) * 1000) as i128;
        let total_nanos = i128::from(secs) * 1_000_000_000 + nanos;
        time::OffsetDateTime::from_unix_timestamp_nanos(total_nanos)
            .map_err(|e| Error::Decode(format!("timestamptz (time): {e}")))
    }
}

// ── PrimitiveDateTime → TIMESTAMP ───────────────────

impl ToSql for time::PrimitiveDateTime {
    fn oid(&self) -> Oid {
        Oid::TIMESTAMP
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        // Treat PrimitiveDateTime as UTC for wire encoding (same as chrono::NaiveDateTime)
        let as_utc = self.assume_utc();
        let unix_us = as_utc.unix_timestamp() * 1_000_000 + i64::from(as_utc.microsecond());
        let pg_us = unix_us - PG_EPOCH_OFFSET_US;
        buf.put_i64(pg_us);
        Ok(())
    }
}

impl FromSql for time::PrimitiveDateTime {
    fn oid() -> Oid {
        Oid::TIMESTAMP
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let odt = time::OffsetDateTime::from_sql(buf)?;
        Ok(time::PrimitiveDateTime::new(odt.date(), odt.time()))
    }
}

// ── Date → DATE ─────────────────────────────────────

impl ToSql for time::Date {
    fn oid(&self) -> Oid {
        Oid::DATE
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        let days = self.to_julian_day() - PG_EPOCH_JULIAN_DAY;
        buf.put_i32(days);
        Ok(())
    }
}

impl FromSql for time::Date {
    fn oid() -> Oid {
        Oid::DATE
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let days = i32::from_sql(buf)?;
        let julian = PG_EPOCH_JULIAN_DAY + days;
        time::Date::from_julian_day(julian).map_err(|e| Error::Decode(format!("date (time): {e}")))
    }
}

// ── Time → TIME ─────────────────────────────────────

impl ToSql for time::Time {
    fn oid(&self) -> Oid {
        Oid::TIME
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        let (h, m, s, us) = self.as_hms_micro();
        let total_us = i64::from(h) * 3_600_000_000
            + i64::from(m) * 60_000_000
            + i64::from(s) * 1_000_000
            + i64::from(us);
        buf.put_i64(total_us);
        Ok(())
    }
}

impl FromSql for time::Time {
    fn oid() -> Oid {
        Oid::TIME
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let us = i64::from_sql(buf)?;
        let total_secs = (us / 1_000_000) as u32;
        let micro = (us % 1_000_000) as u32;
        let h = (total_secs / 3600) as u8;
        let m = ((total_secs % 3600) / 60) as u8;
        let s = (total_secs % 60) as u8;
        time::Time::from_hms_micro(h, m, s, micro)
            .map_err(|e| Error::Decode(format!("time (time): {e}")))
    }
}
