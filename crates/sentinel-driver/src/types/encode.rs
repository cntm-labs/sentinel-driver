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
        if *self == chrono::NaiveDateTime::MAX {
            buf.put_i64(i64::MAX);
        } else if *self == chrono::NaiveDateTime::MIN {
            buf.put_i64(i64::MIN);
        } else {
            let us = self.and_utc().timestamp_micros() - PG_EPOCH_OFFSET_US;
            buf.put_i64(us);
        }
        Ok(())
    }
}

impl ToSql for chrono::DateTime<chrono::Utc> {
    fn oid(&self) -> Oid {
        Oid::TIMESTAMPTZ
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        if self.naive_utc() == chrono::NaiveDateTime::MAX {
            buf.put_i64(i64::MAX);
        } else if self.naive_utc() == chrono::NaiveDateTime::MIN {
            buf.put_i64(i64::MIN);
        } else {
            let us = self.timestamp_micros() - PG_EPOCH_OFFSET_US;
            buf.put_i64(us);
        }
        Ok(())
    }
}

impl ToSql for chrono::NaiveDate {
    fn oid(&self) -> Oid {
        Oid::DATE
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        if *self == chrono::NaiveDate::MAX {
            buf.put_i32(i32::MAX);
        } else if *self == chrono::NaiveDate::MIN {
            buf.put_i32(i32::MIN);
        } else {
            #[allow(clippy::expect_used)]
            let epoch = chrono::NaiveDate::from_ymd_opt(2000, 1, 1).expect("PG epoch is valid");
            let days = (*self - epoch).num_days() as i32;
            buf.put_i32(days);
        }
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

// ── Array types ─────────────────────────────────────

/// Encode a `Vec<T>` as a PostgreSQL 1-D binary array.
///
/// Wire format:
/// - i32 ndim (0 for empty, 1 for non-empty)
/// - i32 has_null (always 0 — nullable elements not supported)
/// - u32 element_oid
/// - i32 dim_len (array length)
/// - i32 dim_lbound (always 1, PG arrays are 1-based)
/// - for each element: i32 data_len + encoded bytes
fn encode_array<T: ToSql>(vec: &[T], element_oid: Oid, buf: &mut BytesMut) -> Result<()> {
    if vec.is_empty() {
        buf.put_i32(0); // ndim = 0
        buf.put_i32(0); // has_null = 0
        buf.put_u32(element_oid.0);
        return Ok(());
    }

    buf.put_i32(1); // ndim = 1
    buf.put_i32(0); // has_null = 0
    buf.put_u32(element_oid.0);
    buf.put_i32(vec.len() as i32); // dim_len
    buf.put_i32(1); // dim_lbound (1-based)

    for elem in vec {
        let len_pos = buf.len();
        buf.put_i32(0); // placeholder for element length
        let data_start = buf.len();
        elem.to_sql(buf)?;
        let data_len = (buf.len() - data_start) as i32;
        buf[len_pos..len_pos + 4].copy_from_slice(&data_len.to_be_bytes());
    }

    Ok(())
}

/// Macro to implement `ToSql` for `Vec<T>` for a specific element type.
macro_rules! impl_array_to_sql {
    ($elem_ty:ty, $array_oid:expr, $elem_oid:expr) => {
        impl ToSql for Vec<$elem_ty> {
            fn oid(&self) -> Oid {
                $array_oid
            }

            fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
                encode_array(self, $elem_oid, buf)
            }
        }
    };
}

impl_array_to_sql!(bool, Oid::BOOL_ARRAY, Oid::BOOL);
impl_array_to_sql!(i16, Oid::INT2_ARRAY, Oid::INT2);
impl_array_to_sql!(i32, Oid::INT4_ARRAY, Oid::INT4);
impl_array_to_sql!(i64, Oid::INT8_ARRAY, Oid::INT8);
impl_array_to_sql!(f32, Oid::FLOAT4_ARRAY, Oid::FLOAT4);
impl_array_to_sql!(f64, Oid::FLOAT8_ARRAY, Oid::FLOAT8);
impl_array_to_sql!(String, Oid::TEXT_ARRAY, Oid::TEXT);
impl_array_to_sql!(uuid::Uuid, Oid::UUID_ARRAY, Oid::UUID);
impl_array_to_sql!(
    crate::types::interval::PgInterval,
    Oid::INTERVAL_ARRAY,
    Oid::INTERVAL
);
impl_array_to_sql!(crate::types::money::PgMoney, Oid::MONEY_ARRAY, Oid::MONEY);
impl_array_to_sql!(crate::types::xml::PgXml, Oid::XML_ARRAY, Oid::XML);
impl_array_to_sql!(crate::types::lsn::PgLsn, Oid::PG_LSN_ARRAY, Oid::PG_LSN);
impl_array_to_sql!(crate::types::network::PgInet, Oid::INET_ARRAY, Oid::INET);
impl_array_to_sql!(crate::types::network::PgCidr, Oid::CIDR_ARRAY, Oid::CIDR);
impl_array_to_sql!(
    crate::types::network::PgMacAddr,
    Oid::MACADDR_ARRAY,
    Oid::MACADDR
);
#[cfg(feature = "with-rust-decimal")]
impl_array_to_sql!(rust_decimal::Decimal, Oid::NUMERIC_ARRAY, Oid::NUMERIC);
impl_array_to_sql!(crate::types::bit::PgBit, Oid::VARBIT_ARRAY, Oid::VARBIT);

impl ToSql for Vec<&str> {
    fn oid(&self) -> Oid {
        Oid::TEXT_ARRAY
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        encode_array(self, Oid::TEXT, buf)
    }
}
