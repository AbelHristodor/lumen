use sda_render::*;
use std::path::Path;

fn check(name: &str, scene: &Scene) {
    let Some(img) = headless::render_to_image(scene) else {
        eprintln!("no GPU adapter, skipping {name}");
        return;
    };
    let path = Path::new("tests/golden").join(format!("{name}.png"));
    if std::env::var("UPDATE_GOLDENS").is_ok() || !path.exists() {
        img.save(&path).unwrap();
        return;
    }
    let want = image::open(&path).unwrap().to_rgba8();
    assert_eq!(want.dimensions(), img.dimensions(), "{name} size");
    let bad = want
        .pixels()
        .zip(img.pixels())
        .filter(|(a, b)| a.0.iter().zip(b.0.iter()).any(|(x, y)| x.abs_diff(*y) > 2))
        .count();
    let budget = (want.width() * want.height()) as usize / 1000;
    assert!(
        bad <= budget,
        "{name}: {bad} pixels differ, budget {budget}"
    );
}

fn base() -> Scene {
    Scene {
        size: (320, 180),
        background: BackgroundState::Solid(sda_core::theme::Rgba::new(0, 0, 0, 255)),
        background_opacity: 1.0,
        current: None,
        outgoing: None,
        transition: sda_core::theme::Transition::default(),
        blank: false,
        now_secs: 0.0,
    }
}

#[test]
fn solid_black() {
    check("solid_black", &base());
}

#[test]
fn solid_half_opacity_red() {
    let mut s = base();
    s.background = BackgroundState::Solid(sda_core::theme::Rgba::new(255, 0, 0, 255));
    s.background_opacity = 0.5;
    let img = headless::render_to_image(&s);
    let Some(img) = img else { return };
    let p = img.get_pixel(10, 10);
    assert!(p[0] > 120 && p[0] < 136 && p[1] == 0, "{p:?}");
    check("solid_half_red", &s);
}
