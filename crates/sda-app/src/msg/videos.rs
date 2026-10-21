//! Videos tab messages. Owned by the Videos tab task.

use sda_core::downloader::{DownloadEvent, Probe};
use std::path::PathBuf;
use std::sync::Arc;

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
    /// Open the file picker for a video to present.
    OpenFile,
    FileChosen(Option<PathBuf>),
    /// The player for the picked file, or why it could not open.
    Opened(Result<Arc<sda_media::Player>, String>),
    /// Play or pause the preview player, which is the live one once presented.
    Toggle,
    /// Seek to a position in seconds, from the slider.
    Seek(f32),
    /// Step by seconds, the arrows and the step buttons.
    Step(i64),
    Volume(f32),
    /// Send the preview player live.
    Present,
}
