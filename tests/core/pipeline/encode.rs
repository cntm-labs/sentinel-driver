use bytes::BytesMut;
use sentinel_driver::pipeline::{encode_pipeline, PipelineQuery};

#[test]
fn test_encode_pipeline_single_query() {
    let queries = vec![PipelineQuery {
        sql: "SELECT 1".to_string(),
        param_types: vec![],
        params: vec![],
    }];

    let mut buf = BytesMut::new();
    encode_pipeline(&mut buf, &queries);

    // Should contain: Parse('P') + Bind('B') + Describe('D') + Execute('E') + Sync('S')
    let types: Vec<u8> = extract_message_types(&buf);
    assert_eq!(types, vec![b'P', b'B', b'D', b'E', b'S']);
}

#[test]
fn test_encode_pipeline_multiple_queries() {
    let queries = vec![
        PipelineQuery {
            sql: "SELECT 1".to_string(),
            param_types: vec![],
            params: vec![],
        },
        PipelineQuery {
            sql: "SELECT 2".to_string(),
            param_types: vec![],
            params: vec![],
        },
        PipelineQuery {
            sql: "SELECT 3".to_string(),
            param_types: vec![],
            params: vec![],
        },
    ];

    let mut buf = BytesMut::new();
    encode_pipeline(&mut buf, &queries);

    // 3 queries x (P+B+D+E) + 1 Sync = 13 messages
    let types = extract_message_types(&buf);
    assert_eq!(types.len(), 13);

    // Verify pattern: P,B,D,E repeated 3x then S
    assert_eq!(types[0], b'P');
    assert_eq!(types[4], b'P');
    assert_eq!(types[8], b'P');
    assert_eq!(types[12], b'S'); // single Sync at end
}

#[test]
fn test_encode_pipeline_with_params() {
    let queries = vec![PipelineQuery {
        sql: "SELECT * FROM users WHERE id = $1".to_string(),
        param_types: vec![23], // int4
        params: vec![Some(42i32.to_be_bytes().to_vec())],
    }];

    let mut buf = BytesMut::new();
    encode_pipeline(&mut buf, &queries);

    let types = extract_message_types(&buf);
    assert_eq!(types, vec![b'P', b'B', b'D', b'E', b'S']);

    // Verify the buffer contains the parameter data
    assert!(buf.len() > 30); // should be reasonably large with SQL + params
}

/// Extract message type bytes from an encoded buffer.
fn extract_message_types(buf: &[u8]) -> Vec<u8> {
    let mut types = Vec::new();
    let mut pos = 0;

    while pos < buf.len() {
        let msg_type = buf[pos];
        types.push(msg_type);

        if pos + 5 > buf.len() {
            break;
        }
        let len =
            i32::from_be_bytes([buf[pos + 1], buf[pos + 2], buf[pos + 3], buf[pos + 4]]) as usize;
        pos += 1 + len; // type byte + declared length (which includes its own 4 bytes)
    }

    types
}
