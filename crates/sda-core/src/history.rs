//! Recently opened hymns. Newest first, one entry per path, capped.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Most entries the history keeps.
pub const MAX_ENTRIES: usize = 50;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub path: PathBuf,
    pub number: Option<String>,
    pub title: String,
    /// Unix seconds when the hymn was opened.
    pub opened_at: i64,
}

/// Put `entry` at the front. Drops any older entry with the same path and
/// trims the list to `MAX_ENTRIES`.
pub fn record(list: &mut Vec<HistoryEntry>, entry: HistoryEntry) {
    list.retain(|e| e.path != entry.path);
    list.insert(0, entry);
    list.truncate(MAX_ENTRIES);
}

/// One presented verse selection. A range presented together is one entry,
/// so Genesis 1:1-3 comes back as one row that restores all three.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerseHistoryEntry {
    pub version: String,
    pub book: usize,
    pub chapter: u32,
    /// The verse numbers presented, in order.
    pub verses: Vec<u32>,
    /// Unix seconds when the verses went live.
    pub presented_at: i64,
}

/// Put `entry` at the front. Drops an older entry for the same version and
/// verses and trims the list to `MAX_ENTRIES`.
pub fn record_verses(list: &mut Vec<VerseHistoryEntry>, entry: VerseHistoryEntry) {
    list.retain(|e| {
        e.version != entry.version
            || e.book != entry.book
            || e.chapter != entry.chapter
            || e.verses != entry.verses
    });
    list.insert(0, entry);
    list.truncate(MAX_ENTRIES);
}

/// Unix seconds now, zero when the clock is before 1970.
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The first `n` entries, or all of them when the list is shorter.
pub fn recent(list: &[HistoryEntry], n: usize) -> &[HistoryEntry] {
    &list[..n.min(list.len())]
}
