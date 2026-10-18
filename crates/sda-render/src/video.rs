//! Video frames from an `ffmpeg` child process. `spawn_decoder` starts
//! ffmpeg writing raw RGBA to a pipe, a thread reads one frame at a time
//! into the `FrameSource`, and the renderer takes `latest` once per frame.
//! Dropping the last `Arc<FrameSource>` kills ffmpeg, which closes the pipe
//! and ends the thread.

use image::RgbaImage;
use std::fmt;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, Weak};

/// Frames ffmpeg produces per second.
const FPS: u32 = 30;

pub struct FrameSource {
    latest: Mutex<Option<Arc<RgbaImage>>>,
    size: (u32, u32),
    child: Mutex<Option<Child>>,
}

impl FrameSource {
    pub fn new(size: (u32, u32)) -> Arc<Self> {
        Arc::new(FrameSource {
            latest: Mutex::new(None),
            size,
            child: Mutex::new(None),
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

impl Drop for FrameSource {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl fmt::Debug for FrameSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FrameSource({}x{})", self.size.0, self.size.1)
    }
}

/// Start `ffmpeg` decoding `media` in a loop at `size` and feed frames into
/// the returned source. Fails when `ffmpeg` is not a file or cannot start.
pub fn spawn_decoder(
    ffmpeg: &Path,
    media: &Path,
    size: (u32, u32),
) -> anyhow::Result<Arc<FrameSource>> {
    if !ffmpeg.is_file() {
        anyhow::bail!("ffmpeg not found at {}", ffmpeg.display());
    }
    let (w, h) = size;
    let mut child = Command::new(ffmpeg)
        // `-re` paces the output at the clip's own frame rate so the newest
        // frame is the one the wall clock asks for.
        .args(["-loglevel", "error", "-re", "-stream_loop", "-1", "-i"])
        .arg(media)
        .args(["-an", "-f", "rawvideo", "-pix_fmt", "rgba", "-s"])
        .arg(format!("{w}x{h}"))
        .args(["-r", &FPS.to_string(), "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("ffmpeg stdout not piped"))?;
    let source = FrameSource::new(size);
    *source.child.lock().unwrap_or_else(|e| e.into_inner()) = Some(child);

    // The thread holds a weak reference so the last strong owner can drop
    // the source, kill ffmpeg, and end this loop through the closed pipe.
    let weak: Weak<FrameSource> = Arc::downgrade(&source);
    std::thread::Builder::new()
        .name("ffmpeg frames".into())
        .spawn(move || {
            let bytes = (w * h * 4) as usize;
            loop {
                let mut buf = vec![0u8; bytes];
                if stdout.read_exact(&mut buf).is_err() {
                    break;
                }
                let Some(src) = weak.upgrade() else { break };
                if let Some(img) = RgbaImage::from_raw(w, h, buf) {
                    src.push(img);
                }
            }
        })?;
    Ok(source)
}
