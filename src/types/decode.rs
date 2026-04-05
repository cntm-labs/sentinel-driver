use crate::error::{Error, Result};
use crate::types::FromSql;
use crate::types::Oid;

// ── Primitive types ──────────────────────────────────

impl FromSql for bool {
    fn oid() -> Oid {
        Oid::BOOL
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        if buf.len() != 1 {
            return Err(Error::Decode(format!(
                "bool: expected 1 byte, got {}",
                buf.len()
            )));
        }
        Ok(buf[0] != 0)
    }
}

impl FromSql for i16 {
    fn oid() -> Oid {
        Oid::INT2
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let arr: [u8; 2] = buf
            .try_into()
            .map_err(|_| Error::Decode(format!("int2: expected 2 bytes, got {}", buf.len())))?;
        Ok(i16::from_be_bytes(arr))
    }
}

impl FromSql for i32 {
    fn oid() -> Oid {
        Oid::INT4
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let arr: [u8; 4] = buf
            .try_into()
            .map_err(|_| Error::Decode(format!("int4: expected 4 bytes, got {}", buf.len())))?;
        Ok(i32::from_be_bytes(arr))
    }
}

impl FromSql for i64 {
    fn oid() -> Oid {
        Oid::INT8
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let arr: [u8; 8] = buf
            .try_into()
            .map_err(|_| Error::Decode(format!("int8: expected 8 bytes, got {}", buf.len())))?;
        Ok(i64::from_be_bytes(arr))
    }
}

impl FromSql for f32 {
    fn oid() -> Oid {
        Oid::FLOAT4
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let arr: [u8; 4] = buf
            .try_into()
            .map_err(|_| Error::Decode(format!("float4: expected 4 bytes, got {}", buf.len())))?;
        Ok(f32::from_be_bytes(arr))
    }
}

impl FromSql for f64 {
    fn oid() -> Oid {
        Oid::FLOAT8
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let arr: [u8; 8] = buf
            .try_into()
            .map_err(|_| Error::Decode(format!("float8: expected 8 bytes, got {}", buf.len())))?;
        Ok(f64::from_be_bytes(arr))
    }
}

// ── String types ─────────────────────────────────────

impl FromSql for String {
    fn oid() -> Oid {
        Oid::TEXT
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        String::from_utf8(buf.to_vec())
            .map_err(|e| Error::Decode(format!("text: invalid UTF-8: {e}")))
    }
}

// ── Byte types ───────────────────────────────────────

impl FromSql for Vec<u8> {
    fn oid() -> Oid {
        Oid::BYTEA
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        Ok(buf.to_vec())
    }
}

// ── chrono types ─────────────────────────────────────

/// PG epoch offset in microseconds from Unix epoch.
const PG_EPOCH_OFFSET_US: i64 = 946_684_800_000_000;

impl FromSql for chrono::NaiveDateTime {
    fn oid() -> Oid {
        Oid::TIMESTAMP
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let us_from_pg_epoch = i64::from_sql(buf)?;
        let us_from_unix_epoch = us_from_pg_epoch + PG_EPOCH_OFFSET_US;
        let secs = us_from_unix_epoch.div_euclid(1_000_000);
        let nsecs = (us_from_unix_epoch.rem_euclid(1_000_000) * 1000) as u32;
        chrono::DateTime::from_timestamp(secs, nsecs)
            .map(|dt| dt.naive_utc())
            .ok_or_else(|| Error::Decode(format!("timestamp out of range: {us_from_pg_epoch}")))
    }
}

impl FromSql for chrono::DateTime<chrono::Utc> {
    fn oid() -> Oid {
        Oid::TIMESTAMPTZ
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let us_from_pg_epoch = i64::from_sql(buf)?;
        let us_from_unix_epoch = us_from_pg_epoch + PG_EPOCH_OFFSET_US;
        let secs = us_from_unix_epoch.div_euclid(1_000_000);
        let nsecs = (us_from_unix_epoch.rem_euclid(1_000_000) * 1000) as u32;
        chrono::DateTime::from_timestamp(secs, nsecs)
            .ok_or_else(|| Error::Decode(format!("timestamptz out of range: {us_from_pg_epoch}")))
    }
}

impl FromSql for chrono::NaiveDate {
    fn oid() -> Oid {
        Oid::DATE
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let days = i32::from_sql(buf)?;
        #[allow(clippy::expect_used)]
        let epoch = chrono::NaiveDate::from_ymd_opt(2000, 1, 1).expect("PG epoch is valid");
        epoch
            .checked_add_signed(chrono::Duration::days(days as i64))
            .ok_or_else(|| Error::Decode(format!("date out of range: {days} days from epoch")))
    }
}

impl FromSql for chrono::NaiveTime {
    fn oid() -> Oid {
        Oid::TIME
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let us = i64::from_sql(buf)?;
        let secs = (us / 1_000_000) as u32;
        let micro = (us % 1_000_000) as u32;
        chrono::NaiveTime::from_num_seconds_from_midnight_opt(secs, micro * 1000)
            .ok_or_else(|| Error::Decode(format!("time out of range: {us} microseconds")))
    }
}

// ── UUID ─────────────────────────────────────────────

impl FromSql for uuid::Uuid {
    fn oid() -> Oid {
        Oid::UUID
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let arr: [u8; 16] = buf
            .try_into()
            .map_err(|_| Error::Decode(format!("uuid: expected 16 bytes, got {}", buf.len())))?;
        Ok(uuid::Uuid::from_bytes(arr))
    }
}
