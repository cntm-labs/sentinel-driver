use std::path::PathBuf;
use std::time::Duration;

use sentinel_driver::config::{ChannelBinding, Config, SslMode};

#[test]
fn parse_basic_connection_string() {
    let config = Config::parse("postgres://user:pass@localhost:5432/mydb").unwrap();
    assert_eq!(config.user(), "user");
    assert_eq!(config.password(), Some("pass"));
    assert_eq!(config.host(), "localhost");
    assert_eq!(config.port(), 5432);
    assert_eq!(config.database(), "mydb");
}

#[test]
fn parse_connection_string_with_params() {
    let config =
        Config::parse("postgres://u:p@host/db?sslmode=require&application_name=test").unwrap();
    assert_eq!(config.ssl_mode(), SslMode::Require);
    assert_eq!(config.application_name(), Some("test"));
}

#[test]
fn parse_connection_string_default_port() {
    let config = Config::parse("postgres://u:p@myhost/db").unwrap();
    assert_eq!(config.host(), "myhost");
    assert_eq!(config.port(), 5432);
}

#[test]
fn parse_percent_encoded_password() {
    let config = Config::parse("postgres://user:p%40ss%23@host/db").unwrap();
    assert_eq!(config.password(), Some("p@ss#"));
}

#[test]
fn builder_defaults() {
    let config = Config::builder().user("test").database("testdb").build();
    assert_eq!(config.host(), "localhost");
    assert_eq!(config.port(), 5432);
    assert_eq!(config.ssl_mode(), SslMode::Prefer);
    assert_eq!(config.connect_timeout(), Duration::from_secs(10));
}

#[test]
fn invalid_scheme_rejected() {
    assert!(Config::parse("mysql://user:pass@host/db").is_err());
}

#[test]
fn test_parse_statement_timeout() {
    let config = Config::parse("postgres://user:pass@localhost/db?statement_timeout=5").unwrap();
    assert_eq!(config.statement_timeout(), Some(Duration::from_secs(5)));
}

#[test]
fn test_statement_timeout_default_none() {
    let config = Config::parse("postgres://user:pass@localhost/db").unwrap();
    assert_eq!(config.statement_timeout(), None);
}

#[test]
fn test_builder_statement_timeout() {
    let config = Config::builder()
        .user("test")
        .statement_timeout(Duration::from_secs(10))
        .build();
    assert_eq!(config.statement_timeout(), Some(Duration::from_secs(10)));
}

#[test]
fn test_invalid_statement_timeout_rejected() {
    let result = Config::parse("postgres://user:pass@localhost/db?statement_timeout=abc");
    assert!(result.is_err());
}

// --- Phase 3A: Client cert, Direct TLS, Channel binding config tests ---

#[test]
fn test_builder_ssl_client_cert_and_key() {
    let config = Config::builder()
        .user("test")
        .ssl_client_cert("/path/to/cert.pem")
        .ssl_client_key("/path/to/key.pem")
        .build();
    assert_eq!(
        config.ssl_client_cert(),
        Some(PathBuf::from("/path/to/cert.pem").as_path())
    );
    assert_eq!(
        config.ssl_client_key(),
        Some(PathBuf::from("/path/to/key.pem").as_path())
    );
}

#[test]
fn test_builder_ssl_direct() {
    let config = Config::builder().user("test").ssl_direct(true).build();
    assert!(config.ssl_direct());
}

#[test]
fn test_builder_ssl_direct_default_false() {
    let config = Config::builder().user("test").build();
    assert!(!config.ssl_direct());
}

#[test]
fn test_builder_channel_binding() {
    let config = Config::builder()
        .user("test")
        .channel_binding(ChannelBinding::Require)
        .build();
    assert_eq!(config.channel_binding(), ChannelBinding::Require);
}

#[test]
fn test_builder_channel_binding_default_prefer() {
    let config = Config::builder().user("test").build();
    assert_eq!(config.channel_binding(), ChannelBinding::Prefer);
}

#[test]
fn test_parse_sslcert_and_sslkey() {
    let config =
        Config::parse("postgres://u:p@host/db?sslcert=/tmp/cert.pem&sslkey=/tmp/key.pem").unwrap();
    assert_eq!(
        config.ssl_client_cert(),
        Some(PathBuf::from("/tmp/cert.pem").as_path())
    );
    assert_eq!(
        config.ssl_client_key(),
        Some(PathBuf::from("/tmp/key.pem").as_path())
    );
}

#[test]
fn test_parse_ssldirect_true() {
    let config = Config::parse("postgres://u:p@host/db?ssldirect=true").unwrap();
    assert!(config.ssl_direct());
}

#[test]
fn test_parse_sslnegotiation_direct() {
    let config = Config::parse("postgres://u:p@host/db?sslnegotiation=direct").unwrap();
    assert!(config.ssl_direct());
}

#[test]
fn test_parse_sslnegotiation_postgres() {
    let config = Config::parse("postgres://u:p@host/db?sslnegotiation=postgres").unwrap();
    assert!(!config.ssl_direct());
}

#[test]
fn test_parse_channel_binding_require() {
    let config = Config::parse("postgres://u:p@host/db?channel_binding=require").unwrap();
    assert_eq!(config.channel_binding(), ChannelBinding::Require);
}

#[test]
fn test_parse_channel_binding_disable() {
    let config = Config::parse("postgres://u:p@host/db?channel_binding=disable").unwrap();
    assert_eq!(config.channel_binding(), ChannelBinding::Disable);
}

#[test]
fn test_parse_invalid_channel_binding() {
    let result = Config::parse("postgres://u:p@host/db?channel_binding=invalid");
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_ssldirect() {
    let result = Config::parse("postgres://u:p@host/db?ssldirect=maybe");
    assert!(result.is_err());
}
