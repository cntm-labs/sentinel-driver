use sentinel_driver::cache::StatementCache;
use sentinel_driver::statement::Statement;

fn make_stmt(sql: &str) -> Statement {
    Statement::new(String::new(), sql.to_string(), vec![], None)
}

#[test]
fn test_register_and_lookup() {
    let mut cache = StatementCache::new();

    cache.register("find_user", make_stmt("SELECT * FROM users WHERE id = $1"));

    let cached = cache.get_registered("find_user").unwrap();
    assert_eq!(cached.statement.sql(), "SELECT * FROM users WHERE id = $1");
    assert_eq!(cached.name, "find_user");

    assert!(cache.get_registered("nonexistent").is_none());
}

#[test]
fn test_adhoc_cache() {
    let mut cache = StatementCache::new();
    let sql = "SELECT * FROM posts WHERE id = $1";

    // Miss
    assert!(cache.get_adhoc(sql).is_none());

    // Insert
    cache.insert_adhoc(sql.to_string(), make_stmt(sql));

    // Hit
    assert!(cache.get_adhoc(sql).is_some());
}

#[test]
fn test_lru_eviction() {
    let mut cache = StatementCache::with_capacity(2);

    cache.insert_adhoc("SELECT 1".to_string(), make_stmt("SELECT 1"));
    cache.insert_adhoc("SELECT 2".to_string(), make_stmt("SELECT 2"));

    // This should evict "SELECT 1"
    let evicted = cache.insert_adhoc("SELECT 3".to_string(), make_stmt("SELECT 3"));
    assert!(evicted.is_some());

    // "SELECT 1" should be gone
    assert!(cache.get_adhoc("SELECT 1").is_none());
    // "SELECT 2" and "SELECT 3" should be present
    assert!(cache.get_adhoc("SELECT 2").is_some());
    assert!(cache.get_adhoc("SELECT 3").is_some());
}

#[test]
fn test_metrics() {
    let mut cache = StatementCache::new();

    // Register a statement
    cache.register("s1", make_stmt("SELECT 1"));

    // Tier 1 hit
    cache.get_registered("s1");
    assert_eq!(cache.metrics().tier1_hits, 1);

    // Insert and hit ad-hoc
    cache.insert_adhoc("SELECT 2".to_string(), make_stmt("SELECT 2"));
    cache.get_adhoc("SELECT 2");
    assert_eq!(cache.metrics().tier2_hits, 1);

    // Miss
    cache.record_miss();
    assert_eq!(cache.metrics().misses, 1);

    // Hit rate: 2 hits / 3 total = 0.666...
    let rate = cache.metrics().hit_rate();
    assert!((rate - 0.6666).abs() < 0.01);
}

#[test]
fn test_unique_names() {
    let cache = StatementCache::new();
    let n1 = cache.generate_name();
    let n2 = cache.generate_name();
    assert_ne!(n1, n2);
    assert!(n1.starts_with("_sentinel_s"));
}

#[test]
fn test_eviction_metrics() {
    let mut cache = StatementCache::with_capacity(1);

    cache.insert_adhoc("SELECT 1".to_string(), make_stmt("SELECT 1"));
    cache.insert_adhoc("SELECT 2".to_string(), make_stmt("SELECT 2"));

    assert_eq!(cache.metrics().evictions, 1);
}

#[test]
fn test_counts() {
    let mut cache = StatementCache::new();

    cache.register("s1", make_stmt("SELECT 1"));
    cache.register("s2", make_stmt("SELECT 2"));
    assert_eq!(cache.registered_count(), 2);

    cache.insert_adhoc("SELECT 3".to_string(), make_stmt("SELECT 3"));
    assert_eq!(cache.adhoc_count(), 1);
}

#[test]
fn test_lookup_or_miss() {
    let mut cache = StatementCache::new();

    // Miss
    assert!(cache.lookup_or_miss("SELECT 1").is_none());
    assert_eq!(cache.metrics().misses, 1);

    // Insert then hit
    cache.insert_adhoc("SELECT 1".to_string(), make_stmt("SELECT 1"));
    assert!(cache.lookup_or_miss("SELECT 1").is_some());
    assert_eq!(cache.metrics().tier2_hits, 1);
}
