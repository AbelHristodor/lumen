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

#[cfg(unix)]
#[test]
fn render_clears_stale_output_and_scales_to_width() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let pdf = "%PDF-1.4\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] >> endobj\ntrailer << /Root 1 0 R >>\n";
    std::fs::write(dir.path().join("fake.pdf"), pdf).unwrap();
    let exe = dir.path().join("soffice");
    std::fs::write(
        &exe,
        format!(
            "#!/bin/sh\ncp '{}' \"$5/$(basename \"$6\" .pptx).pdf\"\n",
            dir.path().join("fake.pdf").display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    std::fs::write(out.join("99.png"), "").unwrap();
    std::fs::write(out.join(".done"), "").unwrap();
    let pptx = dir.path().join("deck.pptx");
    std::fs::write(&pptx, "").unwrap();
    let v = render(&Converter::LibreOffice(exe), &pptx, &out, (400, 300)).unwrap();
    assert_eq!(v, vec![out.join("1.png")]);
    assert!(!out.join("99.png").exists());
    assert!(!out.join("deck.pdf").exists());
    let img = image::image_dimensions(&v[0]).unwrap();
    assert_eq!(img, (400, 200));
}
