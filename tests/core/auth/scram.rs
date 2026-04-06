use sentinel_driver::auth::scram::{
    generate_nonce, hi, hmac_sha256, parse_server_first, saslprep,
};

#[test]
fn test_parse_server_first() {
    let msg = "r=clientNonce+serverNonce,s=c2FsdA==,i=4096";
    let parsed = parse_server_first(msg).unwrap();
    assert_eq!(parsed.nonce, "clientNonce+serverNonce");
    assert_eq!(parsed.salt, "c2FsdA==");
    assert_eq!(parsed.iterations, 4096);
}

#[test]
fn test_hi_known_vector() {
    // RFC 5802 test vector for SCRAM-SHA-1, adapted:
    // We test that Hi produces deterministic output for given inputs.
    let result = hi(b"password", b"salt", 1);
    assert_eq!(result.len(), 32); // SHA-256 output

    // Same inputs should produce same output
    let result2 = hi(b"password", b"salt", 1);
    assert_eq!(result, result2);
}

#[test]
fn test_hi_iterations() {
    // More iterations should produce different result
    let r1 = hi(b"password", b"salt", 1);
    let r4096 = hi(b"password", b"salt", 4096);
    assert_ne!(r1, r4096);
}

#[test]
fn test_hmac_sha256() {
    let result = hmac_sha256(b"key", b"data");
    assert_eq!(result.len(), 32);
}

#[test]
fn test_saslprep_ascii() {
    assert_eq!(saslprep("password").unwrap(), "password");
}

#[test]
fn test_saslprep_unicode() {
    // SASLprep should normalize Unicode
    let result = saslprep("p\u{00E4}ssword");
    assert!(result.is_ok());
}

#[test]
fn test_generate_nonce() {
    let n1 = generate_nonce();
    let n2 = generate_nonce();
    assert_ne!(n1, n2);
    assert!(!n1.is_empty());
}
