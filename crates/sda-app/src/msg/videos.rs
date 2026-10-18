//! Videos tab messages. Owned by the Videos tab task.

use sda_core::downloader::{DownloadEvent, Probe};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum VideosMsg {
    Url(String),
    Fetch,
    Probed(Result<Probe, String>),
    /// Thumbnail bytes fetched after a probe. `None` when the fetch failed.
    Thumbnail(Option<Vec<u8>>),
    Quality(String),
    ChooseFolder,
    FolderChosen(Option<PathBuf>),
    Download,
    Event(DownloadEvent),
    /// The download event stream closed. Only meaningful while downloading.
    Ended,
    Reveal(PathBuf),
}
