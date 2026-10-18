use sda_core::downloader::{
    parse_filepath_line, parse_probe, should_self_update, DownloadRecord, SELF_UPDATE_AFTER_SECS,
};
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

#[test]
fn self_update_runs_when_stamp_missing_or_older_than_seven_days() {
    assert_eq!(SELF_UPDATE_AFTER_SECS, 7 * 24 * 3600);
    assert!(should_self_update(None));
    assert!(should_self_update(Some(SELF_UPDATE_AFTER_SECS + 1)));
    assert!(!should_self_update(Some(SELF_UPDATE_AFTER_SECS)));
    assert!(!should_self_update(Some(0)));
    assert!(!should_self_update(Some(24 * 3600)));
}

#[test]
fn parse_filepath_line_reads_after_move_path() {
    assert_eq!(
        parse_filepath_line("FILEPATH|/tmp/out/My Song.mp4\n"),
        Some(PathBuf::from("/tmp/out/My Song.mp4"))
    );
    assert_eq!(
        parse_filepath_line("FILEPATH|/tmp/a|b.mp4"),
        Some(PathBuf::from("/tmp/a|b.mp4"))
    );
    assert_eq!(parse_filepath_line("FILEPATH|"), None);
    assert_eq!(parse_filepath_line("FILEPATH|NA"), None);
    assert_eq!(parse_filepath_line("PROGRESS|1%|x|y|t"), None);
    assert_eq!(parse_filepath_line("[download] Destination: x"), None);
}
