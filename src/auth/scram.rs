use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use hmac::{Hmac, Mac};
use sha2::{Sha256, Digest};

use crate::connection::stream::PgConnection;
use crate::error::{Error, Result};
use crate::protocol::backend::BackendMessage;
use crate::protocol::frontend;

type HmacSha256 = Hmac<Sha256>;

/// Perform SCRAM-SHA-256 authentication with the server.
///
/// This handles the full 3-message exchange:
/// 1. Client sends SASLInitialResponse with client-first-message
/// 2. Server replies with AuthenticationSaslContinue (server-first-message)
/// 3. Client sends SASLResponse with client-final-message
/// 4. Server replies with AuthenticationSaslFinal (server-final verification)
pub(crate) async fn authenticate(
    conn: &mut PgConnection,
    password: &str,
    mechanisms: &[String],
) -> Result<()> {
    // Check that SCRAM-SHA-256 is offered
    let mechanism = if mechanisms.iter().any(|m| m == "SCRAM-SHA-256") {
        "SCRAM-SHA-256"
    } else {
        return Err(Error::Auth(format!(
            "server offered unsupported SASL mechanisms: {mechanisms:?}"
        )));
    };

    // SASLprep the password (RFC 7613)
    let prepped_password = saslprep(password)?;

    // Generate client nonce
    let client_nonce = generate_nonce();

    // Client-first-message-bare: n=,r=<nonce>
    // We don't send a username in the SCRAM exchange; PG uses the startup user.
    let client_first_bare = format!("n=,r={client_nonce}");
    let client_first_message = format!("n,,{client_first_bare}");

    // Send SASLInitialResponse
    frontend::sasl_initial_response(
        conn.write_buf(),
        mechanism,
        client_first_message.as_bytes(),
    );
    conn.send().await?;

    // Receive server-first-message
    let server_first = match conn.recv().await? {
        BackendMessage::AuthenticationSaslContinue { data } => {
            String::from_utf8(data)
                .map_err(|e| Error::Auth(format!("invalid server-first-message: {e}")))?
        }
        BackendMessage::ErrorResponse { fields } => {
            return Err(Error::server(
                fields.severity, fields.code, fields.message,
                fields.detail, fields.hint, fields.position,
            ));
        }
        other => {
            return Err(Error::protocol(format!(
                "expected SaslContinue, got {other:?}"
            )));
        }
    };

    // Parse server-first-message: r=<nonce>,s=<salt>,i=<iterations>
    let parsed = parse_server_first(&server_first)?;

    // Verify server nonce starts with our client nonce
    if !parsed.nonce.starts_with(&client_nonce) {
        return Err(Error::Auth("server nonce doesn't match client nonce".into()));
    }

    let salt = BASE64
        .decode(&parsed.salt)
        .map_err(|e| Error::Auth(format!("invalid salt base64: {e}")))?;

    // Compute SCRAM proof
    let salted_password = hi(prepped_password.as_bytes(), &salt, parsed.iterations);
    let client_key = hmac_sha256(&salted_password, b"Client Key");
    let stored_key = sha256(&client_key);
    let server_key = hmac_sha256(&salted_password, b"Server Key");

    // client-final-message-without-proof
    let channel_binding = BASE64.encode("n,,");
    let client_final_without_proof = format!("c={channel_binding},r={}", parsed.nonce);

    // AuthMessage = client-first-bare + "," + server-first + "," + client-final-without-proof
    let auth_message = format!("{client_first_bare},{server_first},{client_final_without_proof}");

    let client_signature = hmac_sha256(&stored_key, auth_message.as_bytes());
    let client_proof: Vec<u8> = client_key
        .iter()
        .zip(client_signature.iter())
        .map(|(a, b)| a ^ b)
        .collect();

    let server_signature = hmac_sha256(&server_key, auth_message.as_bytes());

    // Send client-final-message
    let client_final = format!(
        "{client_final_without_proof},p={}",
        BASE64.encode(&client_proof)
    );

    frontend::sasl_response(conn.write_buf(), client_final.as_bytes());
    conn.send().await?;

    // Receive server-final-message
    match conn.recv().await? {
        BackendMessage::AuthenticationSaslFinal { data } => {
            let server_final = String::from_utf8(data)
                .map_err(|e| Error::Auth(format!("invalid server-final-message: {e}")))?;

            // Verify server signature
            let expected_verifier = format!("v={}", BASE64.encode(&server_signature));
            if server_final != expected_verifier {
                return Err(Error::Auth(
                    "server signature verification failed".into(),
                ));
            }
        }
        BackendMessage::ErrorResponse { fields } => {
            return Err(Error::server(
                fields.severity, fields.code, fields.message,
                fields.detail, fields.hint, fields.position,
            ));
        }
        other => {
            return Err(Error::protocol(format!(
                "expected SaslFinal, got {other:?}"
            )));
        }
    }

    // Wait for AuthenticationOk
    match conn.recv().await? {
        BackendMessage::AuthenticationOk => Ok(()),
        BackendMessage::ErrorResponse { fields } => {
            Err(Error::server(
                fields.severity, fields.code, fields.message,
                fields.detail, fields.hint, fields.position,
            ))
        }
        other => Err(Error::protocol(format!(
            "expected AuthenticationOk, got {other:?}"
        ))),
    }
}

struct ServerFirst {
    nonce: String,
    salt: String,
    iterations: u32,
}

fn parse_server_first(msg: &str) -> Result<ServerFirst> {
    let mut nonce = None;
    let mut salt = None;
    let mut iterations = None;

    for part in msg.split(',') {
        if let Some(val) = part.strip_prefix("r=") {
            nonce = Some(val.to_string());
        } else if let Some(val) = part.strip_prefix("s=") {
            salt = Some(val.to_string());
        } else if let Some(val) = part.strip_prefix("i=") {
            iterations = Some(
                val.parse::<u32>()
                    .map_err(|_| Error::Auth(format!("invalid iteration count: {val}")))?,
            );
        }
    }

    Ok(ServerFirst {
        nonce: nonce.ok_or_else(|| Error::Auth("missing nonce in server-first".into()))?,
        salt: salt.ok_or_else(|| Error::Auth("missing salt in server-first".into()))?,
        iterations: iterations
            .ok_or_else(|| Error::Auth("missing iterations in server-first".into()))?,
    })
}

/// Hi(password, salt, iterations) — PBKDF2-HMAC-SHA256.
fn hi(password: &[u8], salt: &[u8], iterations: u32) -> Vec<u8> {
    // U1 = HMAC(password, salt + INT(1))
    let mut salt_with_one = salt.to_vec();
    salt_with_one.extend_from_slice(&1u32.to_be_bytes());

    let mut u_prev = hmac_sha256(password, &salt_with_one);
    let mut result = u_prev.clone();

    for _ in 1..iterations {
        let u_current = hmac_sha256(password, &u_prev);
        for (r, u) in result.iter_mut().zip(u_current.iter()) {
            *r ^= u;
        }
        u_prev = u_current;
    }

    result
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key)
        .expect("HMAC can accept any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn sha256(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// SASLprep (RFC 7613) password normalization.
///
/// This is what sqlx gets wrong — they skip this step, leading to
/// authentication failures with non-ASCII passwords.
fn saslprep(input: &str) -> Result<String> {
    stringprep::saslprep(input)
        .map(|s| s.into_owned())
        .map_err(|e| Error::Auth(format!("SASLprep failed: {e}")))
}

/// Generate a random nonce for SCRAM.
fn generate_nonce() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..24).map(|_| rng.gen()).collect();
    BASE64.encode(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
