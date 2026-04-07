use bytes::BytesMut;

use sentinel_driver::types::bit::PgBit;
use sentinel_driver::types::{FromSql, Oid, ToSql};

fn roundtrip(val: &PgBit) {
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgBit::from_sql(&buf).ok();
    assert_eq!(decoded.as_ref(), Some(val));
}

// -- Roundtrip tests --

#[test]
fn test_bit_single_byte() {
    roundtrip(&PgBit {
        data: vec![0b1010_1010],
        bit_length: 8,
    });
}

#[test]
fn test_bit_partial_byte() {
    // 5 bits: 10110 stored as 10110_000 (padded with trailing zeros)
    roundtrip(&PgBit {
        data: vec![0b1011_0000],
        bit_length: 5,
    });
}

#[test]
fn test_bit_multi_byte() {
    roundtrip(&PgBit {
        data: vec![0xFF, 0x00, 0xAB],
        bit_length: 24,
    });
}

#[test]
fn test_bit_empty() {
    roundtrip(&PgBit {
        data: vec![],
        bit_length: 0,
    });
}

#[test]
fn test_bit_one_bit() {
    roundtrip(&PgBit {
        data: vec![0b1000_0000],
        bit_length: 1,
    });
}

// -- Wire format tests --

#[test]
fn test_bit_encode_wire_format() {
    let mut buf = BytesMut::new();
    let val = PgBit {
        data: vec![0b1010_0000],
        bit_length: 4,
    };
    val.to_sql(&mut buf).ok();
    // bit_length(i32) + data(1 byte) = 5 bytes
    assert_eq!(buf.len(), 5);
    assert_eq!(&buf[0..4], &4i32.to_be_bytes()); // bit_length = 4
    assert_eq!(buf[4], 0b1010_0000); // data byte
}

#[test]
fn test_bit_encode_empty() {
    let mut buf = BytesMut::new();
    let val = PgBit {
        data: vec![],
        bit_length: 0,
    };
    val.to_sql(&mut buf).ok();
    assert_eq!(buf.len(), 4); // just bit_length(i32) = 0
    assert_eq!(&buf[..], &0i32.to_be_bytes());
}

// -- Decode error tests --

#[test]
fn test_bit_decode_too_short() {
    let buf = [0u8; 2]; // less than 4 bytes header
    assert!(PgBit::from_sql(&buf).is_err());
}

#[test]
fn test_bit_decode_data_truncated() {
    // Header says 16 bits (2 bytes) but only 1 byte of data
    let mut buf = vec![];
    buf.extend_from_slice(&16i32.to_be_bytes());
    buf.push(0xFF); // only 1 byte instead of 2
    assert!(PgBit::from_sql(&buf).is_err());
}

#[test]
fn test_bit_decode_negative_length() {
    let buf = (-1i32).to_be_bytes();
    assert!(PgBit::from_sql(&buf).is_err());
}

// -- OID tests --

#[test]
fn test_bit_to_sql_oid() {
    let val = PgBit {
        data: vec![],
        bit_length: 0,
    };
    assert_eq!(val.oid(), Oid::VARBIT);
}

#[test]
fn test_bit_from_sql_oid() {
    assert_eq!(<PgBit as FromSql>::oid(), Oid::VARBIT);
}

// -- Helper method tests --

#[test]
fn test_bit_from_bools() {
    let bits = PgBit::from_bools(&[true, false, true, true, false]);
    assert_eq!(bits.bit_length, 5);
    assert_eq!(bits.data, vec![0b1011_0000]);
}

#[test]
fn test_bit_from_bools_full_byte() {
    let bits = PgBit::from_bools(&[true, true, false, false, true, false, true, false]);
    assert_eq!(bits.bit_length, 8);
    assert_eq!(bits.data, vec![0b1100_1010]);
}

#[test]
fn test_bit_from_bools_empty() {
    let bits = PgBit::from_bools(&[]);
    assert_eq!(bits.bit_length, 0);
    assert!(bits.data.is_empty());
}

#[test]
fn test_bit_from_bools_multi_byte() {
    // 9 bits: 1 full byte + 1 partial
    let bits = PgBit::from_bools(&[true; 9]);
    assert_eq!(bits.bit_length, 9);
    assert_eq!(bits.data, vec![0xFF, 0b1000_0000]);
}

#[test]
fn test_bit_get() {
    let bits = PgBit::from_bools(&[true, false, true, true]);
    assert_eq!(bits.get(0), Some(true));
    assert_eq!(bits.get(1), Some(false));
    assert_eq!(bits.get(2), Some(true));
    assert_eq!(bits.get(3), Some(true));
    assert_eq!(bits.get(4), None); // out of bounds
}

#[test]
fn test_bit_len() {
    let bits = PgBit::from_bools(&[true, false, true]);
    assert_eq!(bits.len(), 3);
    assert!(!bits.is_empty());

    let empty = PgBit::from_bools(&[]);
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
}

// -- Array tests --

#[test]
fn test_bit_array_roundtrip() {
    let val = vec![
        PgBit::from_bools(&[true, false]),
        PgBit::from_bools(&[false, true, true, true]),
    ];
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = Vec::<PgBit>::from_sql(&buf).ok();
    assert_eq!(decoded.as_ref(), Some(&val));
    assert_eq!(val.oid(), Oid::VARBIT_ARRAY);
}

#[test]
fn test_bit_array_empty() {
    let val: Vec<PgBit> = vec![];
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = Vec::<PgBit>::from_sql(&buf).ok();
    assert_eq!(decoded, Some(vec![]));
}
