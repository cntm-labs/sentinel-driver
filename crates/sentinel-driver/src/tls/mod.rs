use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use tokio_rustls::TlsConnector;

use crate::config::{Config, SslMode};
use crate::error::{Error, Result};

/// Build a `TlsConnector` based on the connection configuration.
///
/// Returns `None` if TLS is disabled.
pub(crate) fn make_tls_connector(config: &Config) -> Result<Option<TlsConfig>> {
    let ssl_mode = config.ssl_mode();
    let host = config.host();

    // Validate: ssl_direct requires TLS to be enabled
    if config.ssl_direct() && ssl_mode == SslMode::Disable {
        return Err(Error::Config(
            "ssl_direct requires TLS to be enabled".into(),
        ));
    }

    match ssl_mode {
        SslMode::Disable => Ok(None),
        SslMode::Prefer | SslMode::Require => {
            // Accept any certificate — no verification.
            // "require" in PG means "encrypt but don't verify".
            let builder = rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(NoVerifier));

            let mut tls_config = apply_client_auth(builder, config)?;

            // Set ALPN for direct TLS (PG 17+)
            if config.ssl_direct() {
                tls_config.alpn_protocols = vec![b"postgresql".to_vec()];
            }

            let connector = TlsConnector::from(Arc::new(tls_config));
            #[allow(clippy::expect_used)]
            let server_name = ServerName::try_from(host.to_string()).unwrap_or_else(|_| {
                ServerName::try_from("localhost".to_string())
                    .expect("localhost is a valid server name")
            });

            Ok(Some(TlsConfig {
                connector,
                server_name,
            }))
        }
        SslMode::VerifyCa | SslMode::VerifyFull => {
            let mut root_store = rustls::RootCertStore::empty();
            root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

            let builder = rustls::ClientConfig::builder().with_root_certificates(root_store);

            let mut tls_config = apply_client_auth(builder, config)?;

            // Set ALPN for direct TLS (PG 17+)
            if config.ssl_direct() {
                tls_config.alpn_protocols = vec![b"postgresql".to_vec()];
            }

            let connector = TlsConnector::from(Arc::new(tls_config));
            let server_name = ServerName::try_from(host.to_string())
                .map_err(|e| Error::Tls(format!("invalid server name '{host}': {e}")))?;

            Ok(Some(TlsConfig {
                connector,
                server_name,
            }))
        }
    }
}

/// Apply client certificate authentication or no-client-auth based on config.
fn apply_client_auth(
    builder: rustls::ConfigBuilder<rustls::ClientConfig, rustls::client::WantsClientCert>,
    config: &Config,
) -> Result<rustls::ClientConfig> {
    match (config.ssl_client_cert(), config.ssl_client_key()) {
        (Some(cert_path), Some(key_path)) => {
            let certs = load_certs(cert_path)?;
            let key = load_private_key(key_path)?;
            builder
                .with_client_auth_cert(certs, key)
                .map_err(|e| Error::Tls(format!("invalid client certificate/key: {e}")))
        }
        (Some(_), None) => Err(Error::Config(
            "ssl_client_cert requires ssl_client_key".into(),
        )),
        (None, Some(_)) => Err(Error::Config(
            "ssl_client_key requires ssl_client_cert".into(),
        )),
        (None, None) => Ok(builder.with_no_client_auth()),
    }
}

/// Load PEM-encoded certificates from a file.
fn load_certs(path: &std::path::Path) -> Result<Vec<CertificateDer<'static>>> {
    let file = std::fs::File::open(path).map_err(|e| {
        Error::Tls(format!(
            "client certificate file not found: {}: {e}",
            path.display()
        ))
    })?;
    let mut reader = std::io::BufReader::new(file);
    let certs: std::result::Result<Vec<_>, _> = rustls_pemfile::certs(&mut reader).collect();
    let certs = certs.map_err(|e| {
        Error::Tls(format!(
            "invalid certificate PEM format: {}: {e}",
            path.display()
        ))
    })?;
    if certs.is_empty() {
        return Err(Error::Tls(format!(
            "no certificates found in {}",
            path.display()
        )));
    }
    Ok(certs)
}

/// Load a PEM-encoded private key from a file.
fn load_private_key(path: &std::path::Path) -> Result<PrivateKeyDer<'static>> {
    let file = std::fs::File::open(path).map_err(|e| {
        Error::Tls(format!(
            "client key file not found: {}: {e}",
            path.display()
        ))
    })?;
    let mut reader = std::io::BufReader::new(file);
    rustls_pemfile::private_key(&mut reader)
        .map_err(|e| Error::Tls(format!("invalid key PEM format: {}: {e}", path.display())))?
        .ok_or_else(|| Error::Tls(format!("no private key found in {}", path.display())))
}

/// TLS configuration ready for connection upgrade.
pub(crate) struct TlsConfig {
    pub connector: TlsConnector,
    pub server_name: ServerName<'static>,
}

impl std::fmt::Debug for TlsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TlsConfig")
            .field("server_name", &self.server_name)
            .finish_non_exhaustive()
    }
}

/// A certificate verifier that accepts anything (for sslmode=require).
#[derive(Debug)]
struct NoVerifier;

impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::aws_lc_rs::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_certs_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("certs")
    }

    #[test]
    fn test_load_valid_cert() {
        let cert_path = test_certs_dir().join("test-client.crt");
        let certs = load_certs(&cert_path).unwrap();
        assert!(!certs.is_empty());
    }

    #[test]
    fn test_load_valid_key() {
        let key_path = test_certs_dir().join("test-client.key");
        let key = load_private_key(&key_path);
        assert!(key.is_ok());
    }

    #[test]
    fn test_load_cert_file_not_found() {
        let err = load_certs(std::path::Path::new("/nonexistent/cert.pem")).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("client certificate file not found"), "{msg}");
    }

    #[test]
    fn test_load_key_file_not_found() {
        let err = load_private_key(std::path::Path::new("/nonexistent/key.pem")).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("client key file not found"), "{msg}");
    }

    #[test]
    fn test_load_invalid_pem_cert() {
        // Use a key file as cert — should fail or return empty
        let key_path = test_certs_dir().join("test-client.key");
        let result = load_certs(&key_path);
        // Key file has no certificates, so it should error with "no certificates found"
        assert!(result.is_err());
    }

    #[test]
    fn test_make_tls_connector_cert_without_key() {
        let config = Config::builder()
            .user("test")
            .ssl_mode(crate::config::SslMode::Require)
            .ssl_client_cert(test_certs_dir().join("test-client.crt"))
            .build();
        let err = make_tls_connector(&config).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("ssl_client_cert requires ssl_client_key"),
            "{msg}"
        );
    }

    #[test]
    fn test_make_tls_connector_key_without_cert() {
        let config = Config::builder()
            .user("test")
            .ssl_mode(crate::config::SslMode::Require)
            .ssl_client_key(test_certs_dir().join("test-client.key"))
            .build();
        let err = make_tls_connector(&config).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("ssl_client_key requires ssl_client_cert"),
            "{msg}"
        );
    }

    #[test]
    fn test_make_tls_connector_with_client_cert() {
        let config = Config::builder()
            .user("test")
            .ssl_mode(crate::config::SslMode::Require)
            .ssl_client_cert(test_certs_dir().join("test-client.crt"))
            .ssl_client_key(test_certs_dir().join("test-client.key"))
            .build();
        let result = make_tls_connector(&config);
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_ssl_direct_requires_tls() {
        let config = Config::builder()
            .user("test")
            .ssl_mode(crate::config::SslMode::Disable)
            .ssl_direct(true)
            .build();
        let err = make_tls_connector(&config).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("ssl_direct requires TLS"), "{msg}");
    }

    #[test]
    fn test_ssl_direct_sets_alpn() {
        let config = Config::builder()
            .user("test")
            .ssl_mode(crate::config::SslMode::Require)
            .ssl_direct(true)
            .build();
        let result = make_tls_connector(&config).unwrap();
        // Just verify it succeeds — ALPN is internal to the ClientConfig
        assert!(result.is_some());
    }
}
