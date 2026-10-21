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

use sda_media::{Player, State};

/// Callers hold the env lock.
fn open_player() -> Player {
    unsafe { std::env::set_var("LUMEN_NO_AUDIO", "1") };
    Player::open(&fake_ffmpeg(), std::path::Path::new("clip.mp4"), (320, 180)).unwrap()
}

#[test]
fn open_starts_paused_at_zero_with_the_probe_info() {
    let _env = env_lock();
    unsafe { std::env::set_var("FAKE_FRAMES", "6") };
    let p = open_player();
    unsafe { std::env::remove_var("FAKE_FRAMES") };
    assert_eq!(p.state(), State::Paused);
    assert_eq!(p.position(), Duration::ZERO);
    assert_eq!(p.duration(), Duration::from_secs(2));
    assert_eq!(p.frames().size(), (160, 90));
    // The first frame shows while paused so the preview is not black.
    for _ in 0..50 {
        p.tick();
        if p.frames().latest().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(p.frames().latest().is_some(), "first frame after open");
}

#[test]
fn play_advances_frames_with_the_clock_and_ends() {
    let _env = env_lock();
    unsafe { std::env::set_var("FAKE_FRAMES", "6") };
    unsafe { std::env::set_var("FAKE_SECONDS", "0") };
    let p = open_player();
    unsafe { std::env::remove_var("FAKE_FRAMES") };
    unsafe { std::env::remove_var("FAKE_SECONDS") };
    // No device under `LUMEN_NO_AUDIO`, so the wall clock drives it.
    assert!(p.muted());
    p.play();
    assert_eq!(p.state(), State::Playing);
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while p.state() == State::Playing && std::time::Instant::now() < deadline {
        p.tick();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(p.state(), State::Ended, "six frames at 30 fps end in 0.2 s");
    assert!(p.position() >= Duration::from_millis(190));
    assert!(p.frames().latest().is_none(), "ended means black");
}

#[test]
fn pause_freezes_position_and_seek_restarts_at_the_new_time() {
    let _env = env_lock();
    unsafe { std::env::set_var("FAKE_FRAMES", "60") };
    let p = open_player();
    p.play();
    std::thread::sleep(Duration::from_millis(50));
    p.tick();
    p.pause();
    let at = p.position();
    std::thread::sleep(Duration::from_millis(50));
    p.tick();
    assert_eq!(p.position(), at);
    p.seek(Duration::from_millis(1500));
    assert_eq!(p.state(), State::Paused, "seek keeps the paused state");
    assert_eq!(p.position(), Duration::from_millis(1500));
    for _ in 0..50 {
        p.tick();
        if p.frames().latest().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let f = p.frames().latest().expect("a frame at the seek point");
    assert_eq!(f.get_pixel(0, 0)[0], 1, "the first frame after a restart");
    p.step(-10);
    assert_eq!(p.position(), Duration::ZERO, "clamped at the start");
    p.step(10);
    assert_eq!(p.position(), Duration::from_secs(2), "clamped at the end");
    unsafe { std::env::remove_var("FAKE_FRAMES") };
}

#[test]
fn volume_is_clamped_and_remembered() {
    let _env = env_lock();
    let p = open_player();
    p.set_volume(1.7);
    assert_eq!(p.volume(), 1.0);
    p.set_volume(0.25);
    assert_eq!(p.volume(), 0.25);
}

#[test]
fn open_fails_with_ffmpegs_words() {
    let _env = env_lock();
    unsafe { std::env::set_var("FAKE_FAIL", "1") };
    let r = Player::open(&fake_ffmpeg(), std::path::Path::new("bad.mp4"), (320, 180));
    unsafe { std::env::remove_var("FAKE_FAIL") };
    assert!(r.unwrap_err().to_string().contains("Invalid data"));
}
