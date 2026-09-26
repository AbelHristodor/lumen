use sda_core::index::{crawl_pptx_paths, refresh_index};
use std::path::Path;

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

use sda_core::index::{load_cache, save_cache};
use sda_core::model::HymnEntry;
use std::path::PathBuf;

#[test]
fn cache_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("index.bin");
    let entries = vec![HymnEntry {
        number: Some("7".into()),
        title: "T".into(),
        slides: vec![],
        path: PathBuf::from("/x/7.pptx"),
        library: "L".into(),
        mtime: 123,
    }];
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
    // A cache written in a legacy/foreign format must not load — otherwise a
    // parser change (e.g. better title extraction) would keep serving stale
    // entries because the .pptx mtimes never changed.
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("index.bin");
    // Write raw entries WITHOUT the version envelope (the pre-versioning format).
    let legacy = vec![HymnEntry {
        number: Some("1".into()),
        title: "stale".into(),
        slides: vec![],
        path: PathBuf::from("/x/1.pptx"),
        library: "L".into(),
        mtime: 1,
    }];
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
