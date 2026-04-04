use bytes::{BufMut, BytesMut};

use crate::error::Result;
use crate::types::Oid;
use crate::types::ToSql;

// ── Primitive types ──────────────────────────────────

impl ToSql for bool {
    fn oid(&self) -> Oid { Oid::BOOL }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(if *self { 1 } else { 0 });
        Ok(())
    }
}

impl ToSql for i16 {
    fn oid(&self) -> Oid { Oid::INT2 }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_i16(*self);
        Ok(())
    }
}

impl ToSql for i32 {
    fn oid(&self) -> Oid { Oid::INT4 }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_i32(*self);
        Ok(())
    }
}

impl ToSql for i64 {
    fn oid(&self) -> Oid { Oid::INT8 }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_i64(*self);
        Ok(())
    }
}

impl ToSql for f32 {
    fn oid(&self) -> Oid { Oid::FLOAT4 }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_f32(*self);
        Ok(())
    }
}

impl ToSql for f64 {
    fn oid(&self) -> Oid { Oid::FLOAT8 }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_f64(*self);
        Ok(())
    }
}

// ── String types ─────────────────────────────────────

impl ToSql for &str {
    fn oid(&self) -> Oid { Oid::TEXT }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_slice(self.as_bytes());
        Ok(())
    }
}

impl ToSql for String {
    fn oid(&self) -> Oid { Oid::TEXT }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_slice(self.as_bytes());
        Ok(())
    }
}

// ── Byte types ───────────────────────────────────────

impl ToSql for &[u8] {
    fn oid(&self) -> Oid { Oid::BYTEA }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_slice(self);
        Ok(())
    }
}

impl ToSql for Vec<u8> {
    fn oid(&self) -> Oid { Oid::BYTEA }

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
    fn oid(&self) -> Oid { Oid::TIMESTAMP }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        let us = self.and_utc().timestamp_micros() - PG_EPOCH_OFFSET_US;
        buf.put_i64(us);
        Ok(())
    }
}

impl ToSql for chrono::DateTime<chrono::Utc> {
    fn oid(&self) -> Oid { Oid::TIMESTAMPTZ }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        let us = self.timestamp_micros() - PG_EPOCH_OFFSET_US;
        buf.put_i64(us);
        Ok(())
    }
}

impl ToSql for chrono::NaiveDate {
    fn oid(&self) -> Oid { Oid::DATE }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        let epoch = chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let days = (*self - epoch).num_days() as i32;
        buf.put_i32(days);
        Ok(())
    }
}

impl ToSql for chrono::NaiveTime {
    fn oid(&self) -> Oid { Oid::TIME }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        let us = self
            .signed_duration_since(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .num_microseconds()
            .unwrap_or(0);
        buf.put_i64(us);
        Ok(())
    }
}

// ── UUID ─────────────────────────────────────────────

impl ToSql for uuid::Uuid {
    fn oid(&self) -> Oid { Oid::UUID }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_slice(self.as_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_bool() {
        let mut buf = BytesMut::new();
        true.to_sql(&mut buf).unwrap();
        assert_eq!(&buf[..], &[1]);

        buf.clear();
        false.to_sql(&mut buf).unwrap();
        assert_eq!(&buf[..], &[0]);
    }

    #[test]
    fn test_encode_i32() {
        let mut buf = BytesMut::new();
        42i32.to_sql(&mut buf).unwrap();
        assert_eq!(&buf[..], &42i32.to_be_bytes());
    }

    #[test]
    fn test_encode_i64() {
        let mut buf = BytesMut::new();
        123456789i64.to_sql(&mut buf).unwrap();
        assert_eq!(&buf[..], &123456789i64.to_be_bytes());
    }

    #[test]
    fn test_encode_f64() {
        let mut buf = BytesMut::new();
        3.14f64.to_sql(&mut buf).unwrap();
        assert_eq!(&buf[..], &3.14f64.to_be_bytes());
    }

    #[test]
    fn test_encode_str() {
        let mut buf = BytesMut::new();
        "hello".to_sql(&mut buf).unwrap();
        assert_eq!(&buf[..], b"hello");
    }

    #[test]
    fn test_encode_string() {
        let mut buf = BytesMut::new();
        String::from("world").to_sql(&mut buf).unwrap();
        assert_eq!(&buf[..], b"world");
    }

    #[test]
    fn test_encode_bytes() {
        let mut buf = BytesMut::new();
        let data: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF];
        data.to_sql(&mut buf).unwrap();
        assert_eq!(&buf[..], &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_encode_uuid() {
        let id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let mut buf = BytesMut::new();
        id.to_sql(&mut buf).unwrap();
        assert_eq!(&buf[..], id.as_bytes());
        assert_eq!(buf.len(), 16);
    }

    #[test]
    fn test_encode_naive_date() {
        // 2000-01-01 should encode as 0
        let date = chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let mut buf = BytesMut::new();
        date.to_sql(&mut buf).unwrap();
        assert_eq!(&buf[..], &0i32.to_be_bytes());

        // 2000-01-02 should encode as 1
        buf.clear();
        let date = chrono::NaiveDate::from_ymd_opt(2000, 1, 2).unwrap();
        date.to_sql(&mut buf).unwrap();
        assert_eq!(&buf[..], &1i32.to_be_bytes());
    }

    #[test]
    fn test_encode_timestamp() {
        // PG epoch: 2000-01-01 00:00:00 should encode as 0
        let dt = chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let mut buf = BytesMut::new();
        dt.to_sql(&mut buf).unwrap();
        assert_eq!(&buf[..], &0i64.to_be_bytes());
    }
}
