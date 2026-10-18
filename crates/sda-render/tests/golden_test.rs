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

use sda_core::model::Frame;
use sda_core::resolve::{ResolvedBox, ResolvedLine, ResolvedRun, ResolvedSlide};
use sda_core::theme::{BundledFont, Font, HAlign, Rgba, Shadow, TextSize, VAlign};
use std::sync::Arc;

fn styled(text: &str, align: HAlign, size: TextSize, shadow: Option<Shadow>) -> Arc<ResolvedSlide> {
    Arc::new(ResolvedSlide {
        background: Rgba::new(0, 0, 0, 255),
        footer: None,
        boxes: vec![ResolvedBox {
            frame: Frame {
                x: 0.06,
                y: 0.06,
                w: 0.88,
                h: 0.88,
            },
            v_align: VAlign::Middle,
            lines: text
                .lines()
                .map(|l| ResolvedLine {
                    align,
                    spacing: 1.1,
                    runs: vec![ResolvedRun {
                        text: l.into(),
                        font: Font::Bundled(BundledFont::Inter),
                        size,
                        bold: false,
                        italic: false,
                        color: Rgba::new(255, 255, 255, 255),
                        opacity: 1.0,
                        shadow,
                        outline: None,
                    }],
                })
                .collect(),
        }],
    })
}

#[test]
fn text_center_auto() {
    let mut s = base();
    s.current = Some(Layer::Text(styled(
        "Cerul, pământul\nse vor topi",
        HAlign::Center,
        TextSize::Auto,
        None,
    )));
    check("text_center_auto", &s);
}

#[test]
fn text_left_pt_shadow() {
    let mut s = base();
    s.current = Some(Layer::Text(styled(
        "Plecaţi-vă",
        HAlign::Left,
        TextSize::Pt(24.0),
        Some(Shadow {
            color: Rgba::new(0, 0, 0, 255),
            offset: (2.0, 2.0),
            blur: 0.0,
        }),
    )));
    check("text_left_pt_shadow", &s);
}

#[test]
fn text_footer() {
    let mut s = base();
    let mut r = (*styled("A", HAlign::Center, TextSize::Pt(30.0), None)).clone();
    r.footer = Some("150 · Cerul".into());
    s.current = Some(Layer::Text(Arc::new(r)));
    check("text_footer", &s);
}

#[test]
fn auto_text_fills_box_without_overflow() {
    let mut s = base();
    s.current = Some(Layer::Text(styled(
        "Short",
        HAlign::Center,
        TextSize::Auto,
        None,
    )));
    let Some(img) = headless::render_to_image(&s) else {
        return;
    };
    let white_rows: Vec<u32> = (0..180)
        .filter(|y| (0..320).any(|x| img.get_pixel(x, *y)[0] > 200))
        .collect();
    assert!(
        white_rows.len() > 60,
        "auto size should be large, got {} rows",
        white_rows.len()
    );
    assert!(*white_rows.first().unwrap() >= 10 && *white_rows.last().unwrap() <= 170);
}
