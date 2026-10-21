#![cfg(unix)]
//! Runs the fake ffmpeg script, never the real one. Unix only, the stub
//! is a shell script. Tests serialize on `ENV` because they share env vars.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

/// Env vars are process wide. Every test holds this while it sets them.
static ENV: Mutex<()> = Mutex::new(());

fn env_lock() -> MutexGuard<'static, ()> {
    ENV.lock().unwrap_or_else(|e| e.into_inner())
}

fn fake_ffmpeg() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/fake_ffmpeg.sh"
    ))
}

#[test]
fn probe_reads_duration_size_and_audio() {
    let _env = env_lock();
    let info = sda_media::probe::probe(&fake_ffmpeg(), std::path::Path::new("clip.mp4")).unwrap();
    assert_eq!(info.duration, Duration::from_secs(2));
    assert_eq!(info.size, (160, 90));
    assert!(info.has_audio);
}

#[test]
fn probe_fails_on_a_broken_file() {
    let _env = env_lock();
    // SAFETY: every test holds the env lock, so no other thread reads or
    // writes env vars while this one does.
    unsafe { std::env::set_var("FAKE_FAIL", "1") };
    let r = sda_media::probe::probe(&fake_ffmpeg(), std::path::Path::new("bad.mp4"));
    unsafe { std::env::remove_var("FAKE_FAIL") };
    let e = r.unwrap_err().to_string();
    assert!(e.contains("Invalid data"), "{e}");
}

#[test]
fn spawn_video_delivers_timed_frames_in_order() {
    use sda_media::decode::{FPS, spawn_video};
    let _env = env_lock();
    unsafe { std::env::set_var("FAKE_FRAMES", "4") };
    let (mut child, rx) = spawn_video(
        &fake_ffmpeg(),
        std::path::Path::new("clip.mp4"),
        Duration::from_secs(1),
        (160, 90),
    )
    .unwrap();
    let frames: Vec<_> = rx.iter().collect();
    unsafe { std::env::remove_var("FAKE_FRAMES") };
    let _ = child.wait();
    assert_eq!(frames.len(), 4);
    assert_eq!(frames[0].at, Duration::from_secs(1));
    assert_eq!(
        frames[1].at,
        Duration::from_secs(1) + Duration::from_secs(1) / FPS
    );
    assert_eq!(frames[0].frame.dimensions(), (160, 90));
    assert_eq!(
        frames[3].frame.get_pixel(0, 0)[0],
        4,
        "the stub counts frames in red"
    );
}

#[test]
fn spawn_audio_delivers_pcm_chunks() {
    use sda_media::audio::{CHANNELS, RATE};
    use sda_media::decode::spawn_audio;
    let _env = env_lock();
    unsafe { std::env::set_var("FAKE_SECONDS", "1") };
    let (mut child, rx) = spawn_audio(
        &fake_ffmpeg(),
        std::path::Path::new("clip.mp4"),
        Duration::ZERO,
    )
    .unwrap();
    let total: usize = rx.iter().map(|c| c.len()).sum();
    unsafe { std::env::remove_var("FAKE_SECONDS") };
    let _ = child.wait();
    assert_eq!(total, RATE as usize * CHANNELS as usize);
}
