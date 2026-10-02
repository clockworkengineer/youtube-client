//! # YouTube Data API v3 Quota Tracker & Budget Management
//!
//! The YouTube Data API v3 enforces a daily quota limit (defaulting to 10,000 units/day).
//! Operations incur variable costs:
//! - Read operations (`videos.list`, `channels.list`, `playlists.list`, `subscriptions.list`, `comments.list`): 1 unit
//! - Search queries (`search.list`): 100 units
//! - Write/Mutation operations (`videos.rate`, `comments.insert`, `playlistItems.insert`/`delete`): 50 units
//!
//! [`QuotaTracker`] tracks consumption per UTC day and persists state to disk to prevent accidental
//! quota exhaustion.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Standard YouTube Data API v3 operation costs.
pub mod costs {
    /// Cost of a single page/call of list operations (`videos.list`, `channels.list`, `subscriptions.list`, etc.).
    pub const READ: u32 = 1;
    /// Cost of a search query (`search.list`).
    pub const SEARCH: u32 = 100;
    /// Cost of write / mutation operations (rating, commenting, modifying playlist items).
    pub const WRITE: u32 = 50;
    /// Default free-tier daily quota limit allocated by Google Cloud Console.
    pub const DEFAULT_DAILY_LIMIT: u32 = 10_000;
}

/// Snapshot of current quota consumption.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuotaStatus {
    /// Date of the quota tracking period in `YYYY-MM-DD` UTC format.
    pub date: String,
    /// Total units consumed today.
    pub units_used: u32,
    /// Daily limit in units (default 10,000).
    pub daily_limit: u32,
    /// Number of search queries performed today.
    pub search_count: u32,
    /// Number of read operations performed today.
    pub read_count: u32,
    /// Number of write operations performed today.
    pub write_count: u32,
    /// Last updated timestamp (Unix epoch seconds).
    pub updated_at: u64,
}

impl QuotaStatus {
    /// Calculate the number of units remaining before hitting the daily quota.
    pub fn units_remaining(&self) -> u32 {
        self.daily_limit.saturating_sub(self.units_used)
    }

    /// Calculate percentage of quota used (from `0.0` to `1.0` or higher if exceeded).
    pub fn percentage_used(&self) -> f32 {
        if self.daily_limit == 0 {
            0.0
        } else {
            (self.units_used as f32 / self.daily_limit as f32).min(1.0)
        }
    }

    /// Returns `true` if the quota limit has been reached or exceeded.
    pub fn is_exhausted(&self) -> bool {
        self.units_used >= self.daily_limit
    }

    /// Checks if a proposed operation with the specified cost can be afforded today.
    pub fn can_afford(&self, cost: u32) -> bool {
        self.units_used.saturating_add(cost) <= self.daily_limit
    }
}

/// Persistent, thread-safe YouTube API quota tracker.
pub struct QuotaTracker {
    status: Mutex<QuotaStatus>,
}

impl QuotaTracker {
    /// Create a new quota tracker with the default daily limit (10,000 units).
    pub fn new() -> Self {
        Self::with_limit(costs::DEFAULT_DAILY_LIMIT)
    }

    /// Create a new quota tracker with a custom daily limit.
    pub fn with_limit(daily_limit: u32) -> Self {
        let today = current_utc_date_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            status: Mutex::new(QuotaStatus {
                date: today,
                units_used: 0,
                daily_limit,
                search_count: 0,
                read_count: 0,
                write_count: 0,
                updated_at: now,
            }),
        }
    }

    /// Load quota status from a JSON file, or initialize a new tracker if file does not exist or is invalid.
    pub fn load_from_file_or_new(path: &Path) -> Self {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(status) = serde_json::from_str::<QuotaStatus>(&content) {
                let tracker = Self {
                    status: Mutex::new(status),
                };
                tracker.check_day_rollover();
                return tracker;
            }
        }
        Self::new()
    }

    /// Save the current quota status atomically to disk.
    pub fn save_to_file(&self, path: &Path) -> std::result::Result<(), String> {
        let status = self.status();
        let json_bytes = serde_json::to_vec_pretty(&status)
            .map_err(|e| format!("Failed to serialize quota status: {e}"))?;

        let dir = path
            .parent()
            .ok_or_else(|| "Invalid quota path parent".to_string())?;
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create quota directory: {e}"))?;

        let mut temp_file = tempfile::NamedTempFile::new_in(dir)
            .map_err(|e| format!("Failed to create temp quota file: {e}"))?;
        use std::io::Write;
        temp_file
            .write_all(&json_bytes)
            .map_err(|e| format!("Failed to write quota temp file: {e}"))?;
        temp_file
            .persist(path)
            .map_err(|e| format!("Failed to persist quota file: {e}"))?;

        Ok(())
    }

    /// Check if the date has changed (UTC day rollover), resetting daily counters if necessary.
    fn check_day_rollover(&self) {
        let today = current_utc_date_string();
        let mut guard = self.status.lock().unwrap();
        if guard.date != today {
            guard.date = today;
            guard.units_used = 0;
            guard.search_count = 0;
            guard.read_count = 0;
            guard.write_count = 0;
            guard.updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }
    }

    /// Record a search operation (costs 100 units).
    pub fn record_search(&self) -> QuotaStatus {
        self.record_operation(costs::SEARCH, true, false, false)
    }

    /// Record a read operation (costs 1 unit).
    pub fn record_read(&self) -> QuotaStatus {
        self.record_operation(costs::READ, false, true, false)
    }

    /// Record a write operation (costs 50 units).
    pub fn record_write(&self) -> QuotaStatus {
        self.record_operation(costs::WRITE, false, false, true)
    }

    /// Record an arbitrary cost in units.
    pub fn record_cost(&self, cost: u32) -> QuotaStatus {
        self.record_operation(cost, false, false, false)
    }

    fn record_operation(
        &self,
        cost: u32,
        is_search: bool,
        is_read: bool,
        is_write: bool,
    ) -> QuotaStatus {
        self.check_day_rollover();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut guard = self.status.lock().unwrap();
        guard.units_used = guard.units_used.saturating_add(cost);
        if is_search {
            guard.search_count = guard.search_count.saturating_add(1);
        }
        if is_read {
            guard.read_count = guard.read_count.saturating_add(1);
        }
        if is_write {
            guard.write_count = guard.write_count.saturating_add(1);
        }
        guard.updated_at = now;
        guard.clone()
    }

    /// Get a snapshot of the current quota status.
    pub fn status(&self) -> QuotaStatus {
        self.check_day_rollover();
        self.status.lock().unwrap().clone()
    }
}

impl Default for QuotaTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Format current UTC date as `YYYY-MM-DD` string without pulling in chrono.
fn current_utc_date_string() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Days since Jan 1, 1970
    let days = (now / 86400) as i64;

    // Civil date from day count using Gregorian calendar arithmetic
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_tracker_initial_state() {
        let tracker = QuotaTracker::new();
        let status = tracker.status();
        assert_eq!(status.units_used, 0);
        assert_eq!(status.daily_limit, 10_000);
        assert_eq!(status.units_remaining(), 10_000);
        assert_eq!(status.percentage_used(), 0.0);
        assert!(!status.is_exhausted());
        assert!(status.can_afford(100));
    }

    #[test]
    fn test_quota_operations() {
        let tracker = QuotaTracker::new();
        tracker.record_read();
        tracker.record_read();
        tracker.record_search();
        tracker.record_write();

        let status = tracker.status();
        assert_eq!(status.read_count, 2);
        assert_eq!(status.search_count, 1);
        assert_eq!(status.write_count, 1);
        // 2 * 1 + 100 + 50 = 152
        assert_eq!(status.units_used, 152);
        assert_eq!(status.units_remaining(), 10_000 - 152);
        assert!(status.percentage_used() > 0.015);
    }

    #[test]
    fn test_quota_exhaustion() {
        let tracker = QuotaTracker::with_limit(100);
        assert!(tracker.status().can_afford(100));
        tracker.record_search(); // costs 100
        let status = tracker.status();
        assert!(status.is_exhausted());
        assert_eq!(status.units_remaining(), 0);
        assert!(!status.can_afford(1));
    }

    #[test]
    fn test_quota_persistence() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("quota.json");

        let tracker = QuotaTracker::new();
        tracker.record_search();
        assert!(tracker.save_to_file(&file_path).is_ok());

        let loaded = QuotaTracker::load_from_file_or_new(&file_path);
        let status = loaded.status();
        assert_eq!(status.search_count, 1);
        assert_eq!(status.units_used, 100);
    }

    #[test]
    fn test_utc_date_string_format() {
        let date_str = current_utc_date_string();
        assert_eq!(date_str.len(), 10);
        assert_eq!(&date_str[4..5], "-");
        assert_eq!(&date_str[7..8], "-");
    }
}
