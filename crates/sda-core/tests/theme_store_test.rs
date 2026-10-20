use sda_core::theme::store::{delete_theme, list_themes, load_theme, save_theme};
use sda_core::theme::{bundled, store, Background, Theme};

#[test]
fn save_then_list_and_load_includes_default_and_custom() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let names: Vec<String> = list_themes(root).iter().map(|t| t.name.clone()).collect();
    assert!(names.contains(&"Default".to_string()));
    let t = Theme {
        name: "Christmas".into(),
        ..Theme::default()
    };
    save_theme(root, &t).unwrap();
    let names: Vec<String> = list_themes(root).iter().map(|t| t.name.clone()).collect();
    assert!(names.contains(&"Christmas".to_string()));
    assert!(names.contains(&"Default".to_string()));
    let loaded = load_theme(root, "Christmas").unwrap();
    assert_eq!(loaded.name, "Christmas");
}

#[test]
fn corrupt_theme_file_is_skipped_default_remains() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(root.join("broken.json"), b"{ not valid json").unwrap();
    let names: Vec<String> = list_themes(root).iter().map(|t| t.name.clone()).collect();
    assert!(names.contains(&"Default".to_string()));
    assert!(!names.contains(&"broken".to_string()));
}

#[test]
fn cannot_delete_builtin_default() {
    let dir = tempfile::tempdir().unwrap();
    let err = delete_theme(dir.path(), "Default").unwrap_err().to_string();
    assert!(err.contains("built in"), "{err}");
    assert!(!err.contains('-'), "{err}");
    let err = save_theme(dir.path(), &sda_core::theme::Theme::default())
        .unwrap_err()
        .to_string();
    assert!(!err.contains('-'), "{err}");
}

#[test]
fn delete_removes_custom_theme() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let t = Theme {
        name: "Temp".into(),
        ..Theme::default()
    };
    save_theme(root, &t).unwrap();
    delete_theme(root, "Temp").unwrap();
    let names: Vec<String> = list_themes(root).iter().map(|t| t.name.clone()).collect();
    assert!(!names.contains(&"Temp".to_string()));
}

#[test]
fn bundled_seed_writes_the_suite_once_and_keeps_user_edits() {
    let dir = tempfile::tempdir().unwrap();
    let themes = dir.path().join("themes");
    bundled::seed(&themes);
    let listed = store::list_themes(&themes);
    // Default plus the fourteen bundled ones.
    assert_eq!(
        listed.len(),
        15,
        "{:?}",
        listed.iter().map(|t| &t.name).collect::<Vec<_>>()
    );
    for t in &listed {
        if let Background::Image(p) = &t.background {
            assert!(p.exists(), "{} photo missing: {}", t.name, p.display());
        }
    }
    assert!(themes.join(".bundled").exists());
    let names: Vec<&str> = listed.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names[0], "Default");
    assert!(names[1..].windows(2).all(|w| w[0] < w[1]), "{names:?}");

    // A deleted theme stays deleted and an edit survives a second seed.
    store::delete_theme(&themes, "Gold").unwrap();
    let mut paper = store::load_theme(&themes, "Paper").unwrap();
    paper.footer = false;
    store::save_theme(&themes, &paper).unwrap();
    bundled::seed(&themes);
    let names: Vec<String> = store::list_themes(&themes)
        .into_iter()
        .map(|t| t.name)
        .collect();
    assert!(!names.iter().any(|n| n == "Gold"));
    assert!(!store::load_theme(&themes, "Paper").unwrap().footer);
}

#[test]
fn bundled_themes_round_trip_through_json() {
    for t in bundled::all(std::path::Path::new("/photos")) {
        let back = Theme::from_json(&t.to_json().unwrap()).unwrap();
        assert_eq!(back, t, "{}", t.name);
        assert!(!t.is_builtin_default());
    }
}
