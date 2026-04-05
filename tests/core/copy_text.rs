use sentinel_driver::copy::text::{TextCopyDecoder, TextCopyEncoder};

#[test]
fn test_encode_simple_row() {
    let mut encoder = TextCopyEncoder::new();
    encoder.add_row(&[Some("42"), Some("hello")]);
    let data = String::from_utf8(encoder.finish()).unwrap();
    assert_eq!(data, "42\thello\n");
}

#[test]
fn test_encode_null() {
    let mut encoder = TextCopyEncoder::new();
    encoder.add_row(&[Some("1"), None, Some("three")]);
    let data = String::from_utf8(encoder.finish()).unwrap();
    assert_eq!(data, "1\t\\N\tthree\n");
}

#[test]
fn test_encode_escape_special_chars() {
    let mut encoder = TextCopyEncoder::new();
    encoder.add_row(&[
        Some("line1\nline2"),
        Some("col1\tcol2"),
        Some("back\\slash"),
    ]);
    let data = String::from_utf8(encoder.finish()).unwrap();
    assert_eq!(data, "line1\\nline2\tcol1\\tcol2\tback\\\\slash\n");
}

#[test]
fn test_decode_simple_line() {
    let fields = TextCopyDecoder::parse_line("42\thello").unwrap();
    assert_eq!(
        fields,
        vec![Some("42".to_string()), Some("hello".to_string())]
    );
}

#[test]
fn test_decode_null() {
    let fields = TextCopyDecoder::parse_line("1\t\\N\tthree").unwrap();
    assert_eq!(
        fields,
        vec![Some("1".to_string()), None, Some("three".to_string()),]
    );
}

#[test]
fn test_decode_escaped_chars() {
    let fields =
        TextCopyDecoder::parse_line("line1\\nline2\tcol1\\tcol2\tback\\\\slash").unwrap();
    assert_eq!(
        fields,
        vec![
            Some("line1\nline2".to_string()),
            Some("col1\tcol2".to_string()),
            Some("back\\slash".to_string()),
        ]
    );
}

#[test]
fn test_roundtrip_text() {
    let mut encoder = TextCopyEncoder::new();
    encoder.add_row(&[Some("hello\tworld"), None, Some("back\\slash\nnewline")]);

    let data = String::from_utf8(encoder.finish()).unwrap();
    let rows = TextCopyDecoder::parse_all(&data).unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], Some("hello\tworld".to_string()));
    assert_eq!(rows[0][1], None);
    assert_eq!(rows[0][2], Some("back\\slash\nnewline".to_string()));
}

#[test]
fn test_multiple_rows() {
    let mut encoder = TextCopyEncoder::new();
    encoder.add_row(&[Some("1"), Some("Alice")]);
    encoder.add_row(&[Some("2"), Some("Bob")]);
    encoder.add_row(&[Some("3"), Some("Charlie")]);

    let data = String::from_utf8(encoder.finish()).unwrap();
    let rows = TextCopyDecoder::parse_all(&data).unwrap();

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[2][1], Some("Charlie".to_string()));
}
