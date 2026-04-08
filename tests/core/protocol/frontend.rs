use bytes::BytesMut;

use sentinel_driver::protocol::frontend::*;

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

#[test]
fn test_cancel_request() {
    use bytes::BytesMut;
    use sentinel_driver::protocol::frontend;

    let mut buf = BytesMut::new();
    frontend::cancel_request(&mut buf, 12345, 67890);

    assert_eq!(buf.len(), 16);

    // length = 16
    let len = i32::from_be_bytes(buf[0..4].try_into().unwrap());
    assert_eq!(len, 16);

    // magic = 80877102
    let magic = i32::from_be_bytes(buf[4..8].try_into().unwrap());
    assert_eq!(magic, 80877102);

    // process_id
    let pid = i32::from_be_bytes(buf[8..12].try_into().unwrap());
    assert_eq!(pid, 12345);

    // secret_key
    let key = i32::from_be_bytes(buf[12..16].try_into().unwrap());
    assert_eq!(key, 67890);
}
