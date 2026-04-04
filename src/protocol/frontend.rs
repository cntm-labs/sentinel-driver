use bytes::{BufMut, BytesMut};

/// Encode the startup message (no type byte — special case).
///
/// Format: `[length: i32][protocol_version: i32][param_name\0param_value\0...]\0`
pub fn startup(buf: &mut BytesMut, user: &str, database: &str, params: &[(&str, &str)]) {
    let start = buf.len();
    buf.put_i32(0); // placeholder for length
    buf.put_i32(196608); // protocol version 3.0

    put_cstr(buf, "user");
    put_cstr(buf, user);

    if !database.is_empty() {
        put_cstr(buf, "database");
        put_cstr(buf, database);
    }

    for &(key, value) in params {
        put_cstr(buf, key);
        put_cstr(buf, value);
    }

    buf.put_u8(0); // terminator

    let len = (buf.len() - start) as i32;
    buf[start..start + 4].copy_from_slice(&len.to_be_bytes());
}

/// SSLRequest message — asks server if TLS is supported.
///
/// Format: `[length: i32 = 8][magic: i32 = 80877103]`
pub fn ssl_request(buf: &mut BytesMut) {
    buf.put_i32(8);
    buf.put_i32(80877103);
}

/// Simple Query (Q).
pub fn query(buf: &mut BytesMut, sql: &str) {
    encode_message(buf, b'Q', |buf| {
        put_cstr(buf, sql);
    });
}

/// Parse (P) — prepare a statement.
pub fn parse(buf: &mut BytesMut, name: &str, sql: &str, param_types: &[u32]) {
    encode_message(buf, b'P', |buf| {
        put_cstr(buf, name);
        put_cstr(buf, sql);
        buf.put_i16(param_types.len() as i16);
        for &oid in param_types {
            buf.put_u32(oid);
        }
    });
}

/// Bind (B) — bind parameters to a prepared statement.
///
/// `params` are pre-encoded binary values. `None` represents NULL.
pub fn bind(
    buf: &mut BytesMut,
    portal: &str,
    statement: &str,
    params: &[Option<&[u8]>],
    result_formats: &[i16],
) {
    encode_message(buf, b'B', |buf| {
        put_cstr(buf, portal);
        put_cstr(buf, statement);

        // Parameter format codes: all binary (1)
        buf.put_i16(1); // one format code
        buf.put_i16(1); // binary

        // Parameters
        buf.put_i16(params.len() as i16);
        for param in params {
            match param {
                Some(data) => {
                    buf.put_i32(data.len() as i32);
                    buf.put_slice(data);
                }
                None => {
                    buf.put_i32(-1); // NULL
                }
            }
        }

        // Result format codes
        if result_formats.is_empty() {
            buf.put_i16(1); // one format code
            buf.put_i16(1); // binary
        } else {
            buf.put_i16(result_formats.len() as i16);
            for &fmt in result_formats {
                buf.put_i16(fmt);
            }
        }
    });
}

/// Describe (D) — describe a statement.
pub fn describe_statement(buf: &mut BytesMut, name: &str) {
    encode_message(buf, b'D', |buf| {
        buf.put_u8(b'S');
        put_cstr(buf, name);
    });
}

/// Describe (D) — describe a portal.
pub fn describe_portal(buf: &mut BytesMut, name: &str) {
    encode_message(buf, b'D', |buf| {
        buf.put_u8(b'P');
        put_cstr(buf, name);
    });
}

/// Execute (E) — execute a bound portal.
pub fn execute(buf: &mut BytesMut, portal: &str, max_rows: i32) {
    encode_message(buf, b'E', |buf| {
        put_cstr(buf, portal);
        buf.put_i32(max_rows); // 0 = no limit
    });
}

/// Sync (S) — end of an extended query pipeline.
pub fn sync(buf: &mut BytesMut) {
    encode_message(buf, b'S', |_| {});
}

/// Flush (H) — request server to flush output.
pub fn flush(buf: &mut BytesMut) {
    encode_message(buf, b'H', |_| {});
}

/// Close (C) — close a statement.
pub fn close_statement(buf: &mut BytesMut, name: &str) {
    encode_message(buf, b'C', |buf| {
        buf.put_u8(b'S');
        put_cstr(buf, name);
    });
}

/// Close (C) — close a portal.
pub fn close_portal(buf: &mut BytesMut, name: &str) {
    encode_message(buf, b'C', |buf| {
        buf.put_u8(b'P');
        put_cstr(buf, name);
    });
}

/// Terminate (X) — disconnect.
pub fn terminate(buf: &mut BytesMut) {
    encode_message(buf, b'X', |_| {});
}

/// CopyData (d) — a chunk of COPY data.
pub fn copy_data(buf: &mut BytesMut, data: &[u8]) {
    encode_message(buf, b'd', |buf| {
        buf.put_slice(data);
    });
}

/// CopyDone (c) — end of COPY IN data.
pub fn copy_done(buf: &mut BytesMut) {
    encode_message(buf, b'c', |_| {});
}

/// CopyFail (f) — abort COPY IN with error message.
pub fn copy_fail(buf: &mut BytesMut, message: &str) {
    encode_message(buf, b'f', |buf| {
        put_cstr(buf, message);
    });
}

/// PasswordMessage (p) — send password (cleartext or MD5).
pub fn password(buf: &mut BytesMut, password: &str) {
    encode_message(buf, b'p', |buf| {
        put_cstr(buf, password);
    });
}

/// SASLInitialResponse (p) — first SCRAM message.
pub fn sasl_initial_response(buf: &mut BytesMut, mechanism: &str, data: &[u8]) {
    encode_message(buf, b'p', |buf| {
        put_cstr(buf, mechanism);
        buf.put_i32(data.len() as i32);
        buf.put_slice(data);
    });
}

/// SASLResponse (p) — subsequent SCRAM message.
pub fn sasl_response(buf: &mut BytesMut, data: &[u8]) {
    encode_message(buf, b'p', |buf| {
        buf.put_slice(data);
    });
}

// ── Helpers ──────────────────────────────────────────

/// Encode a PG wire protocol message: `[type: u8][length: i32][payload]`.
fn encode_message(buf: &mut BytesMut, msg_type: u8, encode_body: impl FnOnce(&mut BytesMut)) {
    buf.put_u8(msg_type);
    let len_idx = buf.len();
    buf.put_i32(0); // placeholder
    encode_body(buf);
    let len = (buf.len() - len_idx) as i32;
    buf[len_idx..len_idx + 4].copy_from_slice(&len.to_be_bytes());
}

/// Write a C-string (null-terminated).
fn put_cstr(buf: &mut BytesMut, s: &str) {
    buf.put_slice(s.as_bytes());
    buf.put_u8(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_startup_message() {
        let mut buf = BytesMut::new();
        startup(
            &mut buf,
            "testuser",
            "testdb",
            &[("application_name", "sentinel")],
        );

        let len = i32::from_be_bytes(buf[0..4].try_into().unwrap());
        assert_eq!(len as usize, buf.len());

        let version = i32::from_be_bytes(buf[4..8].try_into().unwrap());
        assert_eq!(version, 196608);

        assert_eq!(buf[buf.len() - 1], 0);
    }

    #[test]
    fn test_query_message() {
        let mut buf = BytesMut::new();
        query(&mut buf, "SELECT 1");

        assert_eq!(buf[0], b'Q');
        let len = i32::from_be_bytes(buf[1..5].try_into().unwrap());
        assert_eq!(len, 13); // 4 (self) + 8 (SQL) + 1 (null)
    }

    #[test]
    fn test_sync_message() {
        let mut buf = BytesMut::new();
        sync(&mut buf);

        assert_eq!(buf[0], b'S');
        let len = i32::from_be_bytes(buf[1..5].try_into().unwrap());
        assert_eq!(len, 4);
    }

    #[test]
    fn test_terminate_message() {
        let mut buf = BytesMut::new();
        terminate(&mut buf);

        assert_eq!(buf[0], b'X');
        assert_eq!(buf.len(), 5);
    }

    #[test]
    fn test_parse_message() {
        let mut buf = BytesMut::new();
        parse(&mut buf, "stmt1", "SELECT $1::int4", &[23]);

        assert_eq!(buf[0], b'P');
        let body = &buf[5..];
        assert!(body.windows(5).any(|w| w == b"stmt1"));
    }

    #[test]
    fn test_ssl_request() {
        let mut buf = BytesMut::new();
        ssl_request(&mut buf);

        assert_eq!(buf.len(), 8);
        let len = i32::from_be_bytes(buf[0..4].try_into().unwrap());
        assert_eq!(len, 8);
        let magic = i32::from_be_bytes(buf[4..8].try_into().unwrap());
        assert_eq!(magic, 80877103);
    }

    #[test]
    fn test_bind_with_null_param() {
        let mut buf = BytesMut::new();
        let params: Vec<Option<&[u8]>> = vec![Some(&[0, 0, 0, 1]), None];
        bind(&mut buf, "", "stmt1", &params, &[]);

        assert_eq!(buf[0], b'B');
    }
}
