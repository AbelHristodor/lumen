//! Bible tab messages. Owned by the Bible tab task.

use sda_core::bible::{Bible, VersionInfo};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum BibleMsg {
    /// The tab body came into view. Loads the saved version once.
    Opened,
    Reference(String),
    Version(String),
    Book(usize),
    Chapter(u32),
    /// Mouse down on a verse row. Toggles it, or with Shift selects from
    /// the anchor to it.
    Press(u32),
    /// The pointer entered a verse row. Extends a drag.
    Enter(u32),
    /// Mouse up anywhere on the list. Ends a drag.
    Release,
    ClearSelection,
    Search(String),
    Present,
    ManageOpen,
    ManageClose,
    Download(String),
    Remove(String),
    IndexLoaded(Result<Vec<VersionInfo>, String>),
    /// The code of the version that finished downloading.
    Downloaded(Result<String, String>),
    /// The code the load was started for and its result. Dropped when the
    /// user picked another version while it loaded.
    Loaded(String, Result<Arc<Bible>, String>),
}
