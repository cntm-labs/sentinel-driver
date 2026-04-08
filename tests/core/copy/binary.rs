use sentinel_driver::copy::binary::{BinaryCopyDecoder, BinaryCopyEncoder};

#[test]
fn test_encode_decode_roundtrip() {
    let mut encoder = BinaryCopyEncoder::new();

    // Row 1: int4(42), text("hello")
    encoder.begin_row(2);
    encoder.write_field(&42i32.to_be_bytes());
    encoder.write_field(b"hello");

    // Row 2: int4(7), NULL
    encoder.begin_row(2);
    encoder.write_field(&7i32.to_be_bytes());
    encoder.write_null();

    let data = encoder.finish();

    // Decode
    let mut decoder = BinaryCopyDecoder::new(&data);

    let row1 = decoder.next_row().unwrap().unwrap();
    assert_eq!(row1.len(), 2);
    assert_eq!(row1[0].unwrap(), &42i32.to_be_bytes());
    assert_eq!(row1[1].unwrap(), b"hello");

    let row2 = decoder.next_row().unwrap().unwrap();
    assert_eq!(row2.len(), 2);
    assert_eq!(row2[0].unwrap(), &7i32.to_be_bytes());
    assert!(row2[1].is_none()); // NULL

    // Trailer
    assert!(decoder.next_row().unwrap().is_none());
}

#[test]
fn test_encode_empty() {
    let encoder = BinaryCopyEncoder::new();
    let data = encoder.finish();

    // Should have header + trailer
    let mut decoder = BinaryCopyDecoder::new(&data);
    assert!(decoder.next_row().unwrap().is_none());
}

#[test]
fn test_header_validation() {
    let mut decoder = BinaryCopyDecoder::new(b"not a valid header at all!!!");
    assert!(decoder.parse_header().is_err());
}

#[test]
fn test_encoder_size() {
    let mut encoder = BinaryCopyEncoder::new();

    // Header: 11 + 4 + 4 = 19 bytes
    encoder.begin_row(1);
    // Row: 2 (field count) + 4 (field len) + 4 (data) = 10
    encoder.write_field(&42i32.to_be_bytes());

    let data = encoder.finish();
    // 19 (header) + 10 (row) + 2 (trailer) = 31
    assert_eq!(data.len(), 31);
}

#[test]
fn test_multiple_rows() {
    let mut encoder = BinaryCopyEncoder::new();

    for i in 0..100 {
        encoder.begin_row(1);
        encoder.write_field(&(i as i32).to_be_bytes());
    }

    let data = encoder.finish();
    let mut decoder = BinaryCopyDecoder::new(&data);

    let mut count = 0;
    while let Some(row) = decoder.next_row().unwrap() {
        let val = i32::from_be_bytes(row[0].unwrap().try_into().unwrap());
        assert_eq!(val, count);
        count += 1;
    }
    assert_eq!(count, 100);
}
