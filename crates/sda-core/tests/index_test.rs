use sda_core::index::{crawl_pptx_paths, load_cache, refresh_index, save_cache};
use sda_core::model::{Hymn, Paragraph, Run, Slide, TextBox};
use std::path::{Path, PathBuf};

/// Build a hymn whose single slide holds one run per line.
fn hymn(number: &str, title: &str, lines: &[&str]) -> Hymn {
    let paragraphs = lines
        .iter()
        .map(|l| Paragraph {
            align: None,
            line_spacing: None,
            runs: vec![Run { text: (*l).into(), ..Default::default() }],
        })
        .collect();
    Hymn {
        number: Some(number.into()),
        title: title.into(),
        slides: vec![Slide {
            background: None,
            boxes: vec![TextBox { frame: None, paragraphs }],
            notes: None,
        }],
        path: PathBuf::from(format!("/x/{number}.pptx")),
        library: "L".into(),
        mtime: 0,
    }
}


#[test]
fn crawl_skips_lock_and_non_pptx_files() {
    let paths = crawl_pptx_paths(Path::new("tests/fixtures"));
    assert!(paths.iter().all(|p| {
        let n = p.file_name().unwrap().to_str().unwrap();
        n.ends_with(".pptx") && !n.starts_with("~$")
    }));
    assert!(paths.len() >= 2);
}

#[test]
fn cold_refresh_parses_fixtures() {
    let entries = refresh_index(Path::new("tests/fixtures"), "test-lib", &[]);
    let one = entries.iter().find(|e| e.number.as_deref() == Some("1")).unwrap();
    assert!(one.title.contains("Plecaţi-vă"));
    assert_eq!(one.library, "test-lib");
}


#[test]
fn cache_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("index.bin");
    let entries = vec![Hymn { mtime: 123, ..hymn("7", "T", &["line one", "line two"]) }];
    save_cache(&cache, &entries).unwrap();
    let loaded = load_cache(&cache).unwrap();
    assert_eq!(loaded, entries);
}

#[test]
fn refresh_reuses_unchanged_entries() {
    let root = std::path::Path::new("tests/fixtures");
    let first = refresh_index(root, "L", &[]);
    let again = refresh_index(root, "L", &first);
    assert_eq!(first.len(), again.len());
}

#[test]
fn cache_with_wrong_version_is_ignored() {
    // A cache written in a legacy format must not load. Otherwise a parser
    // change would keep serving stale entries because the .pptx mtimes
    // never changed.
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("index.bin");
    // Write raw entries WITHOUT the version envelope (the pre-versioning format).
    let legacy = vec![Hymn { mtime: 1, ..hymn("1", "stale", &[]) }];
    std::fs::write(&cache, bincode::serialize(&legacy).unwrap()).unwrap();
    assert!(
        load_cache(&cache).is_none(),
        "legacy/unversioned cache must be treated as a miss"
    );
}

#[test]
fn cold_refresh_preserves_crawl_order() {
    let root = Path::new("tests/fixtures");
    let paths: Vec<_> = refresh_index(root, "L", &[]).into_iter().map(|e| e.path).collect();
    assert_eq!(paths, crawl_pptx_paths(root));
}

#[test]
fn refresh_reparses_when_mtime_changed() {
    let root = Path::new("tests/fixtures");
    let mut stale = refresh_index(root, "L", &[]);
    for e in &mut stale {
        e.mtime -= 1;
        e.title = "STALE".into();
    }
    let fresh = refresh_index(root, "L", &stale);
    assert!(fresh.iter().all(|e| e.title != "STALE"));
}
