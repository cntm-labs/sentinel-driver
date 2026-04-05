use sentinel_driver::transaction::{IsolationLevel, TransactionConfig};

#[test]
fn test_begin_default() {
    let config = TransactionConfig::new();
    assert_eq!(config.begin_sql(), "BEGIN");
}

#[test]
fn test_begin_serializable() {
    let config = TransactionConfig::new().isolation(IsolationLevel::Serializable);
    assert_eq!(config.begin_sql(), "BEGIN ISOLATION LEVEL SERIALIZABLE");
}

#[test]
fn test_begin_read_only() {
    let config = TransactionConfig::new().read_only();
    assert_eq!(config.begin_sql(), "BEGIN READ ONLY");
}

#[test]
fn test_begin_full_options() {
    let config = TransactionConfig::new()
        .isolation(IsolationLevel::Serializable)
        .read_only()
        .deferrable(true);
    assert_eq!(
        config.begin_sql(),
        "BEGIN ISOLATION LEVEL SERIALIZABLE, READ ONLY, DEFERRABLE"
    );
}

#[test]
fn test_begin_repeatable_read_write() {
    let config = TransactionConfig::new()
        .isolation(IsolationLevel::RepeatableRead)
        .read_write();
    assert_eq!(
        config.begin_sql(),
        "BEGIN ISOLATION LEVEL REPEATABLE READ, READ WRITE"
    );
}

#[test]
fn test_isolation_level_as_sql() {
    assert_eq!(IsolationLevel::ReadUncommitted.as_sql(), "READ UNCOMMITTED");
    assert_eq!(IsolationLevel::ReadCommitted.as_sql(), "READ COMMITTED");
    assert_eq!(IsolationLevel::RepeatableRead.as_sql(), "REPEATABLE READ");
    assert_eq!(IsolationLevel::Serializable.as_sql(), "SERIALIZABLE");
}
