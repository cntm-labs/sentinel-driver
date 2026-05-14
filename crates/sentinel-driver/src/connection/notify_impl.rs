use super::{notify, Connection, Notification, Result};

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
    ///
    /// Emits a [`crate::Event::Notification`] event on success.
    ///
    /// Note: in-band NoticeResponse messages during regular queries are not
    /// emitted as Notice events — the driver's query path doesn't yet handle
    /// them gracefully. Startup-time notices log via tracing::debug only
    /// (instrumentation isn't installed until after startup).
    pub async fn wait_for_notification(&mut self) -> Result<Notification> {
        let res = notify::wait_for_notification(&mut self.conn).await;
        if let Ok(n) = &res {
            self.instr().on_event(&crate::Event::Notification {
                channel: &n.channel,
                payload: &n.payload,
                pid: n.process_id,
            });
        }
        res
    }
}
