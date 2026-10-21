use sda_core::media::*;
use std::path::Path;
use std::time::Duration;

#[test]
fn video_files_by_extension_any_case() {
    for ok in ["a.mp4", "b.MKV", "c.mov", "d.webm", "e.avi", "f.m4v"] {
        assert!(is_video_file(Path::new(ok)), "{ok}");
    }
    for no in ["a.mp3", "b.png", "c.pptx", "noext", ".mp4.part"] {
        assert!(!is_video_file(Path::new(no)), "{no}");
    }
}

#[test]
fn list_videos_sorts_by_name_and_skips_others() {
    let dir = tempfile::tempdir().unwrap();
    for f in ["zeta.mp4", "Alpha.mkv", "notes.txt", "beta.MOV"] {
        std::fs::write(dir.path().join(f), b"").unwrap();
    }
    std::fs::create_dir(dir.path().join("sub.mp4")).unwrap();
    let names: Vec<String> = list_videos(dir.path())
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["Alpha.mkv", "beta.MOV", "zeta.mp4"]);
    assert!(list_videos(Path::new("/definitely/missing")).is_empty());
}

#[test]
fn format_time_reads_like_a_player() {
    assert_eq!(format_time(Duration::ZERO), "0:00");
    assert_eq!(format_time(Duration::from_secs(42)), "0:42");
    assert_eq!(format_time(Duration::from_secs(190)), "3:10");
    assert_eq!(format_time(Duration::from_secs(3600 + 65)), "1:01:05");
    assert_eq!(format_time(Duration::from_millis(59_999)), "0:59");
}

#[test]
fn parse_duration_and_size_from_ffmpeg_stderr() {
    let info = include_str!("fixtures/ffmpeg_info.txt");
    let d = parse_duration(info).expect("duration line");
    assert!((d.as_secs_f64() - 190.52).abs() < 1e-3, "{d:?}");
    assert_eq!(parse_size(info), Some((1920, 1080)));
    assert_eq!(parse_duration("no duration here"), None);
    assert_eq!(parse_size("Video: h264, yuv420p, 2380 kb/s"), None);
    assert_eq!(parse_duration("  Duration: N/A, start: 0"), None);
}

#[test]
fn step_is_ten_seconds() {
    assert_eq!(STEP, Duration::from_secs(10));
}
