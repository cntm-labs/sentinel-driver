use bytes::{BufMut, BytesMut};

use sentinel_driver::protocol::backend::BackendMessage;
use sentinel_driver::protocol::codec::{decode_message, decode_ssl_response, SslResponse};

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
    assert!(matches!(msg, BackendMessage::ReadyForQuery { .. }));
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
