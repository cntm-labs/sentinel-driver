use std::collections::HashMap;

use bytes::BytesMut;
use tracing::{debug, warn};

use crate::auth;
use crate::config::Config;
use crate::connection::stream::PgConnection;
use crate::error::{Error, Result};
use crate::protocol::backend::{BackendMessage, TransactionStatus};
use crate::protocol::frontend;

/// Result of a successful startup handshake.
pub(crate) struct StartupResult {
    pub process_id: i32,
    pub secret_key: i32,
    pub _server_params: HashMap<String, String>,
    pub transaction_status: TransactionStatus,
}

/// Perform the PostgreSQL startup handshake on an established connection.
///
/// This sends the startup message, handles authentication, and waits
/// for the server to become ready.
pub(crate) async fn startup(conn: &mut PgConnection, config: &Config) -> Result<StartupResult> {
    // Build startup params
    let mut params: Vec<(&str, &str)> = Vec::new();

    if let Some(app_name) = config.application_name() {
        params.push(("application_name", app_name));
    }

    // Extra float digits for higher precision
    params.push(("extra_float_digits", "3"));

    // Client encoding
    params.push(("client_encoding", "UTF8"));

    // Send startup message
    let mut buf = BytesMut::new();
    frontend::startup(&mut buf, config.user(), config.database(), &params);
    conn.send_raw(&buf).await?;

    // Process auth and startup messages
    let mut process_id = 0;
    let mut secret_key = 0;
    let mut server_params = HashMap::new();
    let transaction_status;

    loop {
        let msg = conn.recv().await?;

        match msg {
            BackendMessage::AuthenticationOk => {
                debug!("authentication successful");
            }

            BackendMessage::AuthenticationCleartextPassword => {
                let password = config.password().ok_or_else(|| {
                    Error::Auth("server requested password but none provided".into())
                })?;
                warn!("using cleartext password authentication (insecure)");
                frontend::password(conn.write_buf(), password);
                conn.send().await?;
            }

            BackendMessage::AuthenticationMd5Password { salt } => {
                let password = config.password().ok_or_else(|| {
                    Error::Auth("server requested password but none provided".into())
                })?;
                warn!("using MD5 authentication (deprecated, consider SCRAM-SHA-256)");
                let hashed = auth::md5::compute_md5(config.user(), password, &salt);
                frontend::password(conn.write_buf(), &hashed);
                conn.send().await?;
            }

            BackendMessage::AuthenticationSasl { mechanisms } => {
                let password = config.password().ok_or_else(|| {
                    Error::Auth("server requested password but none provided".into())
                })?;

                let server_cert = conn.server_certificate_der();
                auth::scram::authenticate(
                    conn,
                    password,
                    &mechanisms,
                    config.channel_binding(),
                    server_cert.as_deref(),
                )
                .await?;
            }

            BackendMessage::AuthenticationSaslContinue { .. }
            | BackendMessage::AuthenticationSaslFinal { .. } => {
                // These are handled inside scram::authenticate
                return Err(Error::protocol(
                    "unexpected SASL message outside of SCRAM flow",
                ));
            }

            BackendMessage::BackendKeyData {
                process_id: pid,
                secret_key: key,
            } => {
                process_id = pid;
                secret_key = key;
                debug!(pid, "received backend key data");
            }

            BackendMessage::ParameterStatus { name, value } => {
                debug!(name = %name, value = %value, "server parameter");
                server_params.insert(name, value);
            }

            BackendMessage::ReadyForQuery {
                transaction_status: ts,
            } => {
                transaction_status = ts;
                debug!("connection ready");
                break;
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

            BackendMessage::NoticeResponse { fields } => {
                debug!(message = %fields.message, "server notice during startup");
            }

            other => {
                return Err(Error::protocol(format!(
                    "unexpected message during startup: {other:?}"
                )));
            }
        }
    }

    Ok(StartupResult {
        process_id,
        secret_key,
        _server_params: server_params,
        transaction_status,
    })
}
