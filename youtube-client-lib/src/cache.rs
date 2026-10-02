//! # In-Memory Metadata Cache with Time-to-Live (TTL)
//!
//! Provides thread-safe, bounded caching for expensive metadata lookups (video details,
//! channel information, dislike metrics). Caching metadata substantially reduces YouTube
//! Data API v3 quota consumption and improves GUI snappiness when navigating between views.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::models::{ChannelDetails, VideoDetails};

struct CacheEntry<V> {
    value: V,
    expires_at: Instant,
}

/// Generic, thread-safe in-memory cache with time-to-live expiration.
pub struct TtlCache<K, V> {
    entries: RwLock<HashMap<K, CacheEntry<V>>>,
    default_ttl: Duration,
}

impl<K: Eq + Hash + Clone, V: Clone> TtlCache<K, V> {
    /// Create a new TTL cache with the specified default time-to-live duration.
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            default_ttl,
        }
    }

    /// Insert an item into the cache with the default TTL.
    pub fn insert(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, self.default_ttl);
    }

    /// Insert an item into the cache with a custom TTL.
    pub fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + ttl,
        };
        let mut map = self.entries.write().unwrap();
        map.insert(key, entry);
    }

    /// Retrieve a cloned value from the cache if it exists and has not expired.
    pub fn get(&self, key: &K) -> Option<V> {
        let now = Instant::now();
        let map = self.entries.read().unwrap();
        if let Some(entry) = map.get(key) {
            if entry.expires_at > now {
                return Some(entry.value.clone());
            }
        }
        None
    }

    /// Explicitly remove an entry from the cache.
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut map = self.entries.write().unwrap();
        map.remove(key).map(|e| e.value)
    }

    /// Clear all entries from the cache.
    pub fn clear(&self) {
        let mut map = self.entries.write().unwrap();
        map.clear();
    }

    /// Prune expired entries to reclaim memory.
    pub fn prune_expired(&self) {
        let now = Instant::now();
        let mut map = self.entries.write().unwrap();
        map.retain(|_, entry| entry.expires_at > now);
    }

    /// Returns the total number of entries currently stored (including potentially un-pruned expired ones).
    pub fn len(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    /// Returns `true` if the cache contains no entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Specialized metadata cache for YouTube domain types.
pub struct MetadataCache {
    videos: TtlCache<String, VideoDetails>,
    channels: TtlCache<String, ChannelDetails>,
}

impl MetadataCache {
    /// Create a new `MetadataCache` with 30-minute video TTL and 2-hour channel TTL.
    pub fn new() -> Self {
        Self {
            videos: TtlCache::new(Duration::from_secs(1800)),
            channels: TtlCache::new(Duration::from_secs(7200)),
        }
    }

    /// Store video details in the cache.
    pub fn set_video(&self, video_id: impl Into<String>, details: VideoDetails) {
        self.videos.insert(video_id.into(), details);
    }

    /// Retrieve video details from the cache if not expired.
    pub fn get_video(&self, video_id: &str) -> Option<VideoDetails> {
        self.videos.get(&video_id.to_string())
    }

    /// Store channel details in the cache.
    pub fn set_channel(&self, channel_id: impl Into<String>, details: ChannelDetails) {
        self.channels.insert(channel_id.into(), details);
    }

    /// Retrieve channel details from the cache if not expired.
    pub fn get_channel(&self, channel_id: &str) -> Option<ChannelDetails> {
        self.channels.get(&channel_id.to_string())
    }

    /// Prune expired records across all cache domains.
    pub fn prune(&self) {
        self.videos.prune_expired();
        self.channels.prune_expired();
    }

    /// Clear all cached metadata.
    pub fn clear(&self) {
        self.videos.clear();
        self.channels.clear();
    }
}

impl Default for MetadataCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ttl_cache_insert_and_get() {
        let cache = TtlCache::new(Duration::from_secs(60));
        cache.insert("video_1".to_string(), 12345);
        assert_eq!(cache.get(&"video_1".to_string()), Some(12345));
        assert_eq!(cache.get(&"video_2".to_string()), None);
    }

    #[test]
    fn test_ttl_cache_expiration() {
        let cache = TtlCache::new(Duration::from_millis(10));
        cache.insert("short_lived".to_string(), "hello");
        std::thread::sleep(Duration::from_millis(25));
        assert_eq!(cache.get(&"short_lived".to_string()), None);
    }

    #[test]
    fn test_ttl_cache_prune() {
        let cache = TtlCache::new(Duration::from_millis(10));
        cache.insert("k1".to_string(), 1);
        std::thread::sleep(Duration::from_millis(25));
        cache.insert_with_ttl("k2".to_string(), 2, Duration::from_secs(60));
        cache.prune_expired();
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&"k2".to_string()), Some(2));
    }
}
