//! The two ffmpeg children for one file. Each writes to a pipe, a thread
//! reads the pipe into a bounded channel. The channel's back pressure
//! throttles ffmpeg, so there is no `-re` and no burst after a pause.
//! Dropping the receiver ends the thread and the child dies on its next
//! write, the player also kills it outright on seek and stop.

use crate::audio::{CHANNELS, RATE};
use image::RgbaImage;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::time::Duration;

/// Constant output rate, so frame k sits at `start + k / FPS`.
pub const FPS: u32 = 30;
/// Frames waiting in the channel before ffmpeg blocks. A third of a second.
const QUEUE: usize = 10;
/// PCM chunk the audio thread hands over, 20 ms of stereo.
const CHUNK_SAMPLES: usize = (RATE as usize / 50) * CHANNELS as usize;

/// One decoded picture and its time in the file.
pub struct Timed {
    pub at: Duration,
    pub frame: RgbaImage,
}

/// `src` fitted inside `max`, aspect kept, never upscaled, both sides
/// even because ffmpeg's scaler wants that for yuv sources.
pub fn fit_size(src: (u32, u32), max: (u32, u32)) -> (u32, u32) {
    let (sw, sh) = (src.0.max(1) as f32, src.1.max(1) as f32);
    let k = (max.0 as f32 / sw).min(max.1 as f32 / sh).min(1.0);
    let even = |v: f32| ((v.round() as u32) / 2 * 2).max(2);
    (even(sw * k), even(sh * k))
}

fn seek_args(start: Duration) -> [String; 2] {
    ["-ss".into(), format!("{:.3}", start.as_secs_f64())]
}

/// Start ffmpeg decoding pictures from `start` at `out` pixels. Frames
/// arrive through the receiver with their time, `QUEUE` ahead at most.
pub fn spawn_video(
    ffmpeg: &Path,
    file: &Path,
    start: Duration,
    out: (u32, u32),
) -> anyhow::Result<(Child, Receiver<Timed>)> {
    let (w, h) = out;
    let mut cmd = Command::new(ffmpeg);
    sda_render::hide_console(&mut cmd);
    let mut child = cmd
        .args(["-hide_banner", "-loglevel", "error"])
        .args(seek_args(start))
        .arg("-i")
        .arg(file)
        .args(["-an", "-fps_mode", "cfr", "-r", &FPS.to_string()])
        .args([
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-s",
            &format!("{w}x{h}"),
            "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("ffmpeg stdout not piped"))?;
    let (tx, rx): (SyncSender<Timed>, Receiver<Timed>) = sync_channel(QUEUE);
    std::thread::Builder::new()
        .name("video frames".into())
        .spawn(move || {
            let bytes = (w * h * 4) as usize;
            let mut k: u32 = 0;
            loop {
                let mut buf = vec![0u8; bytes];
                if stdout.read_exact(&mut buf).is_err() {
                    break;
                }
                let Some(frame) = RgbaImage::from_raw(w, h, buf) else {
                    break;
                };
                let at = start + Duration::from_secs(1) * k / FPS;
                if tx.send(Timed { at, frame }).is_err() {
                    break;
                }
                k += 1;
            }
        })?;
    Ok((child, rx))
}

/// Start ffmpeg decoding sound from `start` as interleaved stereo f32 at
/// `RATE`. Chunks of `CHUNK_SAMPLES` arrive through the receiver.
pub fn spawn_audio(
    ffmpeg: &Path,
    file: &Path,
    start: Duration,
) -> anyhow::Result<(Child, Receiver<Vec<f32>>)> {
    let mut cmd = Command::new(ffmpeg);
    sda_render::hide_console(&mut cmd);
    let mut child = cmd
        .args(["-hide_banner", "-loglevel", "error"])
        .args(seek_args(start))
        .arg("-i")
        .arg(file)
        .args([
            "-vn",
            "-f",
            "f32le",
            "-ac",
            &CHANNELS.to_string(),
            "-ar",
            &RATE.to_string(),
            "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("ffmpeg stdout not piped"))?;
    // Half a second of chunks ahead. Bigger means a longer pause lag.
    let (tx, rx) = sync_channel::<Vec<f32>>(25);
    std::thread::Builder::new()
        .name("audio pcm".into())
        .spawn(move || {
            let mut buf = vec![0u8; CHUNK_SAMPLES * 4];
            loop {
                let mut filled = 0;
                while filled < buf.len() {
                    match stdout.read(&mut buf[filled..]) {
                        Ok(0) => break,
                        Ok(n) => filled += n,
                        Err(_) => break,
                    }
                }
                if filled == 0 {
                    break;
                }
                let samples: Vec<f32> = buf[..filled]
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|b| f32::from_le_bytes(*b))
                    .collect();
                if tx.send(samples).is_err() || filled < buf.len() {
                    break;
                }
            }
        })?;
    Ok((child, rx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_size_keeps_aspect_and_evenness() {
        assert_eq!(fit_size((3840, 2160), (1920, 1080)), (1920, 1080));
        assert_eq!(
            fit_size((1280, 720), (1920, 1080)),
            (1280, 720),
            "never upscales"
        );
        assert_eq!(fit_size((1920, 1440), (1920, 1080)), (1440, 1080));
        assert_eq!(
            fit_size((641, 361), (1920, 1080)),
            (640, 360),
            "even sizes for yuv"
        );
    }
}
