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

// ── Array types ─────────────────────────────────────

/// Decode a PostgreSQL 1-D binary array into `Vec<T>`.
fn decode_array<T: FromSql>(buf: &[u8], expected_elem_oid: Oid) -> Result<Vec<T>> {
    if buf.len() < 12 {
        return Err(Error::Decode("array: header too short".into()));
    }

    let ndim = i32::from_be_bytes(
        buf[0..4]
            .try_into()
            .map_err(|_| Error::Decode("array: invalid ndim".into()))?,
    );

    // has_null at buf[4..8] — we reject NULLs in element loop

    let elem_oid = u32::from_be_bytes(
        buf[8..12]
            .try_into()
            .map_err(|_| Error::Decode("array: invalid element oid".into()))?,
    );

    if ndim == 0 {
        return Ok(Vec::new());
    }

    if ndim != 1 {
        return Err(Error::Decode(format!(
            "array: multi-dimensional arrays not supported (ndim={ndim})"
        )));
    }

    if elem_oid != expected_elem_oid.0 {
        return Err(Error::Decode(format!(
            "array: expected element OID {}, got {elem_oid}",
            expected_elem_oid.0
        )));
    }

    if buf.len() < 20 {
        return Err(Error::Decode("array: dimension header too short".into()));
    }

    let dim_len = i32::from_be_bytes(
        buf[12..16]
            .try_into()
            .map_err(|_| Error::Decode("array: invalid dim_len".into()))?,
    ) as usize;

    // dim_lbound at buf[16..20] — skip, not needed for decoding

    let mut offset = 20;
    let mut result = Vec::with_capacity(dim_len);

    for _ in 0..dim_len {
        if offset + 4 > buf.len() {
            return Err(Error::Decode("array: unexpected end of data".into()));
        }

        let elem_len = i32::from_be_bytes(
            buf[offset..offset + 4]
                .try_into()
                .map_err(|_| Error::Decode("array: invalid element length".into()))?,
        );
        offset += 4;

        if elem_len < 0 {
            return Err(Error::Decode("array: NULL elements not supported".into()));
        }

        let elem_len = elem_len as usize;
        if offset + elem_len > buf.len() {
            return Err(Error::Decode("array: element data truncated".into()));
        }

        let elem = T::from_sql(&buf[offset..offset + elem_len])?;
        result.push(elem);
        offset += elem_len;
    }

    Ok(result)
}

/// Macro to implement `FromSql` for `Vec<T>` for a specific element type.
macro_rules! impl_array_from_sql {
    ($elem_ty:ty, $array_oid:expr, $elem_oid:expr) => {
        impl FromSql for Vec<$elem_ty> {
            fn oid() -> Oid {
                $array_oid
            }

            fn from_sql(buf: &[u8]) -> Result<Self> {
                decode_array::<$elem_ty>(buf, $elem_oid)
            }
        }
    };
}

impl_array_from_sql!(bool, Oid::BOOL_ARRAY, Oid::BOOL);
impl_array_from_sql!(i16, Oid::INT2_ARRAY, Oid::INT2);
impl_array_from_sql!(i32, Oid::INT4_ARRAY, Oid::INT4);
impl_array_from_sql!(i64, Oid::INT8_ARRAY, Oid::INT8);
impl_array_from_sql!(f32, Oid::FLOAT4_ARRAY, Oid::FLOAT4);
impl_array_from_sql!(f64, Oid::FLOAT8_ARRAY, Oid::FLOAT8);
impl_array_from_sql!(String, Oid::TEXT_ARRAY, Oid::TEXT);
impl_array_from_sql!(uuid::Uuid, Oid::UUID_ARRAY, Oid::UUID);
