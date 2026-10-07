//! # In-Memory Metadata Cache with Time-to-Live (TTL)
//!
//! Provides thread-safe, bounded caching for expensive metadata lookups (video details,
//! channel information, dislike metrics). Caching metadata substantially reduces YouTube
//! Data API v3 quota consumption and improves GUI snappiness when navigating between views.

use std::collections::HashMap;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::models::{ChannelDetails, VideoDetails};

#[derive(Clone)]
struct CacheEntry<V> {
    value: V,
    expires_at: Instant,
    expires_at_epoch_secs: u64,
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
        let epoch_now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + ttl,
            expires_at_epoch_secs: epoch_now + ttl.as_secs(),
        };
        let mut map = self.entries.write().unwrap();
        map.insert(key, entry);
    }

    /// Insert an item with an explicit UNIX epoch expiration timestamp.
    pub fn insert_with_epoch(&self, key: K, value: V, expires_at_epoch_secs: u64) {
        let epoch_now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if expires_at_epoch_secs > epoch_now {
            let rem_secs = expires_at_epoch_secs - epoch_now;
            let ttl = Duration::from_secs(rem_secs);
            let entry = CacheEntry {
                value,
                expires_at: Instant::now() + ttl,
                expires_at_epoch_secs,
            };
            let mut map = self.entries.write().unwrap();
            map.insert(key, entry);
        }
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

    /// Export unexpired entries along with their expiration epoch timestamps.
    pub fn export_entries(&self) -> Vec<(K, V, u64)> {
        let now = Instant::now();
        let map = self.entries.read().unwrap();
        map.iter()
            .filter(|(_, entry)| entry.expires_at > now)
            .map(|(k, entry)| (k.clone(), entry.value.clone(), entry.expires_at_epoch_secs))
            .collect()
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

#[derive(serde::Serialize, serde::Deserialize)]
struct PersistentItem<T> {
    key: String,
    value: T,
    expires_at_epoch_secs: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct PersistentCacheData {
    videos: Vec<PersistentItem<VideoDetails>>,
    channels: Vec<PersistentItem<ChannelDetails>>,
}

/// Specialized metadata cache for YouTube domain types with optional atomic disk persistence.
pub struct MetadataCache {
    videos: TtlCache<String, VideoDetails>,
    channels: TtlCache<String, ChannelDetails>,
    persistence_path: Option<PathBuf>,
}

impl MetadataCache {
    /// Create a new in-memory `MetadataCache` with 30-minute video TTL and 2-hour channel TTL.
    pub fn new() -> Self {
        Self {
            videos: TtlCache::new(Duration::from_secs(1800)),
            channels: TtlCache::new(Duration::from_secs(7200)),
            persistence_path: None,
        }
    }

    /// Open or create a persistent `MetadataCache` backed by a JSON file.
    ///
    /// Reads unexpired entries from disk on startup and persists updates atomically.
    pub fn open(path: impl AsRef<Path>) -> Self {
        let path_buf = path.as_ref().to_path_buf();
        let cache = Self {
            videos: TtlCache::new(Duration::from_secs(1800)),
            channels: TtlCache::new(Duration::from_secs(7200)),
            persistence_path: Some(path_buf.clone()),
        };

        if path_buf.exists() {
            if let Ok(content) = std::fs::read_to_string(&path_buf) {
                if let Ok(data) = serde_json::from_str::<PersistentCacheData>(&content) {
                    for item in data.videos {
                        cache
                            .videos
                            .insert_with_epoch(item.key, item.value, item.expires_at_epoch_secs);
                    }
                    for item in data.channels {
                        cache
                            .channels
                            .insert_with_epoch(item.key, item.value, item.expires_at_epoch_secs);
                    }
                }
            }
        }

        cache
    }

    /// Flush unexpired in-memory entries to the configured disk persistence file.
    pub fn flush(&self) -> Result<(), String> {
        let Some(ref path) = self.persistence_path else {
            return Ok(());
        };

        let videos = self
            .videos
            .export_entries()
            .into_iter()
            .map(|(key, value, expires_at_epoch_secs)| PersistentItem {
                key,
                value,
                expires_at_epoch_secs,
            })
            .collect();

        let channels = self
            .channels
            .export_entries()
            .into_iter()
            .map(|(key, value, expires_at_epoch_secs)| PersistentItem {
                key,
                value,
                expires_at_epoch_secs,
            })
            .collect();

        let data = PersistentCacheData { videos, channels };
        let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
        crate::utils::write_atomic(path, &json)
    }

    /// Returns the persistence path if disk persistence is enabled.
    pub fn persistence_path(&self) -> Option<&Path> {
        self.persistence_path.as_deref()
    }

    /// Store video details in the cache.
    pub fn set_video(&self, video_id: impl Into<String>, details: VideoDetails) {
        self.videos.insert(video_id.into(), details);
        if self.persistence_path.is_some() {
            let _ = self.flush();
        }
    }

    /// Retrieve video details from the cache if not expired.
    pub fn get_video(&self, video_id: &str) -> Option<VideoDetails> {
        self.videos.get(&video_id.to_string())
    }

    /// Store channel details in the cache.
    pub fn set_channel(&self, channel_id: impl Into<String>, details: ChannelDetails) {
        self.channels.insert(channel_id.into(), details);
        if self.persistence_path.is_some() {
            let _ = self.flush();
        }
    }

    /// Retrieve channel details from the cache if not expired.
    pub fn get_channel(&self, channel_id: &str) -> Option<ChannelDetails> {
        self.channels.get(&channel_id.to_string())
    }

    /// Prune expired records across all cache domains and update disk.
    pub fn prune(&self) {
        self.videos.prune_expired();
        self.channels.prune_expired();
        if self.persistence_path.is_some() {
            let _ = self.flush();
        }
    }

    /// Clear all cached metadata and update disk.
    pub fn clear(&self) {
        self.videos.clear();
        self.channels.clear();
        if self.persistence_path.is_some() {
            let _ = self.flush();
        }
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

    #[test]
    fn test_metadata_cache_disk_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let cache_file = dir.path().join("test_metadata_cache.json");

        let video = VideoDetails {
            id: "vid123".to_string(),
            title: "Test Video".to_string(),
            description: "Desc".to_string(),
            published_at: "2026-01-01T00:00:00Z".to_string(),
            channel_id: "chan456".to_string(),
            channel_title: "Test Channel".to_string(),
            thumbnail_url: "https://example.com/thumb.jpg".to_string(),
            view_count: 1000,
            like_count: 50,
            comment_count: 5,
            duration_seconds: 120,
            duration_formatted: "02:00".to_string(),
            tags: vec!["test".to_string()],
            dislike_count: Some(2),
        };

        let channel = ChannelDetails {
            id: "chan456".to_string(),
            title: "Test Channel".to_string(),
            description: "Channel Desc".to_string(),
            custom_url: Some("@TestChannel".to_string()),
            thumbnail_url: "https://example.com/avatar.jpg".to_string(),
            subscriber_count: 5000,
            video_count: 42,
            view_count: 100000,
        };

        {
            let cache = MetadataCache::open(&cache_file);
            cache.set_video("vid123", video.clone());
            cache.set_channel("chan456", channel.clone());
        }

        assert!(cache_file.exists());

        // Reopen cache in a new instance and verify persisted entries
        {
            let reopened = MetadataCache::open(&cache_file);
            let cached_vid = reopened.get_video("vid123");
            assert_eq!(cached_vid, Some(video));

            let cached_chan = reopened.get_channel("chan456");
            assert_eq!(cached_chan, Some(channel));
        }
    }
}
