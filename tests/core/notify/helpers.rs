use sentinel_driver::notify::{quote_identifier, quote_literal, validate_channel_name, Notification};

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
