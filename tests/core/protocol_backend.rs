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

#[test]
fn test_decode_backend_key_data() {
    let mut data = Vec::new();
    data.extend_from_slice(&1234i32.to_be_bytes());
    data.extend_from_slice(&5678i32.to_be_bytes());

    let body = Bytes::from(data);
    let msg = decode(b'K', body).unwrap();
    match msg {
        BackendMessage::BackendKeyData {
            process_id,
            secret_key,
        } => {
            assert_eq!(process_id, 1234);
            assert_eq!(secret_key, 5678);
        }
        _ => panic!("expected BackendKeyData"),
    }
}

#[test]
fn test_decode_parameter_status() {
    let mut data = Vec::new();
    data.extend_from_slice(b"server_version\0");
    data.extend_from_slice(b"16.2\0");

    let body = Bytes::from(data);
    let msg = decode(b'S', body).unwrap();
    match msg {
        BackendMessage::ParameterStatus { name, value } => {
            assert_eq!(name, "server_version");
            assert_eq!(value, "16.2");
        }
        _ => panic!("expected ParameterStatus"),
    }
}

#[test]
fn test_decode_row_description() {
    let mut data = Vec::new();
    data.extend_from_slice(&1i16.to_be_bytes()); // 1 field
    data.extend_from_slice(b"id\0"); // field name
    data.extend_from_slice(&0u32.to_be_bytes()); // table_oid
    data.extend_from_slice(&1i16.to_be_bytes()); // column_id
    data.extend_from_slice(&23u32.to_be_bytes()); // type_oid (int4)
    data.extend_from_slice(&4i16.to_be_bytes()); // type_size
    data.extend_from_slice(&(-1i32).to_be_bytes()); // type_modifier
    data.extend_from_slice(&1i16.to_be_bytes()); // format (binary)

    let body = Bytes::from(data);
    let msg = decode(b'T', body).unwrap();
    match msg {
        BackendMessage::RowDescription { fields } => {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].name, "id");
            assert_eq!(fields[0].type_oid, 23);
            assert_eq!(fields[0].format, 1);
        }
        _ => panic!("expected RowDescription"),
    }
}

#[test]
fn test_decode_parameter_description() {
    let mut data = Vec::new();
    data.extend_from_slice(&2i16.to_be_bytes()); // 2 params
    data.extend_from_slice(&23u32.to_be_bytes()); // int4
    data.extend_from_slice(&25u32.to_be_bytes()); // text

    let body = Bytes::from(data);
    let msg = decode(b't', body).unwrap();
    match msg {
        BackendMessage::ParameterDescription { oids } => {
            assert_eq!(oids, vec![23, 25]);
        }
        _ => panic!("expected ParameterDescription"),
    }
}

#[test]
fn test_decode_copy_in_response() {
    let mut data = Vec::new();
    data.push(0); // text format
    data.extend_from_slice(&2i16.to_be_bytes()); // 2 columns
    data.extend_from_slice(&0i16.to_be_bytes()); // col 0: text
    data.extend_from_slice(&0i16.to_be_bytes()); // col 1: text

    let body = Bytes::from(data);
    let msg = decode(b'G', body).unwrap();
    match msg {
        BackendMessage::CopyInResponse {
            format,
            column_formats,
        } => {
            assert_eq!(format, CopyFormat::Text);
            assert_eq!(column_formats, vec![0, 0]);
        }
        _ => panic!("expected CopyInResponse"),
    }
}

#[test]
fn test_decode_copy_out_response() {
    let mut data = Vec::new();
    data.push(1); // binary format
    data.extend_from_slice(&1i16.to_be_bytes()); // 1 column
    data.extend_from_slice(&1i16.to_be_bytes()); // col 0: binary

    let body = Bytes::from(data);
    let msg = decode(b'H', body).unwrap();
    match msg {
        BackendMessage::CopyOutResponse {
            format,
            column_formats,
        } => {
            assert_eq!(format, CopyFormat::Binary);
            assert_eq!(column_formats, vec![1]);
        }
        _ => panic!("expected CopyOutResponse"),
    }
}

#[test]
fn test_decode_notice_response() {
    let mut data = Vec::new();
    data.push(b'S');
    data.extend_from_slice(b"NOTICE\0");
    data.push(b'M');
    data.extend_from_slice(b"some notice\0");
    data.push(0); // terminator

    let body = Bytes::from(data);
    let msg = decode(b'N', body).unwrap();
    match msg {
        BackendMessage::NoticeResponse { fields } => {
            assert_eq!(fields.severity, "NOTICE");
            assert_eq!(fields.message, "some notice");
        }
        _ => panic!("expected NoticeResponse"),
    }
}

#[test]
fn test_decode_auth_sasl() {
    let mut data = Vec::new();
    data.extend_from_slice(&10i32.to_be_bytes()); // SASL
    data.extend_from_slice(b"SCRAM-SHA-256\0");
    data.push(0); // end of mechanisms

    let body = Bytes::from(data);
    let msg = decode(b'R', body).unwrap();
    match msg {
        BackendMessage::AuthenticationSasl { mechanisms } => {
            assert_eq!(mechanisms, vec!["SCRAM-SHA-256"]);
        }
        _ => panic!("expected AuthenticationSasl"),
    }
}

#[test]
fn test_decode_auth_sasl_continue() {
    let mut data = Vec::new();
    data.extend_from_slice(&11i32.to_be_bytes()); // SASLContinue
    data.extend_from_slice(b"server-data");

    let body = Bytes::from(data);
    let msg = decode(b'R', body).unwrap();
    match msg {
        BackendMessage::AuthenticationSaslContinue { data: sasl_data } => {
            assert_eq!(sasl_data, b"server-data");
        }
        _ => panic!("expected AuthenticationSaslContinue"),
    }
}

#[test]
fn test_decode_auth_sasl_final() {
    let mut data = Vec::new();
    data.extend_from_slice(&12i32.to_be_bytes()); // SASLFinal
    data.extend_from_slice(b"final-msg");

    let body = Bytes::from(data);
    let msg = decode(b'R', body).unwrap();
    match msg {
        BackendMessage::AuthenticationSaslFinal { data: sasl_data } => {
            assert_eq!(sasl_data, b"final-msg");
        }
        _ => panic!("expected AuthenticationSaslFinal"),
    }
}

#[test]
fn test_decode_simple_messages() {
    // ParseComplete
    let msg = decode(b'1', Bytes::new()).unwrap();
    assert!(matches!(msg, BackendMessage::ParseComplete));

    // BindComplete
    let msg = decode(b'2', Bytes::new()).unwrap();
    assert!(matches!(msg, BackendMessage::BindComplete));

    // CloseComplete
    let msg = decode(b'3', Bytes::new()).unwrap();
    assert!(matches!(msg, BackendMessage::CloseComplete));

    // NoData
    let msg = decode(b'n', Bytes::new()).unwrap();
    assert!(matches!(msg, BackendMessage::NoData));

    // EmptyQueryResponse
    let msg = decode(b'I', Bytes::new()).unwrap();
    assert!(matches!(msg, BackendMessage::EmptyQueryResponse));

    // CopyDone
    let msg = decode(b'c', Bytes::new()).unwrap();
    assert!(matches!(msg, BackendMessage::CopyDone));

    // CopyData
    let msg = decode(b'd', Bytes::from_static(b"raw data")).unwrap();
    match msg {
        BackendMessage::CopyData { data } => assert_eq!(&data[..], b"raw data"),
        _ => panic!("expected CopyData"),
    }
}

#[test]
fn test_decode_unknown_message_type() {
    assert!(decode(b'?', Bytes::new()).is_err());
}

#[test]
fn test_decode_auth_cleartext() {
    let body = Bytes::from_static(&[0, 0, 0, 3]); // cleartext
    let msg = decode(b'R', body).unwrap();
    assert!(matches!(
        msg,
        BackendMessage::AuthenticationCleartextPassword
    ));
}
