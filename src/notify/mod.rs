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
            BackendMessage::CommandComplete { .. } => {}
            BackendMessage::ReadyForQuery { .. } => return Ok(()),
            BackendMessage::ErrorResponse { fields } => {
                // Drain until ReadyForQuery
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
            _ => continue,
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
            BackendMessage::CommandComplete { .. } => {}
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
            _ => continue,
        }
    }
}

/// Unsubscribe from all channels.
pub(crate) async fn unlisten_all(conn: &mut PgConnection) -> Result<()> {
    frontend::query(conn.write_buf(), "UNLISTEN *");
    conn.send().await?;

    loop {
        match conn.recv().await? {
            BackendMessage::CommandComplete { .. } => {}
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
            _ => continue,
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
            BackendMessage::CommandComplete { .. }
            | BackendMessage::DataRow { .. }
            | BackendMessage::RowDescription { .. } => {}
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
            _ => continue,
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
            // Silently consume keepalive/status messages
            BackendMessage::ParameterStatus { .. } | BackendMessage::NoticeResponse { .. } => {
                continue
            }
            _ => continue,
        }
    }
}

/// Validate that a channel name is safe to use in SQL.
fn validate_channel_name(name: &str) -> Result<()> {
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
pub(crate) fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Quote a string literal for safe use in SQL (single-quote escaping).
fn quote_literal(val: &str) -> String {
    format!("'{}'", val.replace('\'', "''"))
}

async fn drain_until_ready(conn: &mut PgConnection) -> Result<()> {
    loop {
        match conn.recv().await? {
            BackendMessage::ReadyForQuery { .. } => return Ok(()),
            _ => continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_channel_name() {
        assert!(validate_channel_name("my_channel").is_ok());
        assert!(validate_channel_name("").is_err());
        assert!(validate_channel_name(&"x".repeat(64)).is_err());
        assert!(validate_channel_name(&"x".repeat(63)).is_ok());
    }

    #[test]
    fn test_quote_identifier() {
        assert_eq!(quote_identifier("simple"), "\"simple\"");
        assert_eq!(quote_identifier("has\"quote"), "\"has\"\"quote\"");
        assert_eq!(quote_identifier("MiXeD"), "\"MiXeD\"");
    }

    #[test]
    fn test_quote_literal() {
        assert_eq!(quote_literal("hello"), "'hello'");
        assert_eq!(quote_literal("it's"), "'it''s'");
        assert_eq!(quote_literal(""), "''");
    }

    #[test]
    fn test_notification_struct() {
        let n = Notification {
            process_id: 123,
            channel: "test_channel".to_string(),
            payload: "hello world".to_string(),
        };
        assert_eq!(n.process_id, 123);
        assert_eq!(n.channel, "test_channel");
        assert_eq!(n.payload, "hello world");
    }
}
