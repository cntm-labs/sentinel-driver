use sentinel_driver::auth::md5::{compute_md5, hex_encode, md5_compute};

#[test]
fn test_md5_basic() {
    // MD5("") = d41d8cd98f00b204e9800998ecf8427e
    let result = md5_compute(b"");
    assert_eq!(hex_encode(&result), "d41d8cd98f00b204e9800998ecf8427e");
}

#[test]
fn test_md5_hello() {
    // MD5("hello") = 5d41402abc4b2a76b9719d911017c592
    let result = md5_compute(b"hello");
    assert_eq!(hex_encode(&result), "5d41402abc4b2a76b9719d911017c592");
}

#[test]
fn test_compute_md5_password() {
    // Verify the two-round MD5 hash format
    let result = compute_md5("user", "pass", &[0x01, 0x02, 0x03, 0x04]);
    assert!(result.starts_with("md5"));
    assert_eq!(result.len(), 3 + 32); // "md5" + 32 hex chars
}

#[test]
fn test_md5_known_vector() {
    // Known PG MD5 auth test vector:
    // user="postgres", password="postgres", salt=[0x93, 0xf8, 0xa3, 0xe4]
    // Expected: md5 + md5(md5("postgrespostgres") + salt)
    let hash1 = md5_compute(b"postgrespostgres");
    let hex1 = hex_encode(&hash1);

    let mut round2_input = hex1.into_bytes();
    round2_input.extend_from_slice(&[0x93, 0xf8, 0xa3, 0xe4]);
    let hash2 = md5_compute(&round2_input);
    let expected = format!("md5{}", hex_encode(&hash2));

    let result = compute_md5("postgres", "postgres", &[0x93, 0xf8, 0xa3, 0xe4]);
    assert_eq!(result, expected);
}
