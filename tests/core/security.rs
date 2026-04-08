use sentinel_driver::{ChannelBinding, Config, SslMode};

#[test]
fn test_channel_binding_default() {
    let config = Config::builder()
        .host("localhost")
        .database("test")
        .user("user")
        .build();
    assert_eq!(config.channel_binding(), ChannelBinding::Prefer);
}

#[test]
fn test_ssl_direct_default_false() {
    let config = Config::builder()
        .host("localhost")
        .database("test")
        .user("user")
        .build();
    assert!(!config.ssl_direct());
}

#[test]
fn test_ssl_client_cert_default_none() {
    let config = Config::builder()
        .host("localhost")
        .database("test")
        .user("user")
        .build();
    assert!(config.ssl_client_cert().is_none());
    assert!(config.ssl_client_key().is_none());
}

#[test]
fn test_channel_binding_variants() {
    let _prefer = ChannelBinding::Prefer;
    let _require = ChannelBinding::Require;
    let _disable = ChannelBinding::Disable;
}

#[test]
fn test_ssl_mode_variants() {
    let _disable = SslMode::Disable;
    let _prefer = SslMode::Prefer;
    let _require = SslMode::Require;
    let _verify_ca = SslMode::VerifyCa;
    let _verify_full = SslMode::VerifyFull;
}
