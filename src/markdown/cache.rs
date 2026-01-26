//! Render caching for performance optimization

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// LRU-style render cache for layout jobs
pub struct RenderCache {
    /// Cache entries keyed by content hash
    entries: HashMap<u64, CacheEntry>,

    /// Maximum number of entries
    max_entries: usize,

    /// Access counter for LRU eviction
    access_counter: u64,
}

/// A cached render entry
#[derive(Clone)]
struct CacheEntry {
    /// The cached layout job (simplified as string for now)
    content_hash: u64,

    /// Last access time (counter value)
    last_access: u64,
}

impl RenderCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(max_entries),
            max_entries,
            access_counter: 0,
        }
    }

    /// Get a cached entry if it exists
    pub fn get(&mut self, key: u64) -> Option<u64> {
        self.access_counter += 1;

        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_access = self.access_counter;
            return Some(entry.content_hash);
        }

        None
    }

    /// Insert an entry into the cache
    pub fn insert(&mut self, key: u64, content_hash: u64) {
        self.access_counter += 1;

        // Evict oldest entries if at capacity
        if self.entries.len() >= self.max_entries {
            self.evict_oldest();
        }

        self.entries.insert(
            key,
            CacheEntry {
                content_hash,
                last_access: self.access_counter,
            },
        );
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.entries.clear();
        self.access_counter = 0;
    }

    /// Evict the oldest (least recently used) entries
    fn evict_oldest(&mut self) {
        // Remove 10% of entries (or at least 1)
        let to_remove = (self.max_entries / 10).max(1);

        let mut entries_by_access: Vec<_> = self
            .entries
            .iter()
            .map(|(k, v)| (*k, v.last_access))
            .collect();

        entries_by_access.sort_by_key(|(_, access)| *access);

        for (key, _) in entries_by_access.into_iter().take(to_remove) {
            self.entries.remove(&key);
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.entries.len(),
            max_entries: self.max_entries,
            access_count: self.access_counter,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub max_entries: usize,
    pub access_count: u64,
}

/// Compute a hash for cache keying
pub fn compute_cache_key(content: &str, context: u64) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    context.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_get() {
        let mut cache = RenderCache::new(10);

        cache.insert(1, 100);
        cache.insert(2, 200);

        assert_eq!(cache.get(1), Some(100));
        assert_eq!(cache.get(2), Some(200));
        assert_eq!(cache.get(3), None);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = RenderCache::new(5);

        for i in 0..10 {
            cache.insert(i, i * 100);
        }

        // Should have evicted some entries
        assert!(cache.entries.len() <= 5);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = RenderCache::new(10);

        cache.insert(1, 100);
        cache.insert(2, 200);

        cache.clear();

        assert_eq!(cache.entries.len(), 0);
        assert_eq!(cache.get(1), None);
    }
}
