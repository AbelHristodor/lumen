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

/// The first `n` entries, or all of them when the list is shorter.
pub fn recent(list: &[HistoryEntry], n: usize) -> &[HistoryEntry] {
    &list[..n.min(list.len())]
}
