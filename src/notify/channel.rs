use std::collections::HashSet;

use tokio::sync::broadcast;

use crate::notify::Notification;

/// Capacity for the broadcast channel.
const DEFAULT_CAPACITY: usize = 256;

/// A notification dispatcher that routes PG notifications to subscribers.
///
/// Wraps a tokio broadcast channel. Multiple receivers can subscribe,
/// and each receives all notifications. Used internally by the listener
/// to fan out notifications from a single dedicated connection.
pub struct NotificationDispatcher {
    sender: broadcast::Sender<Notification>,
    channels: HashSet<String>,
}

impl NotificationDispatcher {
    /// Create a new dispatcher with the default buffer capacity.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Create a new dispatcher with a specific buffer capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            channels: HashSet::new(),
        }
    }

    /// Subscribe to receive notifications.
    ///
    /// Returns a receiver that will get all notifications dispatched
    /// after this point.
    pub fn subscribe(&self) -> NotificationReceiver {
        NotificationReceiver {
            receiver: self.sender.subscribe(),
        }
    }

    /// Dispatch a notification to all subscribers.
    ///
    /// Returns the number of receivers that got the message.
    /// Returns 0 if there are no active subscribers (which is fine).
    pub fn dispatch(&self, notification: Notification) -> usize {
        self.sender.send(notification).unwrap_or(0)
    }

    /// Track that we're listening on a channel.
    pub fn add_channel(&mut self, channel: String) {
        self.channels.insert(channel);
    }

    /// Remove a tracked channel.
    pub fn remove_channel(&mut self, channel: &str) {
        self.channels.remove(channel);
    }

    /// Get all tracked channel names (for re-subscribing on reconnect).
    pub fn channels(&self) -> &HashSet<String> {
        &self.channels
    }

    /// Number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for NotificationDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// A receiver for PG notifications.
pub struct NotificationReceiver {
    receiver: broadcast::Receiver<Notification>,
}

impl NotificationReceiver {
    /// Wait for the next notification.
    ///
    /// Returns `None` if the dispatcher has been dropped (no more notifications).
    /// Skips over lagged messages (if the receiver falls behind).
    pub async fn recv(&mut self) -> Option<Notification> {
        loop {
            match self.receiver.recv().await {
                Ok(notification) => return Some(notification),
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(count = n, "notification receiver lagged, skipped messages");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_notification(channel: &str, payload: &str) -> Notification {
        Notification {
            process_id: 1,
            channel: channel.to_string(),
            payload: payload.to_string(),
        }
    }

    #[test]
    fn test_dispatcher_new() {
        let dispatcher = NotificationDispatcher::new();
        assert_eq!(dispatcher.subscriber_count(), 0);
        assert!(dispatcher.channels().is_empty());
    }

    #[test]
    fn test_dispatcher_dispatch_no_subscribers() {
        let dispatcher = NotificationDispatcher::new();
        let count = dispatcher.dispatch(test_notification("ch", "msg"));
        assert_eq!(count, 0); // no error, just 0 receivers
    }

    #[tokio::test]
    async fn test_dispatcher_single_subscriber() {
        let dispatcher = NotificationDispatcher::new();
        let mut rx = dispatcher.subscribe();

        dispatcher.dispatch(test_notification("orders", "new_order"));

        let n = rx.recv().await.unwrap();
        assert_eq!(n.channel, "orders");
        assert_eq!(n.payload, "new_order");
    }

    #[tokio::test]
    async fn test_dispatcher_multiple_subscribers() {
        let dispatcher = NotificationDispatcher::new();
        let mut rx1 = dispatcher.subscribe();
        let mut rx2 = dispatcher.subscribe();

        assert_eq!(dispatcher.subscriber_count(), 2);

        dispatcher.dispatch(test_notification("ch", "hello"));

        let n1 = rx1.recv().await.unwrap();
        let n2 = rx2.recv().await.unwrap();
        assert_eq!(n1, n2);
        assert_eq!(n1.payload, "hello");
    }

    #[test]
    fn test_channel_tracking() {
        let mut dispatcher = NotificationDispatcher::new();

        dispatcher.add_channel("orders".to_string());
        dispatcher.add_channel("users".to_string());
        assert_eq!(dispatcher.channels().len(), 2);

        dispatcher.remove_channel("orders");
        assert_eq!(dispatcher.channels().len(), 1);
        assert!(dispatcher.channels().contains("users"));
    }

    #[tokio::test]
    async fn test_receiver_closed() {
        let dispatcher = NotificationDispatcher::new();
        let mut rx = dispatcher.subscribe();

        drop(dispatcher);

        // Receiver should return None when dispatcher is dropped
        assert!(rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn test_multiple_notifications() {
        let dispatcher = NotificationDispatcher::new();
        let mut rx = dispatcher.subscribe();

        for i in 0..5 {
            dispatcher.dispatch(test_notification("ch", &format!("msg_{i}")));
        }

        for i in 0..5 {
            let n = rx.recv().await.unwrap();
            assert_eq!(n.payload, format!("msg_{i}"));
        }
    }
}
