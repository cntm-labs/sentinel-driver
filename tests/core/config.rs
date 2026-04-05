use std::time::Duration;

use sentinel_driver::config::{Config, SslMode};

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
