use bytes::{BufMut, BytesMut};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

/// PostgreSQL LTREE type -- a dot-separated label path for hierarchical data.
///
/// LTREE is a PostgreSQL extension type used for representing labels of data
/// stored in a hierarchical tree-like structure. Example: `"top.science.astronomy"`.
///
/// Wire format: UTF-8 text bytes (same as TEXT). Uses TEXT OID as carrier
/// since LTREE is an extension type without a stable OID.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PgLTree(pub String);

impl ToSql for PgLTree {
    fn oid(&self) -> Oid {
        Oid::TEXT
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_slice(self.0.as_bytes());
        Ok(())
    }
}

impl FromSql for PgLTree {
    fn oid() -> Oid {
        Oid::TEXT
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let s = String::from_utf8(buf.to_vec())
            .map_err(|e| Error::Decode(format!("ltree: invalid UTF-8: {e}")))?;
        Ok(PgLTree(s))
    }
}

impl std::fmt::Display for PgLTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for PgLTree {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(PgLTree(s.to_owned()))
    }
}

/// PostgreSQL LQUERY type -- a pattern for matching LTREE paths.
///
/// LQUERY extends LTREE with pattern-matching syntax including `*` (any label),
/// `*{n}` (exactly n labels), and `*{n,m}` (between n and m labels).
/// Example: `"*.science.*"` matches any path containing "science".
///
/// Wire format: UTF-8 text bytes (same as TEXT). Uses TEXT OID as carrier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PgLQuery(pub String);

impl ToSql for PgLQuery {
    fn oid(&self) -> Oid {
        Oid::TEXT
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_slice(self.0.as_bytes());
        Ok(())
    }
}

impl FromSql for PgLQuery {
    fn oid() -> Oid {
        Oid::TEXT
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let s = String::from_utf8(buf.to_vec())
            .map_err(|e| Error::Decode(format!("lquery: invalid UTF-8: {e}")))?;
        Ok(PgLQuery(s))
    }
}

impl std::fmt::Display for PgLQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for PgLQuery {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(PgLQuery(s.to_owned()))
    }
}
