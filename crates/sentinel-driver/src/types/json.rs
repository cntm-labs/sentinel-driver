use bytes::{BufMut, BytesMut};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

/// JSONB version byte prepended to the binary wire format.
const JSONB_VERSION: u8 = 1;

/// A wrapper for serializing and deserializing Rust types as PostgreSQL JSONB.
///
/// Wraps any `T` that implements `serde::Serialize` (for encoding) or
/// `serde::de::DeserializeOwned` (for decoding). The wire format uses
/// JSONB (OID 3802) with a version-1 prefix byte.
///
/// # Example
///
/// ```rust,ignore
/// use sentinel_driver::Json;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct UserData {
///     name: String,
///     age: u32,
/// }
///
/// let data = Json(UserData { name: "Alice".into(), age: 30 });
/// conn.execute("INSERT INTO users (data) VALUES ($1)", &[&data]).await?;
///
/// let row = conn.query_one("SELECT data FROM users LIMIT 1", &[]).await?;
/// let data: Json<UserData> = row.get(0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Json<T>(pub T);

impl<T: serde::Serialize> ToSql for Json<T> {
    fn oid(&self) -> Oid {
        Oid::JSONB
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(JSONB_VERSION);
        serde_json::to_writer(buf.writer(), &self.0)
            .map_err(|e| Error::Encode(format!("jsonb: serialization failed: {e}")))?;
        Ok(())
    }
}

impl<T: serde::de::DeserializeOwned> FromSql for Json<T> {
    fn oid() -> Oid {
        Oid::JSONB
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        // JSONB binary format: first byte is version (1), rest is JSON
        let data = if buf.first() == Some(&JSONB_VERSION) {
            &buf[1..]
        } else {
            buf
        };
        let value = serde_json::from_slice(data)
            .map_err(|e| Error::Decode(format!("jsonb: deserialization failed: {e}")))?;
        Ok(Json(value))
    }
}

// ── serde_json::Value direct support ────────────────

impl ToSql for serde_json::Value {
    fn oid(&self) -> Oid {
        Oid::JSONB
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(JSONB_VERSION);
        serde_json::to_writer(buf.writer(), self)
            .map_err(|e| Error::Encode(format!("jsonb: serialization failed: {e}")))?;
        Ok(())
    }
}

impl FromSql for serde_json::Value {
    fn oid() -> Oid {
        Oid::JSONB
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let data = if buf.first() == Some(&JSONB_VERSION) {
            &buf[1..]
        } else {
            buf
        };
        serde_json::from_slice(data)
            .map_err(|e| Error::Decode(format!("jsonb: deserialization failed: {e}")))
    }
}
