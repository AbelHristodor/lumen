use sda_core::library::{default_library, Config, Library};
use sda_core::refresh::{force_clean_paths, register_default_library};
use std::fs;

#[test]
fn force_clean_removes_library_stamp_and_cache() {
    let tmp = tempfile::tempdir().unwrap();
    let hymns = tmp.path().join("hymns");
    let stamp = tmp.path().join("hymns.etag");
    let cache = tmp.path().join("index.bin");
    fs::create_dir_all(&hymns).unwrap();
    fs::write(hymns.join("a.pptx"), b"x").unwrap();
    fs::write(&stamp, b"etag").unwrap();
    fs::write(&cache, b"cached").unwrap();

    force_clean_paths(Some(&hymns), Some(&stamp), Some(&cache)).unwrap();

    assert!(!hymns.exists());
    assert!(!stamp.exists());
    assert!(!cache.exists());
}

#[test]
fn force_clean_is_ok_when_absent() {
    let tmp = tempfile::tempdir().unwrap();
    force_clean_paths(
        Some(&tmp.path().join("nope")),
        Some(&tmp.path().join("nope.etag")),
        Some(&tmp.path().join("nope.bin")),
    )
    .unwrap();
    force_clean_paths(None, None, None).unwrap();
}

#[test]
fn register_adds_default_when_missing() {
    let mut cfg = Config::default();
    assert!(register_default_library(&mut cfg));
    assert_eq!(cfg.libraries.len(), 1);
    assert_eq!(cfg.libraries[0], default_library().unwrap());
}

#[test]
fn register_repairs_legacy_git_clone_path() {
    let mut cfg = Config {
        libraries: vec![Library {
            name: "Imnuri Creștine".into(),
            path: "/old/default-library/assets/920".into(),
            enabled: false,
            managed_by_git: true,
        }],
        ..Config::default()
    };
    register_default_library(&mut cfg);
    assert_eq!(cfg.libraries.len(), 1);
    assert_eq!(cfg.libraries[0].path, default_library().unwrap().path);
    assert!(!cfg.libraries[0].enabled, "user's enabled choice survives");
}
