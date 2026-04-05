use sentinel_driver::CancelToken;

#[test]
fn test_cancel_token_public_api_exists() {
    // Verify the method signature exists on Connection at compile time.
    // We can't create a real Connection without PG, but we verify CancelToken
    // has the right shape returned by cancel_token().
    let token = CancelToken::new("localhost", 5432, 100, 200);
    let cloned = token.clone();
    // Verify it can be sent across threads
    std::thread::spawn(move || {
        let _ = cloned;
    })
    .join()
    .expect("thread should complete");
}

#[test]
fn test_cancel_token_clone_send_sync() {
    fn assert_clone_send_sync<T: Clone + Send + Sync>() {}
    assert_clone_send_sync::<CancelToken>();
}

#[test]
fn test_cancel_token_creation() {
    let token = CancelToken::new("localhost", 5432, 12345, 67890);
    // Token should be creatable and cloneable
    let _clone = token.clone();
}

#[tokio::test]
async fn test_cancel_token_cancel_connection_refused() {
    // Cancel to a port with nothing listening should return an error
    let token = CancelToken::new("127.0.0.1", 1, 12345, 67890);
    let result = token.cancel().await;
    assert!(result.is_err());
}
