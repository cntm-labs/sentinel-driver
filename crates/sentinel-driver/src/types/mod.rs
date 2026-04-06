pub mod bit;
pub mod builtin;
pub mod decode;
pub mod encode;
pub mod geometric;
pub mod interval;
pub mod lsn;
pub mod money;
pub mod network;
#[cfg(feature = "with-rust-decimal")]
pub mod numeric;
pub mod range;
pub mod xml;

use crate::error::Result;
use bytes::BytesMut;

// ── PostgreSQL Type OIDs ─────────────────────────────

/// Well-known PostgreSQL type OIDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Oid(pub u32);

impl Oid {
    pub const BOOL: Oid = Oid(16);
    pub const BYTEA: Oid = Oid(17);
    pub const CHAR: Oid = Oid(18);
    pub const INT8: Oid = Oid(20);
    pub const INT2: Oid = Oid(21);
    pub const INT4: Oid = Oid(23);
    pub const TEXT: Oid = Oid(25);
    pub const OID: Oid = Oid(26);
    pub const FLOAT4: Oid = Oid(700);
    pub const FLOAT8: Oid = Oid(701);
    pub const VARCHAR: Oid = Oid(1043);
    pub const DATE: Oid = Oid(1082);
    pub const TIME: Oid = Oid(1083);
    pub const TIMESTAMP: Oid = Oid(1114);
    pub const TIMESTAMPTZ: Oid = Oid(1184);
    pub const UUID: Oid = Oid(2950);
    pub const JSONB: Oid = Oid(3802);
    pub const JSON: Oid = Oid(114);
    pub const INET: Oid = Oid(869);
    pub const CIDR: Oid = Oid(650);
    pub const INET_ARRAY: Oid = Oid(1041);
    pub const CIDR_ARRAY: Oid = Oid(651);
    pub const MACADDR: Oid = Oid(829);
    pub const MACADDR_ARRAY: Oid = Oid(1040);
    pub const INTERVAL: Oid = Oid(1186);
    pub const INTERVAL_ARRAY: Oid = Oid(1187);
    pub const NUMERIC: Oid = Oid(1700);
    pub const NUMERIC_ARRAY: Oid = Oid(1231);
    pub const INT4RANGE: Oid = Oid(3904);
    pub const INT8RANGE: Oid = Oid(3926);
    pub const NUMRANGE: Oid = Oid(3906);
    pub const TSRANGE: Oid = Oid(3908);
    pub const TSTZRANGE: Oid = Oid(3910);
    pub const DATERANGE: Oid = Oid(3912);
    pub const INT4RANGE_ARRAY: Oid = Oid(3905);
    pub const INT8RANGE_ARRAY: Oid = Oid(3927);
    pub const NUMRANGE_ARRAY: Oid = Oid(3907);
    pub const TSRANGE_ARRAY: Oid = Oid(3909);
    pub const TSTZRANGE_ARRAY: Oid = Oid(3911);
    pub const DATERANGE_ARRAY: Oid = Oid(3913);
    pub const MONEY: Oid = Oid(790);
    pub const MONEY_ARRAY: Oid = Oid(791);
    pub const POINT: Oid = Oid(600);
    pub const LINE: Oid = Oid(628);
    pub const LSEG: Oid = Oid(601);
    pub const PG_BOX: Oid = Oid(603);
    pub const PATH: Oid = Oid(602);
    pub const POLYGON: Oid = Oid(604);
    pub const CIRCLE: Oid = Oid(718);
    pub const XML: Oid = Oid(142);
    pub const XML_ARRAY: Oid = Oid(143);
    pub const PG_LSN: Oid = Oid(3220);
    pub const PG_LSN_ARRAY: Oid = Oid(3221);
    pub const BIT: Oid = Oid(1560);
    pub const BIT_ARRAY: Oid = Oid(1561);
    pub const VARBIT: Oid = Oid(1562);
    pub const VARBIT_ARRAY: Oid = Oid(1563);

    // Array types
    pub const BOOL_ARRAY: Oid = Oid(1000);
    pub const INT2_ARRAY: Oid = Oid(1005);
    pub const INT4_ARRAY: Oid = Oid(1007);
    pub const INT8_ARRAY: Oid = Oid(1016);
    pub const FLOAT4_ARRAY: Oid = Oid(1021);
    pub const FLOAT8_ARRAY: Oid = Oid(1022);
    pub const TEXT_ARRAY: Oid = Oid(1009);
    pub const VARCHAR_ARRAY: Oid = Oid(1015);
    pub const UUID_ARRAY: Oid = Oid(2951);
}

impl From<u32> for Oid {
    fn from(v: u32) -> Self {
        Oid(v)
    }
}

impl From<Oid> for u32 {
    fn from(oid: Oid) -> Self {
        oid.0
    }
}

// ── Traits ───────────────────────────────────────────

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
