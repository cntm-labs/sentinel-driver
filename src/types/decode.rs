use crate::error::{Error, Result};
use crate::types::Oid;
use crate::types::FromSql;

// ── Primitive types ──────────────────────────────────

impl FromSql for bool {
    fn oid() -> Oid { Oid::BOOL }

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
    fn oid() -> Oid { Oid::INT2 }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let arr: [u8; 2] = buf
            .try_into()
            .map_err(|_| Error::Decode(format!("int2: expected 2 bytes, got {}", buf.len())))?;
        Ok(i16::from_be_bytes(arr))
    }
}

impl FromSql for i32 {
    fn oid() -> Oid { Oid::INT4 }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let arr: [u8; 4] = buf
            .try_into()
            .map_err(|_| Error::Decode(format!("int4: expected 4 bytes, got {}", buf.len())))?;
        Ok(i32::from_be_bytes(arr))
    }
}

impl FromSql for i64 {
    fn oid() -> Oid { Oid::INT8 }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let arr: [u8; 8] = buf
            .try_into()
            .map_err(|_| Error::Decode(format!("int8: expected 8 bytes, got {}", buf.len())))?;
        Ok(i64::from_be_bytes(arr))
    }
}

impl FromSql for f32 {
    fn oid() -> Oid { Oid::FLOAT4 }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let arr: [u8; 4] = buf
            .try_into()
            .map_err(|_| Error::Decode(format!("float4: expected 4 bytes, got {}", buf.len())))?;
        Ok(f32::from_be_bytes(arr))
    }
}

impl FromSql for f64 {
    fn oid() -> Oid { Oid::FLOAT8 }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let arr: [u8; 8] = buf
            .try_into()
            .map_err(|_| Error::Decode(format!("float8: expected 8 bytes, got {}", buf.len())))?;
        Ok(f64::from_be_bytes(arr))
    }
}

// ── String types ─────────────────────────────────────

impl FromSql for String {
    fn oid() -> Oid { Oid::TEXT }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        String::from_utf8(buf.to_vec())
            .map_err(|e| Error::Decode(format!("text: invalid UTF-8: {e}")))
    }
}

// ── Byte types ───────────────────────────────────────

impl FromSql for Vec<u8> {
    fn oid() -> Oid { Oid::BYTEA }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        Ok(buf.to_vec())
    }
}

// ── chrono types ─────────────────────────────────────

/// PG epoch offset in microseconds from Unix epoch.
const PG_EPOCH_OFFSET_US: i64 = 946_684_800_000_000;

impl FromSql for chrono::NaiveDateTime {
    fn oid() -> Oid { Oid::TIMESTAMP }

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
    fn oid() -> Oid { Oid::TIMESTAMPTZ }

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
    fn oid() -> Oid { Oid::DATE }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let days = i32::from_sql(buf)?;
        let epoch = chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        epoch
            .checked_add_signed(chrono::Duration::days(days as i64))
            .ok_or_else(|| Error::Decode(format!("date out of range: {days} days from epoch")))
    }
}

impl FromSql for chrono::NaiveTime {
    fn oid() -> Oid { Oid::TIME }

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
    fn oid() -> Oid { Oid::UUID }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let arr: [u8; 16] = buf
            .try_into()
            .map_err(|_| Error::Decode(format!("uuid: expected 16 bytes, got {}", buf.len())))?;
        Ok(uuid::Uuid::from_bytes(arr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use crate::types::ToSql;

    /// Helper: encode then decode, verify roundtrip.
    fn roundtrip<T: ToSql + FromSql + std::fmt::Debug + PartialEq>(val: &T) {
        let mut buf = BytesMut::new();
        val.to_sql(&mut buf).unwrap();
        let decoded = T::from_sql(&buf).unwrap();
        assert_eq!(&decoded, val);
    }

    #[test]
    fn test_roundtrip_bool() {
        roundtrip(&true);
        roundtrip(&false);
    }

    #[test]
    fn test_roundtrip_i16() {
        roundtrip(&0i16);
        roundtrip(&i16::MIN);
        roundtrip(&i16::MAX);
        roundtrip(&-42i16);
    }

    #[test]
    fn test_roundtrip_i32() {
        roundtrip(&0i32);
        roundtrip(&i32::MIN);
        roundtrip(&i32::MAX);
        roundtrip(&42i32);
    }

    #[test]
    fn test_roundtrip_i64() {
        roundtrip(&0i64);
        roundtrip(&i64::MIN);
        roundtrip(&i64::MAX);
    }

    #[test]
    fn test_roundtrip_f32() {
        roundtrip(&0.0f32);
        roundtrip(&3.14f32);
        roundtrip(&-1.0f32);
    }

    #[test]
    fn test_roundtrip_f64() {
        roundtrip(&0.0f64);
        roundtrip(&std::f64::consts::PI);
        roundtrip(&-1.0f64);
    }

    #[test]
    fn test_roundtrip_string() {
        roundtrip(&String::from("hello world"));
        roundtrip(&String::from(""));
        roundtrip(&String::from("日本語テスト"));
    }

    #[test]
    fn test_roundtrip_bytes() {
        roundtrip(&vec![0xDE, 0xAD, 0xBE, 0xEF]);
        roundtrip(&vec![]);
    }

    #[test]
    fn test_roundtrip_uuid() {
        let id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        roundtrip(&id);
        roundtrip(&uuid::Uuid::nil());
    }

    #[test]
    fn test_roundtrip_naive_date() {
        let date = chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        roundtrip(&date);

        let date = chrono::NaiveDate::from_ymd_opt(2026, 4, 3).unwrap();
        roundtrip(&date);

        let date = chrono::NaiveDate::from_ymd_opt(1999, 12, 31).unwrap();
        roundtrip(&date);
    }

    #[test]
    fn test_roundtrip_naive_datetime() {
        let dt = chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        roundtrip(&dt);

        let dt = chrono::NaiveDate::from_ymd_opt(2026, 4, 3)
            .unwrap()
            .and_hms_micro_opt(12, 30, 45, 123456)
            .unwrap();
        roundtrip(&dt);
    }

    #[test]
    fn test_roundtrip_datetime_utc() {
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap();
        roundtrip(&dt);

        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(1712150400, 500_000_000).unwrap();
        roundtrip(&dt);
    }

    #[test]
    fn test_roundtrip_naive_time() {
        let t = chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        roundtrip(&t);

        let t = chrono::NaiveTime::from_hms_micro_opt(23, 59, 59, 999999).unwrap();
        roundtrip(&t);
    }

    #[test]
    fn test_decode_wrong_size() {
        assert!(i32::from_sql(&[0, 0]).is_err());
        assert!(bool::from_sql(&[]).is_err());
        assert!(uuid::Uuid::from_sql(&[0; 15]).is_err());
    }

    #[test]
    fn test_option_from_sql() {
        let result: Option<i32> = FromSql::from_sql_nullable(None).unwrap();
        assert_eq!(result, None);

        let buf = 42i32.to_be_bytes();
        let result: Option<i32> = FromSql::from_sql_nullable(Some(&buf)).unwrap();
        assert_eq!(result, Some(42));
    }
}
