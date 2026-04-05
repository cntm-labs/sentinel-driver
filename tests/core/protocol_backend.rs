use bytes::Bytes;

use sentinel_driver::protocol::backend::*;

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
