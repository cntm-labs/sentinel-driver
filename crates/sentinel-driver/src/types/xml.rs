use bytes::{BufMut, BytesMut};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

/// PostgreSQL XML type -- a validated XML string.
///
/// On the wire, XML is just UTF-8 text with the XML OID.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PgXml(pub String);

impl ToSql for PgXml {
    fn oid(&self) -> Oid {
        Oid::XML
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_slice(self.0.as_bytes());
        Ok(())
    }
}

impl FromSql for PgXml {
    fn oid() -> Oid {
        Oid::XML
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let s = String::from_utf8(buf.to_vec())
            .map_err(|e| Error::Decode(format!("xml: invalid UTF-8: {e}")))?;
        Ok(PgXml(s))
    }
}
