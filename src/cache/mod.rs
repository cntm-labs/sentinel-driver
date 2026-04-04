use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use lru::LruCache;
use std::num::NonZeroUsize;

use crate::statement::Statement;

/// Default LRU cache capacity for ad-hoc queries.
const DEFAULT_LRU_CAPACITY: usize = 256;

/// Two-tier prepared statement cache.
///
/// - **Tier 1** (HashMap): Pre-registered queries. Never evicted. O(1) lookup.
/// - **Tier 2** (LRU): Ad-hoc queries. Auto-evicted when full. O(1) amortized.
///
/// Statements are keyed by SQL text. Each cached statement has a unique
/// server-side name for the PG prepared statement protocol.
pub struct StatementCache {
    /// Tier 1: registered (permanent) statements, keyed by user-given name.
    registered: HashMap<String, CachedStatement>,
    /// Tier 2: ad-hoc statements, keyed by SQL text.
    adhoc: LruCache<String, CachedStatement>,
    /// Counter for generating unique statement names.
    name_counter: AtomicU64,
    /// Metrics.
    metrics: CacheMetrics,
}

/// A cached prepared statement entry.
#[derive(Debug, Clone)]
pub struct CachedStatement {
    /// The server-side statement name.
    pub name: String,
    /// The full statement metadata.
    pub statement: Statement,
}

/// Cache hit/miss metrics.
#[derive(Debug, Clone, Default)]
pub struct CacheMetrics {
    pub tier1_hits: u64,
    pub tier2_hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl CacheMetrics {
    /// Total cache hits (tier 1 + tier 2).
    pub fn total_hits(&self) -> u64 {
        self.tier1_hits + self.tier2_hits
    }

    /// Hit rate as a fraction (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_hits() + self.misses;
        if total == 0 {
            0.0
        } else {
            self.total_hits() as f64 / total as f64
        }
    }
}

impl StatementCache {
    /// Create a new statement cache with the default LRU capacity (256).
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_LRU_CAPACITY)
    }

    /// Create a new statement cache with a custom LRU capacity.
    pub fn with_capacity(lru_capacity: usize) -> Self {
        Self {
            registered: HashMap::new(),
            adhoc: LruCache::new(
                NonZeroUsize::new(lru_capacity).unwrap_or(NonZeroUsize::new(1).unwrap()),
            ),
            name_counter: AtomicU64::new(0),
            metrics: CacheMetrics::default(),
        }
    }

    /// Register a statement in Tier 1 (permanent, never evicted).
    ///
    /// The `name` is the user-defined name (also used as the server-side name).
    pub fn register(&mut self, name: String, statement: Statement) {
        self.registered.insert(
            name.clone(),
            CachedStatement {
                name: name.clone(),
                statement,
            },
        );
    }

    /// Look up a registered statement by name (Tier 1).
    pub fn get_registered(&mut self, name: &str) -> Option<&CachedStatement> {
        let result = self.registered.get(name);
        if result.is_some() {
            self.metrics.tier1_hits += 1;
        }
        result
    }

    /// Look up an ad-hoc statement by SQL text (Tier 2).
    pub fn get_adhoc(&mut self, sql: &str) -> Option<&CachedStatement> {
        let result = self.adhoc.get(sql);
        if result.is_some() {
            self.metrics.tier2_hits += 1;
        }
        result
    }

    /// Insert an ad-hoc statement into Tier 2.
    ///
    /// Returns the evicted statement's server-side name if the cache was full,
    /// so the caller can send a Close message to the server.
    pub fn insert_adhoc(&mut self, sql: String, statement: Statement) -> Option<String> {
        let name = self.generate_name();

        // Check if inserting will evict
        let evicted = if self.adhoc.len() == self.adhoc.cap().get() {
            // Peek at the LRU entry that will be evicted
            self.adhoc.peek_lru().map(|(_, cached)| cached.name.clone())
        } else {
            None
        };

        if evicted.is_some() {
            self.metrics.evictions += 1;
        }

        self.adhoc.put(sql, CachedStatement { name, statement });

        evicted
    }

    /// Record a cache miss.
    pub fn record_miss(&mut self) {
        self.metrics.misses += 1;
    }

    /// Get the server-side name for an ad-hoc query, or generate one.
    ///
    /// Checks Tier 2 first. If not found, records a miss and returns `None`.
    pub fn lookup_or_miss(&mut self, sql: &str) -> Option<&CachedStatement> {
        if self.adhoc.get(sql).is_some() {
            self.metrics.tier2_hits += 1;
            self.adhoc.get(sql)
        } else {
            self.metrics.misses += 1;
            None
        }
    }

    /// Get cache metrics.
    pub fn metrics(&self) -> &CacheMetrics {
        &self.metrics
    }

    /// Number of registered (Tier 1) statements.
    pub fn registered_count(&self) -> usize {
        self.registered.len()
    }

    /// Number of cached ad-hoc (Tier 2) statements.
    pub fn adhoc_count(&self) -> usize {
        self.adhoc.len()
    }

    /// Generate a unique server-side statement name.
    fn generate_name(&self) -> String {
        let id = self.name_counter.fetch_add(1, Ordering::Relaxed);
        format!("_sentinel_s{id}")
    }
}

impl Default for StatementCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Oid;

    fn make_stmt(sql: &str) -> Statement {
        Statement::new(String::new(), sql.to_string(), vec![], None)
    }

    #[test]
    fn test_register_and_lookup() {
        let mut cache = StatementCache::new();

        cache.register(
            "find_user".to_string(),
            make_stmt("SELECT * FROM users WHERE id = $1"),
        );

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
        cache.register("s1".to_string(), make_stmt("SELECT 1"));

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

        cache.register("s1".to_string(), make_stmt("SELECT 1"));
        cache.register("s2".to_string(), make_stmt("SELECT 2"));
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
}
