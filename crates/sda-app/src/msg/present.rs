//! Present tab messages. Owned by the Present tab task.

use image::RgbaImage;
use std::path::PathBuf;
use std::sync::Arc;

/// One decoded as is slide, keyed by its PNG path.
pub type Bitmap = (PathBuf, Arc<RgbaImage>);

#[derive(Debug, Clone)]
pub enum PresentMsg {
    /// The Original toggle for the selected hymn.
    AsIsToggle(bool),
    /// An as is render or bitmap load finished off the UI thread. `Ok`
    /// carries the hymn path and its decoded slides in order.
    AsIsRendered(Result<(PathBuf, Vec<Bitmap>), String>),
}
