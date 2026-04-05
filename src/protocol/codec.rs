use bytes::BytesMut;

use crate::error::{Error, Result};
use crate::protocol::backend::BackendMessage;

/// Minimum header size: type (1) + length (4) = 5 bytes.
const HEADER_LEN: usize = 5;

/// Attempts to decode a single backend message from `buf`.
///
/// Returns `Ok(Some(msg))` if a complete message was decoded (consuming it from `buf`),
/// `Ok(None)` if more data is needed, or `Err` on protocol violation.
pub fn decode_message(buf: &mut BytesMut) -> Result<Option<BackendMessage>> {
    if buf.len() < HEADER_LEN {
        return Ok(None);
    }

    let msg_type = buf[0];
    let body_len = i32::from_be_bytes([buf[1], buf[2], buf[3], buf[4]]);

    if body_len < 4 {
        return Err(Error::protocol(format!(
            "invalid message length: {body_len}"
        )));
    }

    let total_len = 1 + body_len as usize; // type byte + declared length

    if buf.len() < total_len {
        // Need more data. Reserve space to avoid repeated allocations.
        buf.reserve(total_len - buf.len());
        return Ok(None);
    }

    // Consume the entire message frame from the buffer.
    let frame = buf.split_to(total_len).freeze();

    // Body is everything after type(1) + length(4).
    let body = frame.slice(HEADER_LEN..);

    super::backend::decode(msg_type, body).map(Some)
}

/// The initial response to an SSLRequest: 'S' (supports) or 'N' (doesn't).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SslResponse {
    Accepted,
    Rejected,
}

/// Decode the single-byte SSL response.
pub fn decode_ssl_response(buf: &mut BytesMut) -> Option<SslResponse> {
    if buf.is_empty() {
        return None;
    }
    let byte = buf.split_to(1)[0];
    match byte {
        b'S' => Some(SslResponse::Accepted),
        b'N' => Some(SslResponse::Rejected),
        _ => None,
    }
}
