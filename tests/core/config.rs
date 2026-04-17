use std::path::PathBuf;
use std::time::Duration;

use sentinel_driver::config::{
    ChannelBinding, Config, LoadBalanceHosts, SslMode, TargetSessionAttrs,
};

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

// --- Phase 4C: Multi-host, LoadBalanceHosts, TargetSessionAttrs, Unix socket ---

#[test]
fn parse_multi_host_connection_string() {
    let config =
        Config::parse("postgres://user:pass@host1:5432,host2:5433,host3:5434/mydb").unwrap();
    let hosts = config.hosts();
    assert_eq!(hosts.len(), 3);
    assert_eq!(hosts[0], ("host1".to_string(), 5432));
    assert_eq!(hosts[1], ("host2".to_string(), 5433));
    assert_eq!(hosts[2], ("host3".to_string(), 5434));
    // Backward-compat: host() returns first host
    assert_eq!(config.host(), "host1");
    assert_eq!(config.port(), 5432);
}

#[test]
fn parse_multi_host_default_port() {
    let config = Config::parse("postgres://user:pass@host1,host2:5433/db").unwrap();
    let hosts = config.hosts();
    assert_eq!(hosts.len(), 2);
    assert_eq!(hosts[0], ("host1".to_string(), 5432));
    assert_eq!(hosts[1], ("host2".to_string(), 5433));
}

#[test]
fn parse_single_host_still_works() {
    let config = Config::parse("postgres://user:pass@localhost:5432/mydb").unwrap();
    let hosts = config.hosts();
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0], ("localhost".to_string(), 5432));
    assert_eq!(config.host(), "localhost");
    assert_eq!(config.port(), 5432);
}

#[test]
fn parse_load_balance_hosts_random() {
    let config = Config::parse("postgres://u:p@h1,h2/db?load_balance_hosts=random").unwrap();
    assert_eq!(config.load_balance_hosts(), LoadBalanceHosts::Random);
}

#[test]
fn parse_load_balance_hosts_disable() {
    let config = Config::parse("postgres://u:p@h1,h2/db?load_balance_hosts=disable").unwrap();
    assert_eq!(config.load_balance_hosts(), LoadBalanceHosts::Disable);
}

#[test]
fn parse_invalid_load_balance_hosts() {
    let result = Config::parse("postgres://u:p@h1/db?load_balance_hosts=invalid");
    assert!(result.is_err());
}

#[test]
fn parse_target_session_attrs_read_write() {
    let config = Config::parse("postgres://u:p@host/db?target_session_attrs=read-write").unwrap();
    assert_eq!(config.target_session_attrs(), TargetSessionAttrs::ReadWrite);
}

#[test]
fn parse_target_session_attrs_read_only() {
    let config = Config::parse("postgres://u:p@host/db?target_session_attrs=read-only").unwrap();
    assert_eq!(config.target_session_attrs(), TargetSessionAttrs::ReadOnly);
}

#[test]
fn parse_target_session_attrs_any() {
    let config = Config::parse("postgres://u:p@host/db?target_session_attrs=any").unwrap();
    assert_eq!(config.target_session_attrs(), TargetSessionAttrs::Any);
}

#[test]
fn builder_multi_host_appends() {
    let config = Config::builder()
        .host("primary.pg.example.com")
        .host("replica1.pg.example.com")
        .port(5432)
        .user("test")
        .build();
    let hosts = config.hosts();
    assert_eq!(hosts.len(), 2);
    assert_eq!(hosts[0].0, "primary.pg.example.com");
    assert_eq!(hosts[1].0, "replica1.pg.example.com");
}

#[test]
fn builder_load_balance_hosts() {
    let config = Config::builder()
        .user("test")
        .load_balance_hosts(LoadBalanceHosts::Random)
        .build();
    assert_eq!(config.load_balance_hosts(), LoadBalanceHosts::Random);
}

#[test]
fn builder_load_balance_hosts_default_disable() {
    let config = Config::builder().user("test").build();
    assert_eq!(config.load_balance_hosts(), LoadBalanceHosts::Disable);
}

#[test]
fn builder_target_session_attrs() {
    let config = Config::builder()
        .user("test")
        .target_session_attrs(TargetSessionAttrs::ReadWrite)
        .build();
    assert_eq!(config.target_session_attrs(), TargetSessionAttrs::ReadWrite);
}

#[test]
fn builder_target_session_attrs_default_any() {
    let config = Config::builder().user("test").build();
    assert_eq!(config.target_session_attrs(), TargetSessionAttrs::Any);
}

#[cfg(unix)]
#[test]
fn parse_unix_socket_host() {
    let config = Config::parse("postgres://user@/db?host=/var/run/postgresql").unwrap();
    let hosts = config.hosts();
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].0, "/var/run/postgresql");
    assert_eq!(hosts[0].1, 5432);
}

#[cfg(unix)]
#[test]
fn builder_unix_socket_host() {
    let config = Config::builder()
        .host("/var/run/postgresql")
        .port(5433)
        .user("test")
        .database("mydb")
        .build();
    let hosts = config.hosts();
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].0, "/var/run/postgresql");
    assert_eq!(hosts[0].1, 5433);
}

#[test]
fn parse_multi_host_with_all_params() {
    let config = Config::parse(
        "postgres://user:pass@h1:5432,h2:5433/db?target_session_attrs=read-write&load_balance_hosts=random",
    )
    .unwrap();
    let hosts = config.hosts();
    assert_eq!(hosts.len(), 2);
    assert_eq!(config.target_session_attrs(), TargetSessionAttrs::ReadWrite);
    assert_eq!(config.load_balance_hosts(), LoadBalanceHosts::Random);
}
