use std::sync::Arc;

use rustls::pki_types::ServerName;
use tokio_rustls::TlsConnector;

use crate::config::SslMode;
use crate::error::{Error, Result};

/// Build a `TlsConnector` based on the SSL mode.
///
/// Returns `None` if TLS is disabled.
pub(crate) fn make_tls_connector(ssl_mode: SslMode, host: &str) -> Result<Option<TlsConfig>> {
    match ssl_mode {
        SslMode::Disable => Ok(None),
        SslMode::Prefer | SslMode::Require => {
            // Accept any certificate — no verification.
            // "require" in PG means "encrypt but don't verify".
            let config = rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(NoVerifier))
                .with_no_client_auth();

            let connector = TlsConnector::from(Arc::new(config));
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

            let config = rustls::ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();

            let connector = TlsConnector::from(Arc::new(config));
            let server_name = ServerName::try_from(host.to_string())
                .map_err(|e| Error::Tls(format!("invalid server name '{host}': {e}")))?;

            Ok(Some(TlsConfig {
                connector,
                server_name,
            }))
        }
    }
}

/// TLS configuration ready for connection upgrade.
pub(crate) struct TlsConfig {
    pub connector: TlsConnector,
    pub server_name: ServerName<'static>,
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
