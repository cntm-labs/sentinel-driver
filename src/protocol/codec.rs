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
    let body_len = i32::from_be_bytes(buf[1..5].try_into().unwrap());

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

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BufMut;

    #[test]
    fn test_decode_needs_more_data() {
        let mut buf = BytesMut::from(&[b'Z'][..]);
        assert!(decode_message(&mut buf).unwrap().is_none());
    }

    #[test]
    fn test_decode_complete_message() {
        // ReadyForQuery: type='Z', length=5, status='I'
        let mut buf = BytesMut::new();
        buf.put_u8(b'Z');
        buf.put_i32(5); // length includes self
        buf.put_u8(b'I');

        let msg = decode_message(&mut buf).unwrap().unwrap();
        assert!(matches!(
            msg,
            BackendMessage::ReadyForQuery { .. }
        ));
        assert!(buf.is_empty()); // fully consumed
    }

    #[test]
    fn test_decode_partial_body() {
        let mut buf = BytesMut::new();
        buf.put_u8(b'Z');
        buf.put_i32(5);
        // missing the status byte

        assert!(decode_message(&mut buf).unwrap().is_none());
        assert_eq!(buf.len(), 5); // not consumed
    }

    #[test]
    fn test_decode_multiple_messages() {
        let mut buf = BytesMut::new();

        // Message 1: ParseComplete (type='1', length=4)
        buf.put_u8(b'1');
        buf.put_i32(4);

        // Message 2: BindComplete (type='2', length=4)
        buf.put_u8(b'2');
        buf.put_i32(4);

        let msg1 = decode_message(&mut buf).unwrap().unwrap();
        assert!(matches!(msg1, BackendMessage::ParseComplete));

        let msg2 = decode_message(&mut buf).unwrap().unwrap();
        assert!(matches!(msg2, BackendMessage::BindComplete));

        assert!(buf.is_empty());
    }

    #[test]
    fn test_invalid_length() {
        let mut buf = BytesMut::new();
        buf.put_u8(b'Z');
        buf.put_i32(2); // invalid: less than 4

        assert!(decode_message(&mut buf).is_err());
    }

    #[test]
    fn test_ssl_response() {
        let mut buf = BytesMut::from(&[b'S'][..]);
        assert_eq!(decode_ssl_response(&mut buf), Some(SslResponse::Accepted));

        let mut buf = BytesMut::from(&[b'N'][..]);
        assert_eq!(decode_ssl_response(&mut buf), Some(SslResponse::Rejected));

        let mut buf = BytesMut::new();
        assert_eq!(decode_ssl_response(&mut buf), None);
    }
}
