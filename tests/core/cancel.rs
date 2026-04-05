use sentinel_driver::CancelToken;

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
