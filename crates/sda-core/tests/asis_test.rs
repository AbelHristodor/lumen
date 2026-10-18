use sda_core::asis::*;
use std::path::Path;

#[test]
fn detect_finds_soffice_on_path_var() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join(if cfg!(windows) {
        "soffice.exe"
    } else {
        "soffice"
    });
    std::fs::write(&exe, "").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    assert!(matches!(
        detect_in(dir.path().to_str().unwrap(), &[]),
        Some(Converter::LibreOffice(p)) if p == exe
    ));
    assert!(detect_in("", &[]).is_none());
}

#[test]
fn detect_prefers_candidates_that_exist() {
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("Microsoft PowerPoint.app");
    std::fs::create_dir(&app).unwrap();
    assert!(matches!(
        detect_in("", std::slice::from_ref(&app)),
        Some(Converter::PowerPoint(p)) if p == app
    ));
}

#[test]
fn cache_dir_changes_with_mtime_and_size() {
    let a = cache_dir_for(Path::new("/c"), Path::new("/h/1.pptx"), 1, (1920, 1080));
    assert_ne!(
        a,
        cache_dir_for(Path::new("/c"), Path::new("/h/1.pptx"), 2, (1920, 1080))
    );
    assert_ne!(
        a,
        cache_dir_for(Path::new("/c"), Path::new("/h/1.pptx"), 1, (1280, 720))
    );
    assert!(a.starts_with("/c"));
}

#[test]
fn cached_requires_done_marker_and_sorts_numerically() {
    let dir = tempfile::tempdir().unwrap();
    for n in [10, 2, 1] {
        std::fs::write(dir.path().join(format!("{n}.png")), "").unwrap();
    }
    assert!(cached(dir.path()).is_none());
    std::fs::write(dir.path().join(".done"), "").unwrap();
    let v = cached(dir.path()).unwrap();
    assert_eq!(
        v.iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect::<Vec<_>>(),
        ["1.png", "2.png", "10.png"]
    );
}
