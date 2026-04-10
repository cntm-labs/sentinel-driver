use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::config::ChannelBinding;
use crate::connection::stream::PgConnection;
use crate::error::{Error, Result};
use crate::protocol::backend::BackendMessage;
use crate::protocol::frontend;

type HmacSha256 = Hmac<Sha256>;

/// Perform SCRAM-SHA-256 (or SCRAM-SHA-256-PLUS) authentication with the server.
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
    channel_binding: ChannelBinding,
    server_cert_der: Option<&[u8]>,
) -> Result<()> {
    // Determine mechanism and GS2 header based on channel binding config
    let has_plus = mechanisms.iter().any(|m| m == "SCRAM-SHA-256-PLUS");
    let has_plain = mechanisms.iter().any(|m| m == "SCRAM-SHA-256");
    let is_tls = server_cert_der.is_some();

    let (mechanism, gs2_header) = select_mechanism(channel_binding, is_tls, has_plus, has_plain)?;

    // SASLprep the password (RFC 7613)
    let prepped_password = saslprep(password)?;

    // Generate client nonce
    let client_nonce = generate_nonce();

    // Client-first-message-bare: n=,r=<nonce>
    // We don't send a username in the SCRAM exchange; PG uses the startup user.
    let client_first_bare = format!("n=,r={client_nonce}");
    let client_first_message = format!("{gs2_header}{client_first_bare}");

    // Send SASLInitialResponse
    frontend::sasl_initial_response(conn.write_buf(), mechanism, client_first_message.as_bytes());
    conn.send().await?;

    // Receive server-first-message
    let server_first = match conn.recv().await? {
        BackendMessage::AuthenticationSaslContinue { data } => String::from_utf8(data)
            .map_err(|e| Error::Auth(format!("invalid server-first-message: {e}")))?,
        BackendMessage::ErrorResponse { fields } => {
            return Err(Error::server(
                fields.severity,
                fields.code,
                fields.message,
                fields.detail,
                fields.hint,
                fields.position,
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
        return Err(Error::Auth(
            "server nonce doesn't match client nonce".into(),
        ));
    }

    let salt = BASE64
        .decode(&parsed.salt)
        .map_err(|e| Error::Auth(format!("invalid salt base64: {e}")))?;

    // Compute SCRAM proof
    let salted_password = hi(prepped_password.as_bytes(), &salt, parsed.iterations);
    let client_key = hmac_sha256(&salted_password, b"Client Key");
    let stored_key = sha256(&client_key);
    let server_key = hmac_sha256(&salted_password, b"Server Key");

    // Build channel binding data for c= parameter
    let cbind_input = build_channel_binding_data(gs2_header, server_cert_der);
    let channel_binding_b64 = BASE64.encode(&cbind_input);

    // client-final-message-without-proof
    let client_final_without_proof = format!("c={channel_binding_b64},r={}", parsed.nonce);

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
                return Err(Error::Auth("server signature verification failed".into()));
            }
        }
        BackendMessage::ErrorResponse { fields } => {
            return Err(Error::server(
                fields.severity,
                fields.code,
                fields.message,
                fields.detail,
                fields.hint,
                fields.position,
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
        BackendMessage::ErrorResponse { fields } => Err(Error::server(
            fields.severity,
            fields.code,
            fields.message,
            fields.detail,
            fields.hint,
            fields.position,
        )),
        other => Err(Error::protocol(format!(
            "expected AuthenticationOk, got {other:?}"
        ))),
    }
}

/// Select SCRAM mechanism and GS2 header based on channel binding config.
fn select_mechanism(
    channel_binding: ChannelBinding,
    is_tls: bool,
    has_plus: bool,
    has_plain: bool,
) -> Result<(&'static str, &'static str)> {
    match channel_binding {
        ChannelBinding::Require => {
            if !is_tls {
                return Err(Error::Auth("channel binding requires TLS".into()));
            }
            if !has_plus {
                return Err(Error::Auth(
                    "server does not support SCRAM-SHA-256-PLUS".into(),
                ));
            }
            Ok(("SCRAM-SHA-256-PLUS", "p=tls-server-end-point,,"))
        }
        ChannelBinding::Prefer => {
            if is_tls && has_plus {
                Ok(("SCRAM-SHA-256-PLUS", "p=tls-server-end-point,,"))
            } else if has_plain {
                // y,, = client supports channel binding but server doesn't advertise it
                let gs2 = if is_tls { "y,," } else { "n,," };
                Ok(("SCRAM-SHA-256", gs2))
            } else {
                Err(Error::Auth(
                    "server offered no supported SASL mechanisms".into(),
                ))
            }
        }
        ChannelBinding::Disable => {
            if has_plain {
                Ok(("SCRAM-SHA-256", "n,,"))
            } else {
                Err(Error::Auth(
                    "server offered no supported SASL mechanisms".into(),
                ))
            }
        }
    }
}

/// Build the channel binding input bytes: gs2_header + cbind_data.
///
/// For `tls-server-end-point`, cbind_data is SHA-256 hash of the server's DER certificate.
/// For non-PLUS, cbind_data is empty (just the GS2 header).
fn build_channel_binding_data(gs2_header: &str, server_cert_der: Option<&[u8]>) -> Vec<u8> {
    let mut data = gs2_header.as_bytes().to_vec();
    if gs2_header.starts_with("p=tls-server-end-point") {
        if let Some(cert_der) = server_cert_der {
            let hash = sha256(cert_der);
            data.extend_from_slice(&hash);
        }
    }
    data
}

pub struct ServerFirst {
    pub nonce: String,
    pub salt: String,
    pub iterations: u32,
}

pub fn parse_server_first(msg: &str) -> Result<ServerFirst> {
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
pub fn hi(password: &[u8], salt: &[u8], iterations: u32) -> Vec<u8> {
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

pub fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    #[allow(clippy::expect_used)]
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
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
pub fn saslprep(input: &str) -> Result<String> {
    stringprep::saslprep(input)
        .map(std::borrow::Cow::into_owned)
        .map_err(|e| Error::Auth(format!("SASLprep failed: {e}")))
}

/// Generate a random nonce for SCRAM.
pub fn generate_nonce() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..24).map(|_| rng.gen()).collect();
    BASE64.encode(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_mechanism_require_with_tls_and_plus() {
        let (mech, gs2) = select_mechanism(ChannelBinding::Require, true, true, true).unwrap();
        assert_eq!(mech, "SCRAM-SHA-256-PLUS");
        assert_eq!(gs2, "p=tls-server-end-point,,");
    }

    #[test]
    fn test_select_mechanism_require_without_tls() {
        let err = select_mechanism(ChannelBinding::Require, false, false, true).unwrap_err();
        assert!(err.to_string().contains("channel binding requires TLS"));
    }

    #[test]
    fn test_select_mechanism_require_no_plus() {
        let err = select_mechanism(ChannelBinding::Require, true, false, true).unwrap_err();
        assert!(err
            .to_string()
            .contains("does not support SCRAM-SHA-256-PLUS"));
    }

    #[test]
    fn test_select_mechanism_prefer_with_tls_and_plus() {
        let (mech, gs2) = select_mechanism(ChannelBinding::Prefer, true, true, true).unwrap();
        assert_eq!(mech, "SCRAM-SHA-256-PLUS");
        assert_eq!(gs2, "p=tls-server-end-point,,");
    }

    #[test]
    fn test_select_mechanism_prefer_tls_no_plus() {
        let (mech, gs2) = select_mechanism(ChannelBinding::Prefer, true, false, true).unwrap();
        assert_eq!(mech, "SCRAM-SHA-256");
        assert_eq!(gs2, "y,,");
    }

    #[test]
    fn test_select_mechanism_prefer_no_tls() {
        let (mech, gs2) = select_mechanism(ChannelBinding::Prefer, false, false, true).unwrap();
        assert_eq!(mech, "SCRAM-SHA-256");
        assert_eq!(gs2, "n,,");
    }

    #[test]
    fn test_select_mechanism_disable() {
        let (mech, gs2) = select_mechanism(ChannelBinding::Disable, true, true, true).unwrap();
        assert_eq!(mech, "SCRAM-SHA-256");
        assert_eq!(gs2, "n,,");
    }

    #[test]
    fn test_build_channel_binding_no_plus() {
        let data = build_channel_binding_data("n,,", None);
        assert_eq!(data, b"n,,");
        assert_eq!(BASE64.encode(&data), "biws");
    }

    #[test]
    fn test_build_channel_binding_with_plus() {
        let fake_cert = b"fake-server-certificate-der";
        let data = build_channel_binding_data("p=tls-server-end-point,,", Some(fake_cert));
        // Should be: gs2_header bytes + sha256(cert)
        let expected_hash = sha256(fake_cert);
        let mut expected = b"p=tls-server-end-point,,".to_vec();
        expected.extend_from_slice(&expected_hash);
        assert_eq!(data, expected);
    }

    #[test]
    fn test_gs2_header_y_flag() {
        // y,, means client supports CB but server didn't advertise PLUS
        let (mech, gs2) = select_mechanism(ChannelBinding::Prefer, true, false, true).unwrap();
        assert_eq!(mech, "SCRAM-SHA-256");
        assert_eq!(gs2, "y,,");
        let data = build_channel_binding_data(gs2, Some(b"cert"));
        // y,, should NOT include channel binding data
        assert_eq!(data, b"y,,");
    }

    #[test]
    fn test_select_mechanism_prefer_no_mechanisms() {
        let err = select_mechanism(ChannelBinding::Prefer, false, false, false).unwrap_err();
        assert!(err.to_string().contains("no supported SASL mechanisms"));
    }

    #[test]
    fn test_select_mechanism_prefer_tls_no_mechanisms() {
        let err = select_mechanism(ChannelBinding::Prefer, true, false, false).unwrap_err();
        assert!(err.to_string().contains("no supported SASL mechanisms"));
    }

    #[test]
    fn test_select_mechanism_disable_no_plain() {
        let err = select_mechanism(ChannelBinding::Disable, true, true, false).unwrap_err();
        assert!(err.to_string().contains("no supported SASL mechanisms"));
    }

    #[test]
    fn test_select_mechanism_prefer_no_tls_plus_only() {
        // Server only offers PLUS but client has no TLS — should fail
        let err = select_mechanism(ChannelBinding::Prefer, false, true, false).unwrap_err();
        assert!(err.to_string().contains("no supported SASL mechanisms"));
    }
}
