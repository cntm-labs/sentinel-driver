use bytes::{BufMut, BytesMut};

use crate::error::Result;
use crate::types::Oid;
use crate::types::ToSql;

// ── Primitive types ──────────────────────────────────

impl ToSql for bool {
    fn oid(&self) -> Oid {
        Oid::BOOL
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(u8::from(*self));
        Ok(())
    }
}

impl ToSql for i16 {
    fn oid(&self) -> Oid {
        Oid::INT2
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_i16(*self);
        Ok(())
    }
}

impl ToSql for i32 {
    fn oid(&self) -> Oid {
        Oid::INT4
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_i32(*self);
        Ok(())
    }
}

impl ToSql for i64 {
    fn oid(&self) -> Oid {
        Oid::INT8
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_i64(*self);
        Ok(())
    }
}

impl ToSql for f32 {
    fn oid(&self) -> Oid {
        Oid::FLOAT4
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_f32(*self);
        Ok(())
    }
}

impl ToSql for f64 {
    fn oid(&self) -> Oid {
        Oid::FLOAT8
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_f64(*self);
        Ok(())
    }
}

// ── String types ─────────────────────────────────────

impl ToSql for &str {
    fn oid(&self) -> Oid {
        Oid::TEXT
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_slice(self.as_bytes());
        Ok(())
    }
}

impl ToSql for String {
    fn oid(&self) -> Oid {
        Oid::TEXT
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_slice(self.as_bytes());
        Ok(())
    }
}

// ── Byte types ───────────────────────────────────────

impl ToSql for &[u8] {
    fn oid(&self) -> Oid {
        Oid::BYTEA
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_slice(self);
        Ok(())
    }
}

impl ToSql for Vec<u8> {
    fn oid(&self) -> Oid {
        Oid::BYTEA
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_slice(self);
        Ok(())
    }
}

// ── chrono types ─────────────────────────────────────

/// PG epoch: 2000-01-01 00:00:00 UTC.
/// Unix epoch is 1970-01-01. Difference = 946684800 seconds.
const PG_EPOCH_OFFSET_US: i64 = 946_684_800_000_000;

impl ToSql for chrono::NaiveDateTime {
    fn oid(&self) -> Oid {
        Oid::TIMESTAMP
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        let us = self.and_utc().timestamp_micros() - PG_EPOCH_OFFSET_US;
        buf.put_i64(us);
        Ok(())
    }
}

impl ToSql for chrono::DateTime<chrono::Utc> {
    fn oid(&self) -> Oid {
        Oid::TIMESTAMPTZ
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        let us = self.timestamp_micros() - PG_EPOCH_OFFSET_US;
        buf.put_i64(us);
        Ok(())
    }
}

impl ToSql for chrono::NaiveDate {
    fn oid(&self) -> Oid {
        Oid::DATE
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        #[allow(clippy::expect_used)]
        let epoch = chrono::NaiveDate::from_ymd_opt(2000, 1, 1).expect("PG epoch is valid");
        let days = (*self - epoch).num_days() as i32;
        buf.put_i32(days);
        Ok(())
    }
}

impl ToSql for chrono::NaiveTime {
    fn oid(&self) -> Oid {
        Oid::TIME
    }

    #[allow(clippy::expect_used)]
    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        let midnight = chrono::NaiveTime::from_hms_opt(0, 0, 0).expect("midnight is valid");
        let us = self
            .signed_duration_since(midnight)
            .num_microseconds()
            .unwrap_or(0);
        buf.put_i64(us);
        Ok(())
    }
}

// ── UUID ─────────────────────────────────────────────

impl ToSql for uuid::Uuid {
    fn oid(&self) -> Oid {
        Oid::UUID
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_slice(self.as_bytes());
        Ok(())
    }
}
