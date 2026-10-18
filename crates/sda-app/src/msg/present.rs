//! Present tab messages. Owned by the Present tab task.

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum PresentMsg {
    /// The Original toggle for the selected hymn.
    AsIsToggle(bool),
    /// An as is render finished. `Ok` carries the hymn path and its PNGs.
    AsIsRendered(Result<(PathBuf, Vec<PathBuf>), String>),
}
