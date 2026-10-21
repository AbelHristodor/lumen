//! Pure helpers for presenting video files. No process, no I/O beyond a
//! directory read. The player itself lives in the `sda-media` crate.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// How far one step moves, the arrow keys and the step buttons.
pub const STEP: Duration = Duration::from_secs(10);

const EXTENSIONS: [&str; 6] = ["mp4", "mkv", "mov", "webm", "avi", "m4v"];

/// True for the container extensions ffmpeg plays everywhere, any case.
pub fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| EXTENSIONS.contains(&e.as_str()))
}

/// Video files directly in `dir`, sorted by name without case. A missing
/// or unreadable dir gives an empty list.
pub fn list_videos(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_video_file(p))
        .collect();
    out.sort_by_key(|p| {
        p.file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    });
    out
}

/// `m:ss`, or `h:mm:ss` from one hour. Whole seconds, rounded down.
pub fn format_time(d: Duration) -> String {
    let s = d.as_secs();
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{sec:02}")
    } else {
        format!("{m}:{sec:02}")
    }
}

/// The `Duration: hh:mm:ss.cc` line ffmpeg prints for an input.
pub fn parse_duration(stderr: &str) -> Option<Duration> {
    let line = stderr
        .lines()
        .find(|l| l.trim_start().starts_with("Duration:"))?;
    let value = line.trim_start().strip_prefix("Duration:")?.trim();
    let value = value.split(',').next()?.trim();
    let mut parts = value.split(':');
    let h: u64 = parts.next()?.parse().ok()?;
    let m: u64 = parts.next()?.parse().ok()?;
    let s: f64 = parts.next()?.parse().ok()?;
    Some(Duration::from_secs_f64((h * 3600 + m * 60) as f64 + s))
}

/// The first `WxH` of the first video stream line, the coded picture size.
pub fn parse_size(stderr: &str) -> Option<(u32, u32)> {
    let line = stderr.lines().find(|l| l.contains("Video:"))?;
    line.split([' ', ','])
        .filter_map(|tok| {
            let (w, h) = tok.split_once('x')?;
            Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?))
        })
        .find(|(w, h)| *w >= 16 && *h >= 16)
}
