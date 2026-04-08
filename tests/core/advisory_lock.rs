use sentinel_driver::advisory_lock::PgAdvisoryLock;

#[test]
fn test_advisory_lock_new() {
    let lock = PgAdvisoryLock::new(12345);
    assert_eq!(lock.key(), 12345);
}

#[test]
fn test_advisory_lock_from_name() {
    let lock = PgAdvisoryLock::from_name("my_lock");
    // Key is a hash — just verify it's deterministic
    let lock2 = PgAdvisoryLock::from_name("my_lock");
    assert_eq!(lock.key(), lock2.key());
}

#[test]
fn test_advisory_lock_different_names_different_keys() {
    let lock1 = PgAdvisoryLock::from_name("lock_a");
    let lock2 = PgAdvisoryLock::from_name("lock_b");
    assert_ne!(lock1.key(), lock2.key());
}

#[test]
fn test_advisory_lock_debug() {
    let lock = PgAdvisoryLock::new(42);
    let debug = format!("{lock:?}");
    assert!(debug.contains("42"));
}

#[test]
fn test_advisory_lock_clone_copy() {
    let lock = PgAdvisoryLock::new(100);
    let lock2 = lock; // Copy
    let lock3 = lock.clone(); // Clone
    assert_eq!(lock2.key(), lock3.key());
}
