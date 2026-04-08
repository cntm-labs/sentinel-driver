pub mod channel;

use crate::connection::stream::PgConnection;
use crate::error::{Error, Result};
use crate::protocol::backend::BackendMessage;
use crate::protocol::frontend;

/// A notification received from PostgreSQL via LISTEN/NOTIFY.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    /// The PID of the backend process that sent the notification.
    pub process_id: i32,
    /// The channel name.
    pub channel: String,
    /// The payload string (may be empty).
    pub payload: String,
}

/// Subscribe to a channel on the given connection.
///
/// Sends `LISTEN <channel>` and waits for confirmation.
pub(crate) async fn listen(conn: &mut PgConnection, channel: &str) -> Result<()> {
    // Validate channel name (prevent SQL injection)
    validate_channel_name(channel)?;

    let sql = format!("LISTEN {}", quote_identifier(channel));
    frontend::query(conn.write_buf(), &sql);
    conn.send().await?;

    // Expect CommandComplete + ReadyForQuery
    loop {
        match conn.recv().await? {
            BackendMessage::ReadyForQuery { .. } => return Ok(()),
            BackendMessage::ErrorResponse { fields } => {
                drain_until_ready(conn).await.ok();
                return Err(Error::server(
                    fields.severity,
                    fields.code,
                    fields.message,
                    fields.detail,
                    fields.hint,
                    fields.position,
                ));
            }
            _ => {}
        }
    }
}

/// Unsubscribe from a channel.
pub(crate) async fn unlisten(conn: &mut PgConnection, channel: &str) -> Result<()> {
    validate_channel_name(channel)?;

    let sql = format!("UNLISTEN {}", quote_identifier(channel));
    frontend::query(conn.write_buf(), &sql);
    conn.send().await?;

    loop {
        match conn.recv().await? {
            BackendMessage::ReadyForQuery { .. } => return Ok(()),
            BackendMessage::ErrorResponse { fields } => {
                drain_until_ready(conn).await.ok();
                return Err(Error::server(
                    fields.severity,
                    fields.code,
                    fields.message,
                    fields.detail,
                    fields.hint,
                    fields.position,
                ));
            }
            _ => {}
        }
    }
}

/// Unsubscribe from all channels.
pub(crate) async fn unlisten_all(conn: &mut PgConnection) -> Result<()> {
    frontend::query(conn.write_buf(), "UNLISTEN *");
    conn.send().await?;

    loop {
        match conn.recv().await? {
            BackendMessage::ReadyForQuery { .. } => return Ok(()),
            BackendMessage::ErrorResponse { fields } => {
                drain_until_ready(conn).await.ok();
                return Err(Error::server(
                    fields.severity,
                    fields.code,
                    fields.message,
                    fields.detail,
                    fields.hint,
                    fields.position,
                ));
            }
            _ => {}
        }
    }
}

/// Send a notification on a channel.
pub(crate) async fn notify(conn: &mut PgConnection, channel: &str, payload: &str) -> Result<()> {
    validate_channel_name(channel)?;

    // Use pg_notify() function to safely pass the payload as a parameter
    let sql = format!(
        "SELECT pg_notify({}, {})",
        quote_literal(channel),
        quote_literal(payload)
    );
    frontend::query(conn.write_buf(), &sql);
    conn.send().await?;

    loop {
        match conn.recv().await? {
            BackendMessage::ReadyForQuery { .. } => return Ok(()),
            BackendMessage::ErrorResponse { fields } => {
                drain_until_ready(conn).await.ok();
                return Err(Error::server(
                    fields.severity,
                    fields.code,
                    fields.message,
                    fields.detail,
                    fields.hint,
                    fields.position,
                ));
            }
            _ => {}
        }
    }
}

/// Wait for the next notification on the connection.
///
/// This blocks until a NotificationResponse is received.
/// Other messages (ParameterStatus, NoticeResponse) are silently consumed.
pub(crate) async fn wait_for_notification(conn: &mut PgConnection) -> Result<Notification> {
    loop {
        match conn.recv().await? {
            BackendMessage::NotificationResponse {
                process_id,
                channel,
                payload,
            } => {
                return Ok(Notification {
                    process_id,
                    channel,
                    payload,
                });
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
            _ => {}
        }
    }
}

/// Validate that a channel name is safe to use in SQL.
pub fn validate_channel_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::Config("channel name cannot be empty".into()));
    }
    if name.len() > 63 {
        return Err(Error::Config(
            "channel name exceeds 63 character limit".into(),
        ));
    }
    Ok(())
}

/// Quote an identifier for safe use in SQL (double-quote escaping).
pub fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Quote a string literal for safe use in SQL (single-quote escaping).
pub fn quote_literal(val: &str) -> String {
    format!("'{}'", val.replace('\'', "''"))
}

async fn drain_until_ready(conn: &mut PgConnection) -> Result<()> {
    loop {
        if let BackendMessage::ReadyForQuery { .. } = conn.recv().await? {
            return Ok(());
        }
    }
}
