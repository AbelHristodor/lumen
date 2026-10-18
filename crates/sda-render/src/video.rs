//! Frame hand off between a video decoder and the renderer. The decoder
//! arrives in a later task. It calls `push` from its own thread and the
//! renderer reads `latest` once per frame.

use image::RgbaImage;
use std::fmt;
use std::sync::{Arc, Mutex};

pub struct FrameSource {
    latest: Mutex<Option<Arc<RgbaImage>>>,
    size: (u32, u32),
}

impl FrameSource {
    pub fn new(size: (u32, u32)) -> Arc<Self> {
        Arc::new(FrameSource {
            latest: Mutex::new(None),
            size,
        })
    }

    /// Replace the frame the renderer will draw next.
    pub fn push(&self, frame: RgbaImage) {
        *self.latest.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(frame));
    }

    pub fn latest(&self) -> Option<Arc<RgbaImage>> {
        self.latest
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }
}

impl fmt::Debug for FrameSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FrameSource({}x{})", self.size.0, self.size.1)
    }
}
