use crate::error::Result;
use crate::types::Oid;
use bytes::BytesMut;

/// Encode a Rust value into PostgreSQL binary format.
///
/// Implementations write the value's binary representation into `buf`.
/// The caller is responsible for writing the length prefix.
pub trait ToSql {
    /// The PostgreSQL type OID for this Rust type.
    fn oid(&self) -> Oid;

    /// Encode this value into PG binary format, appending to `buf`.
    fn to_sql(&self, buf: &mut BytesMut) -> Result<()>;

    /// Encode this value into a standalone byte vector for use as a bind parameter.
    fn to_sql_vec(&self) -> Result<Vec<u8>> {
        let mut buf = BytesMut::new();
        self.to_sql(&mut buf)?;
        Ok(buf.to_vec())
    }
}

/// Decode a Rust value from PostgreSQL binary format.
///
/// `buf` contains the raw column bytes (without the length prefix).
pub trait FromSql: Sized {
    /// The PostgreSQL type OID this decoder handles.
    fn oid() -> Oid;

    /// Decode from PG binary format.
    fn from_sql(buf: &[u8]) -> Result<Self>;

    /// Decode from a potentially NULL column.
    fn from_sql_nullable(buf: Option<&[u8]>) -> Result<Self> {
        match buf {
            Some(b) => Self::from_sql(b),
            None => Err(crate::error::Error::Decode(
                "unexpected NULL value".to_string(),
            )),
        }
    }
}

/// Marker trait for types that can be NULL (Option<T>).
impl<T: FromSql> FromSql for Option<T> {
    fn oid() -> Oid {
        T::oid()
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        T::from_sql(buf).map(Some)
    }

    fn from_sql_nullable(buf: Option<&[u8]>) -> Result<Self> {
        match buf {
            Some(b) => T::from_sql(b).map(Some),
            None => Ok(None),
        }
    }
}

/// ToSql for Option<T> — encodes as NULL when None.
impl<T: ToSql> ToSql for Option<T> {
    fn oid(&self) -> Oid {
        match self {
            Some(v) => v.oid(),
            // Default to TEXT for NULL; the server infers the actual type.
            None => Oid::TEXT,
        }
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        match self {
            Some(v) => v.to_sql(buf),
            None => Ok(()), // caller handles NULL encoding (-1 length)
        }
    }
}

/// Encode a value as a bind parameter (Some = value, None = NULL).
pub fn encode_param<T: ToSql>(val: &T) -> Result<Vec<u8>> {
    val.to_sql_vec()
}

/// Encode an optional value as a bind parameter.
pub fn encode_param_nullable<T: ToSql>(val: &Option<T>) -> Result<Option<Vec<u8>>> {
    match val {
        Some(v) => Ok(Some(v.to_sql_vec()?)),
        None => Ok(None),
    }
}
