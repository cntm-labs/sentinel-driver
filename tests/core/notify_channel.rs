use sentinel_driver::notify::channel::NotificationDispatcher;
use sentinel_driver::notify::Notification;

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
