use sda_core::downloader::{parse_probe, DownloadRecord};
use std::path::PathBuf;

#[test]
fn parse_probe_dedupes_by_height_and_sorts_desc() {
    let p = parse_probe(include_str!("fixtures/ytdlp_probe.json")).unwrap();
    assert_eq!(p.title, "Sample");
    assert_eq!(p.duration_secs, Some(125));
    let h: Vec<u32> = p.qualities.iter().map(|q| q.height).collect();
    assert_eq!(h, [1080, 720, 360]);
    // largest file wins at equal height
    assert_eq!(p.qualities[0].format_id, "299");
    assert_eq!(p.qualities[0].label, "1080p");
}

#[test]
fn parse_probe_keeps_thumbnail_and_sizes() {
    let p = parse_probe(include_str!("fixtures/ytdlp_probe.json")).unwrap();
    assert_eq!(p.thumbnail.as_deref(), Some("https://x/t.jpg"));
    assert_eq!(p.qualities[0].size_bytes, Some(9500));
    assert_eq!(p.qualities[2].size_bytes, Some(1000));
}

#[test]
fn parse_probe_rejects_non_object() {
    assert!(parse_probe("[]").is_err());
    assert!(parse_probe("not json").is_err());
}

#[test]
fn download_record_round_trips_through_json() {
    let r = DownloadRecord {
        title: "Sample".into(),
        path: PathBuf::from("/tmp/Sample.mp4"),
        at: 1_700_000_000,
    };
    let json = serde_json::to_string(&r).unwrap();
    let back: DownloadRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(back, r);
}
