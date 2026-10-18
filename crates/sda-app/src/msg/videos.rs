//! Videos tab messages. Owned by the Videos tab task.

use sda_core::downloader::{DownloadEvent, Probe};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum VideosMsg {
    Url(String),
    Fetch,
    Probed(Result<Probe, String>),
    Quality(String),
    ChooseFolder,
    FolderChosen(Option<PathBuf>),
    Download,
    Event(DownloadEvent),
    Reveal(PathBuf),
}
