//! What a file is, from `ffmpeg -i file`. ffmpeg prints the streams on
//! stderr and exits 1 because no output was given. That is the only
//! probe this app needs, so no ffprobe.

use sda_core::media::{parse_duration, parse_size};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Info {
    pub duration: Duration,
    pub size: (u32, u32),
    pub has_audio: bool,
}

/// Run `ffmpeg -i file` and read the stream lines. Fails when ffmpeg is
/// missing or the file has no video stream, with ffmpeg's own words.
pub fn probe(ffmpeg: &Path, file: &Path) -> anyhow::Result<Info> {
    let mut cmd = Command::new(ffmpeg);
    sda_render::hide_console(&mut cmd);
    let out = cmd
        .args(["-hide_banner", "-i"])
        .arg(file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()?;
    let text = String::from_utf8_lossy(&out.stderr);
    let Some(duration) = parse_duration(&text) else {
        let reason = text
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty() && !l.contains("At least one output"))
            .unwrap_or("no duration in ffmpeg output")
            .trim()
            .to_string();
        anyhow::bail!("{reason}");
    };
    let size = parse_size(&text).ok_or_else(|| anyhow::anyhow!("no video stream"))?;
    Ok(Info {
        duration,
        size,
        has_audio: text.contains("Audio:"),
    })
}
