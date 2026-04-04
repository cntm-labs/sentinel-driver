use bytes::Bytes;

use crate::error::{Error, Result};

/// A decoded backend (server → client) message.
#[derive(Debug)]
pub enum BackendMessage {
    AuthenticationOk,
    AuthenticationCleartextPassword,
    AuthenticationMd5Password {
        salt: [u8; 4],
    },
    AuthenticationSasl {
        mechanisms: Vec<String>,
    },
    AuthenticationSaslContinue {
        data: Vec<u8>,
    },
    AuthenticationSaslFinal {
        data: Vec<u8>,
    },

    BackendKeyData {
        process_id: i32,
        secret_key: i32,
    },

    ParameterStatus {
        name: String,
        value: String,
    },

    ReadyForQuery {
        transaction_status: TransactionStatus,
    },

    RowDescription {
        fields: Vec<FieldDescription>,
    },

    DataRow {
        columns: DataRowColumns,
    },

    CommandComplete {
        tag: String,
    },

    EmptyQueryResponse,

    ErrorResponse {
        fields: ErrorFields,
    },

    NoticeResponse {
        fields: ErrorFields,
    },

    ParseComplete,
    BindComplete,
    CloseComplete,
    NoData,
    PortalSuspended,

    ParameterDescription {
        oids: Vec<u32>,
    },

    CopyInResponse {
        format: CopyFormat,
        column_formats: Vec<i16>,
    },
    CopyOutResponse {
        format: CopyFormat,
        column_formats: Vec<i16>,
    },
    CopyData {
        data: Bytes,
    },
    CopyDone,

    NotificationResponse {
        process_id: i32,
        channel: String,
        payload: String,
    },
}

/// Transaction status indicator from ReadyForQuery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionStatus {
    /// Not in a transaction block.
    Idle,
    /// In a transaction block.
    InTransaction,
    /// In a failed transaction block.
    Failed,
}

/// COPY format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyFormat {
    Text,
    Binary,
}

/// Description of a single column in RowDescription.
#[derive(Debug, Clone)]
pub struct FieldDescription {
    pub name: String,
    pub table_oid: u32,
    pub column_id: i16,
    pub type_oid: u32,
    pub type_size: i16,
    pub type_modifier: i32,
    pub format: i16,
}

/// Error/Notice response fields.
#[derive(Debug, Clone)]
pub struct ErrorFields {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub detail: Option<String>,
    pub hint: Option<String>,
    pub position: Option<u32>,
    pub internal_position: Option<u32>,
    pub internal_query: Option<String>,
    pub where_: Option<String>,
    pub schema: Option<String>,
    pub table: Option<String>,
    pub column: Option<String>,
    pub data_type: Option<String>,
    pub constraint: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub routine: Option<String>,
}

/// Zero-copy column data from a DataRow message.
///
/// Stores the raw buffer and column offsets for deferred decoding.
#[derive(Debug)]
pub struct DataRowColumns {
    buf: Bytes,
    /// Each entry is (offset, length). length == -1 means NULL.
    columns: Vec<(usize, i32)>,
}

impl DataRowColumns {
    /// Number of columns.
    pub fn len(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    /// Get raw bytes for column at `idx`. Returns `None` for NULL.
    pub fn get(&self, idx: usize) -> Option<Bytes> {
        let &(offset, len) = self.columns.get(idx)?;
        if len < 0 {
            None // NULL
        } else {
            Some(self.buf.slice(offset..offset + len as usize))
        }
    }

    /// Returns `true` if column `idx` is NULL.
    pub fn is_null(&self, idx: usize) -> bool {
        self.columns.get(idx).map_or(true, |&(_, len)| len < 0)
    }
}

/// Decode a single backend message from a raw frame.
///
/// `msg_type` is the first byte, `body` is the payload (after the length).
pub fn decode(msg_type: u8, body: Bytes) -> Result<BackendMessage> {
    match msg_type {
        b'R' => decode_auth(body),
        b'K' => decode_backend_key_data(body),
        b'S' => decode_parameter_status(body),
        b'Z' => decode_ready_for_query(body),
        b'T' => decode_row_description(body),
        b'D' => decode_data_row(body),
        b'C' => decode_command_complete(body),
        b'I' => Ok(BackendMessage::EmptyQueryResponse),
        b'E' => decode_error_response(body),
        b'N' => decode_notice_response(body),
        b'1' => Ok(BackendMessage::ParseComplete),
        b'2' => Ok(BackendMessage::BindComplete),
        b'3' => Ok(BackendMessage::CloseComplete),
        b'n' => Ok(BackendMessage::NoData),
        b's' => Ok(BackendMessage::PortalSuspended),
        b't' => decode_parameter_description(body),
        b'G' => decode_copy_in_response(body),
        b'H' => decode_copy_out_response(body),
        b'd' => Ok(BackendMessage::CopyData { data: body }),
        b'c' => Ok(BackendMessage::CopyDone),
        b'A' => decode_notification(body),
        _ => Err(Error::protocol(format!(
            "unknown message type: 0x{msg_type:02x}"
        ))),
    }
}

// ── Decoders ─────────────────────────────────────────

fn decode_auth(body: Bytes) -> Result<BackendMessage> {
    if body.len() < 4 {
        return Err(Error::protocol("auth message too short"));
    }
    let auth_type = read_i32(&body, 0);

    match auth_type {
        0 => Ok(BackendMessage::AuthenticationOk),
        3 => Ok(BackendMessage::AuthenticationCleartextPassword),
        5 => {
            if body.len() < 8 {
                return Err(Error::protocol("MD5 auth message too short"));
            }
            let mut salt = [0u8; 4];
            salt.copy_from_slice(&body[4..8]);
            Ok(BackendMessage::AuthenticationMd5Password { salt })
        }
        10 => {
            // SASL — parse null-separated mechanism list
            let mut mechanisms = Vec::new();
            let mut pos = 4;
            loop {
                if pos >= body.len() {
                    break;
                }
                let s = read_cstr(&body, &mut pos)?;
                if s.is_empty() {
                    break;
                }
                mechanisms.push(s);
            }
            Ok(BackendMessage::AuthenticationSasl { mechanisms })
        }
        11 => Ok(BackendMessage::AuthenticationSaslContinue {
            data: body[4..].to_vec(),
        }),
        12 => Ok(BackendMessage::AuthenticationSaslFinal {
            data: body[4..].to_vec(),
        }),
        _ => Err(Error::protocol(format!(
            "unsupported auth type: {auth_type}"
        ))),
    }
}

fn decode_backend_key_data(body: Bytes) -> Result<BackendMessage> {
    if body.len() < 8 {
        return Err(Error::protocol("BackendKeyData too short"));
    }
    Ok(BackendMessage::BackendKeyData {
        process_id: read_i32(&body, 0),
        secret_key: read_i32(&body, 4),
    })
}

fn decode_parameter_status(body: Bytes) -> Result<BackendMessage> {
    let mut pos = 0;
    let name = read_cstr(&body, &mut pos)?;
    let value = read_cstr(&body, &mut pos)?;
    Ok(BackendMessage::ParameterStatus { name, value })
}

fn decode_ready_for_query(body: Bytes) -> Result<BackendMessage> {
    if body.is_empty() {
        return Err(Error::protocol("ReadyForQuery empty"));
    }
    let status = match body[0] {
        b'I' => TransactionStatus::Idle,
        b'T' => TransactionStatus::InTransaction,
        b'E' => TransactionStatus::Failed,
        s => return Err(Error::protocol(format!("unknown transaction status: {s}"))),
    };
    Ok(BackendMessage::ReadyForQuery {
        transaction_status: status,
    })
}

fn decode_row_description(body: Bytes) -> Result<BackendMessage> {
    if body.len() < 2 {
        return Err(Error::protocol("RowDescription too short"));
    }
    let field_count = read_i16(&body, 0) as usize;
    let mut fields = Vec::with_capacity(field_count);
    let mut pos = 2;

    for _ in 0..field_count {
        let name = read_cstr(&body, &mut pos)?;

        if pos + 18 > body.len() {
            return Err(Error::protocol("RowDescription field truncated"));
        }

        let table_oid = read_u32(&body, pos);
        let column_id = read_i16(&body, pos + 4);
        let type_oid = read_u32(&body, pos + 6);
        let type_size = read_i16(&body, pos + 10);
        let type_modifier = read_i32(&body, pos + 12);
        let format = read_i16(&body, pos + 16);
        pos += 18;

        fields.push(FieldDescription {
            name,
            table_oid,
            column_id,
            type_oid,
            type_size,
            type_modifier,
            format,
        });
    }

    Ok(BackendMessage::RowDescription { fields })
}

fn decode_data_row(body: Bytes) -> Result<BackendMessage> {
    if body.len() < 2 {
        return Err(Error::protocol("DataRow too short"));
    }
    let col_count = read_i16(&body, 0) as usize;
    let mut columns = Vec::with_capacity(col_count);
    let mut pos = 2;

    for _ in 0..col_count {
        if pos + 4 > body.len() {
            return Err(Error::protocol("DataRow column truncated"));
        }
        let len = read_i32(&body, pos);
        pos += 4;

        if len < 0 {
            columns.push((0, -1)); // NULL
        } else {
            let len_usize = len as usize;
            if pos + len_usize > body.len() {
                return Err(Error::protocol("DataRow column data truncated"));
            }
            columns.push((pos, len));
            pos += len_usize;
        }
    }

    Ok(BackendMessage::DataRow {
        columns: DataRowColumns { buf: body, columns },
    })
}

fn decode_command_complete(body: Bytes) -> Result<BackendMessage> {
    let mut pos = 0;
    let tag = read_cstr(&body, &mut pos)?;
    Ok(BackendMessage::CommandComplete { tag })
}

fn decode_error_notice_fields(body: &Bytes) -> Result<ErrorFields> {
    let mut severity = String::new();
    let mut code = String::new();
    let mut message = String::new();
    let mut detail = None;
    let mut hint = None;
    let mut position = None;
    let mut internal_position = None;
    let mut internal_query = None;
    let mut where_ = None;
    let mut schema = None;
    let mut table = None;
    let mut column = None;
    let mut data_type = None;
    let mut constraint = None;
    let mut file = None;
    let mut line = None;
    let mut routine = None;

    let mut pos = 0;
    loop {
        if pos >= body.len() {
            break;
        }
        let field_type = body[pos];
        pos += 1;
        if field_type == 0 {
            break;
        }
        let value = read_cstr(body, &mut pos)?;

        match field_type {
            b'S' => severity = value,
            b'C' => code = value,
            b'M' => message = value,
            b'D' => detail = Some(value),
            b'H' => hint = Some(value),
            b'P' => position = value.parse().ok(),
            b'p' => internal_position = value.parse().ok(),
            b'q' => internal_query = Some(value),
            b'W' => where_ = Some(value),
            b's' => schema = Some(value),
            b't' => table = Some(value),
            b'c' => column = Some(value),
            b'd' => data_type = Some(value),
            b'n' => constraint = Some(value),
            b'F' => file = Some(value),
            b'L' => line = value.parse().ok(),
            b'R' => routine = Some(value),
            _ => {} // ignore unknown fields
        }
    }

    Ok(ErrorFields {
        severity,
        code,
        message,
        detail,
        hint,
        position,
        internal_position,
        internal_query,
        where_,
        schema,
        table,
        column,
        data_type,
        constraint,
        file,
        line,
        routine,
    })
}

fn decode_error_response(body: Bytes) -> Result<BackendMessage> {
    let fields = decode_error_notice_fields(&body)?;
    Ok(BackendMessage::ErrorResponse { fields })
}

fn decode_notice_response(body: Bytes) -> Result<BackendMessage> {
    let fields = decode_error_notice_fields(&body)?;
    Ok(BackendMessage::NoticeResponse { fields })
}

fn decode_parameter_description(body: Bytes) -> Result<BackendMessage> {
    if body.len() < 2 {
        return Err(Error::protocol("ParameterDescription too short"));
    }
    let count = read_i16(&body, 0) as usize;
    let mut oids = Vec::with_capacity(count);
    let mut pos = 2;

    for _ in 0..count {
        if pos + 4 > body.len() {
            return Err(Error::protocol("ParameterDescription truncated"));
        }
        oids.push(read_u32(&body, pos));
        pos += 4;
    }

    Ok(BackendMessage::ParameterDescription { oids })
}

fn decode_copy_response(body: &Bytes) -> Result<(CopyFormat, Vec<i16>)> {
    if body.len() < 3 {
        return Err(Error::protocol("CopyResponse too short"));
    }
    let format = match body[0] {
        0 => CopyFormat::Text,
        1 => CopyFormat::Binary,
        f => return Err(Error::protocol(format!("unknown copy format: {f}"))),
    };
    let col_count = read_i16(body, 1) as usize;
    let mut column_formats = Vec::with_capacity(col_count);
    let mut pos = 3;

    for _ in 0..col_count {
        if pos + 2 > body.len() {
            return Err(Error::protocol("CopyResponse column formats truncated"));
        }
        column_formats.push(read_i16(body, pos));
        pos += 2;
    }

    Ok((format, column_formats))
}

fn decode_copy_in_response(body: Bytes) -> Result<BackendMessage> {
    let (format, column_formats) = decode_copy_response(&body)?;
    Ok(BackendMessage::CopyInResponse {
        format,
        column_formats,
    })
}

fn decode_copy_out_response(body: Bytes) -> Result<BackendMessage> {
    let (format, column_formats) = decode_copy_response(&body)?;
    Ok(BackendMessage::CopyOutResponse {
        format,
        column_formats,
    })
}

fn decode_notification(body: Bytes) -> Result<BackendMessage> {
    if body.len() < 4 {
        return Err(Error::protocol("NotificationResponse too short"));
    }
    let process_id = read_i32(&body, 0);
    let mut pos = 4;
    let channel = read_cstr(&body, &mut pos)?;
    let payload = read_cstr(&body, &mut pos)?;

    Ok(BackendMessage::NotificationResponse {
        process_id,
        channel,
        payload,
    })
}

// ── Read helpers ─────────────────────────────────────

fn read_i32(buf: &[u8], offset: usize) -> i32 {
    i32::from_be_bytes(buf[offset..offset + 4].try_into().unwrap())
}

fn read_u32(buf: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(buf[offset..offset + 4].try_into().unwrap())
}

fn read_i16(buf: &[u8], offset: usize) -> i16 {
    i16::from_be_bytes(buf[offset..offset + 2].try_into().unwrap())
}

/// Read a null-terminated string starting at `pos`, advancing `pos` past the null.
fn read_cstr(buf: &[u8], pos: &mut usize) -> Result<String> {
    let start = *pos;
    let null_pos = buf[start..]
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| Error::protocol("missing null terminator"))?;

    let s = std::str::from_utf8(&buf[start..start + null_pos])
        .map_err(|e| Error::protocol(format!("invalid UTF-8 in message: {e}")))?
        .to_string();

    *pos = start + null_pos + 1;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_auth_ok() {
        let body = Bytes::from_static(&[0, 0, 0, 0]);
        let msg = decode(b'R', body).unwrap();
        assert!(matches!(msg, BackendMessage::AuthenticationOk));
    }

    #[test]
    fn test_decode_auth_md5() {
        let body = Bytes::from_static(&[0, 0, 0, 5, 0xAA, 0xBB, 0xCC, 0xDD]);
        let msg = decode(b'R', body).unwrap();
        match msg {
            BackendMessage::AuthenticationMd5Password { salt } => {
                assert_eq!(salt, [0xAA, 0xBB, 0xCC, 0xDD]);
            }
            _ => panic!("expected MD5 auth"),
        }
    }

    #[test]
    fn test_decode_ready_for_query() {
        let body = Bytes::from_static(&[b'I']);
        let msg = decode(b'Z', body).unwrap();
        match msg {
            BackendMessage::ReadyForQuery { transaction_status } => {
                assert_eq!(transaction_status, TransactionStatus::Idle);
            }
            _ => panic!("expected ReadyForQuery"),
        }
    }

    #[test]
    fn test_decode_data_row() {
        // 2 columns: first = 3 bytes "abc", second = NULL
        let mut data = Vec::new();
        data.extend_from_slice(&2i16.to_be_bytes()); // column count
        data.extend_from_slice(&3i32.to_be_bytes()); // col 0 length
        data.extend_from_slice(b"abc"); // col 0 data
        data.extend_from_slice(&(-1i32).to_be_bytes()); // col 1 NULL

        let body = Bytes::from(data);
        let msg = decode(b'D', body).unwrap();
        match msg {
            BackendMessage::DataRow { columns } => {
                assert_eq!(columns.len(), 2);
                assert_eq!(columns.get(0).unwrap().as_ref(), b"abc");
                assert!(columns.is_null(1));
            }
            _ => panic!("expected DataRow"),
        }
    }

    #[test]
    fn test_decode_command_complete() {
        let mut data = Vec::new();
        data.extend_from_slice(b"INSERT 0 5\0");
        let body = Bytes::from(data);
        let msg = decode(b'C', body).unwrap();
        match msg {
            BackendMessage::CommandComplete { tag } => {
                assert_eq!(tag, "INSERT 0 5");
            }
            _ => panic!("expected CommandComplete"),
        }
    }

    #[test]
    fn test_decode_error_response() {
        let mut data = Vec::new();
        data.push(b'S');
        data.extend_from_slice(b"ERROR\0");
        data.push(b'C');
        data.extend_from_slice(b"42P01\0");
        data.push(b'M');
        data.extend_from_slice(b"relation \"foo\" does not exist\0");
        data.push(0); // terminator

        let body = Bytes::from(data);
        let msg = decode(b'E', body).unwrap();
        match msg {
            BackendMessage::ErrorResponse { fields } => {
                assert_eq!(fields.severity, "ERROR");
                assert_eq!(fields.code, "42P01");
                assert!(fields.message.contains("foo"));
            }
            _ => panic!("expected ErrorResponse"),
        }
    }

    #[test]
    fn test_decode_notification() {
        let mut data = Vec::new();
        data.extend_from_slice(&12345i32.to_be_bytes());
        data.extend_from_slice(b"my_channel\0");
        data.extend_from_slice(b"hello world\0");

        let body = Bytes::from(data);
        let msg = decode(b'A', body).unwrap();
        match msg {
            BackendMessage::NotificationResponse {
                process_id,
                channel,
                payload,
            } => {
                assert_eq!(process_id, 12345);
                assert_eq!(channel, "my_channel");
                assert_eq!(payload, "hello world");
            }
            _ => panic!("expected NotificationResponse"),
        }
    }

    #[test]
    fn test_data_row_zero_copy() {
        // Verify that get() returns a Bytes slice (zero-copy) into the original buffer
        let mut data = Vec::new();
        data.extend_from_slice(&1i16.to_be_bytes());
        data.extend_from_slice(&5i32.to_be_bytes());
        data.extend_from_slice(b"hello");

        let body = Bytes::from(data);
        let msg = decode(b'D', body).unwrap();
        match msg {
            BackendMessage::DataRow { columns } => {
                let val = columns.get(0).unwrap();
                assert_eq!(&val[..], b"hello");
            }
            _ => panic!("expected DataRow"),
        }
    }
}
