use super::{Connection, Notification, Result, notify};

impl Connection {
    /// Subscribe to LISTEN/NOTIFY on a channel.
    pub async fn listen(&mut self, channel: &str) -> Result<()> {
        notify::listen(&mut self.conn, channel).await
    }

    /// Unsubscribe from a channel.
    pub async fn unlisten(&mut self, channel: &str) -> Result<()> {
        notify::unlisten(&mut self.conn, channel).await
    }

    /// Unsubscribe from all channels.
    pub async fn unlisten_all(&mut self) -> Result<()> {
        notify::unlisten_all(&mut self.conn).await
    }

    /// Send a notification on a channel.
    pub async fn notify(&mut self, channel: &str, payload: &str) -> Result<()> {
        notify::notify(&mut self.conn, channel, payload).await
    }

    /// Wait for the next LISTEN/NOTIFY notification.
    ///
    /// Blocks until a notification arrives on any subscribed channel.
    pub async fn wait_for_notification(&mut self) -> Result<Notification> {
        notify::wait_for_notification(&mut self.conn).await
    }
}
