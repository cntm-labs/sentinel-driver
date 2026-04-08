use std::collections::HashMap;

use bytes::{Buf, BufMut, BytesMut};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

#[allow(clippy::implicit_hasher)]
impl ToSql for HashMap<String, Option<String>> {
    fn oid(&self) -> Oid {
        // HSTORE is an extension type — use TEXT OID as carrier.
        // The server resolves the actual type via parameter binding.
        Oid::TEXT
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_i32(self.len() as i32);
        for (key, val) in self {
            let key_bytes = key.as_bytes();
            buf.put_i32(key_bytes.len() as i32);
            buf.put_slice(key_bytes);
            match val {
                Some(v) => {
                    let val_bytes = v.as_bytes();
                    buf.put_i32(val_bytes.len() as i32);
                    buf.put_slice(val_bytes);
                }
                None => {
                    buf.put_i32(-1);
                }
            }
        }
        Ok(())
    }
}

#[allow(clippy::implicit_hasher)]
impl FromSql for HashMap<String, Option<String>> {
    fn oid() -> Oid {
        Oid::TEXT
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        if buf.len() < 4 {
            return Err(Error::Decode("hstore: buffer too short for count".into()));
        }
        let mut cur = buf;
        let count = cur.get_i32();
        if count < 0 {
            return Err(Error::Decode("hstore: negative count".into()));
        }
        let mut map = HashMap::with_capacity(count as usize);

        for _ in 0..count {
            if cur.remaining() < 4 {
                return Err(Error::Decode("hstore: truncated key length".into()));
            }
            let key_len = cur.get_i32();
            if key_len < 0 {
                return Err(Error::Decode("hstore: negative key length".into()));
            }
            let key_len = key_len as usize;
            if cur.remaining() < key_len {
                return Err(Error::Decode("hstore: truncated key data".into()));
            }
            let key = String::from_utf8(cur[..key_len].to_vec())
                .map_err(|e| Error::Decode(format!("hstore: invalid UTF-8 key: {e}")))?;
            cur.advance(key_len);

            if cur.remaining() < 4 {
                return Err(Error::Decode("hstore: truncated value length".into()));
            }
            let val_len = cur.get_i32();
            let val = if val_len < 0 {
                None
            } else {
                let val_len = val_len as usize;
                if cur.remaining() < val_len {
                    return Err(Error::Decode("hstore: truncated value data".into()));
                }
                let v = String::from_utf8(cur[..val_len].to_vec())
                    .map_err(|e| Error::Decode(format!("hstore: invalid UTF-8 value: {e}")))?;
                cur.advance(val_len);
                Some(v)
            };

            map.insert(key, val);
        }
        Ok(map)
    }
}
