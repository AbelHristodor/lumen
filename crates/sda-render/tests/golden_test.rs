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

use sda_core::theme::{Direction, Easing, Transition, TransitionKind};

fn two(kind: TransitionKind) -> Scene {
    let mut s = base();
    s.current = Some(Layer::Text(styled(
        "B",
        HAlign::Center,
        TextSize::Pt(80.0),
        None,
    )));
    s.outgoing = Some((
        Layer::Text(styled("A", HAlign::Center, TextSize::Pt(80.0), None)),
        0.5,
    ));
    s.transition = Transition {
        kind,
        duration_ms: 400,
        easing: Easing::Linear,
    };
    s
}

#[test]
fn fade_half() {
    check("fade_half", &two(TransitionKind::Fade));
}

#[test]
fn slide_left_half() {
    check(
        "slide_left_half",
        &two(TransitionKind::Slide(Direction::Left)),
    );
}

#[test]
fn scale_half() {
    check("scale_half", &two(TransitionKind::Scale));
}

#[test]
fn blur_half() {
    check("blur_half", &two(TransitionKind::Blur));
}

#[test]
fn blank_hides_text() {
    let mut s = two(TransitionKind::Fade);
    s.blank = true;
    let Some(img) = headless::render_to_image(&s) else {
        return;
    };
    assert!(img.pixels().all(|p| p[0] < 3));
    check("blank", &s);
}

#[test]
fn bitmap_layer_covers_output() {
    let mut bmp = image::RgbaImage::new(64, 36);
    for p in bmp.pixels_mut() {
        *p = image::Rgba([0, 200, 0, 255]);
    }
    let mut s = base();
    s.current = Some(Layer::Bitmap(Arc::new(bmp)));
    let Some(img) = headless::render_to_image(&s) else {
        return;
    };
    assert!(img.get_pixel(160, 90)[1] > 180);
    check("bitmap", &s);
}

#[test]
fn image_background() {
    let mut bmp = image::RgbaImage::new(8, 8);
    for (x, _, p) in bmp.enumerate_pixels_mut() {
        *p = image::Rgba([x as u8 * 30, 0, 0, 255]);
    }
    let mut s = base();
    s.background = BackgroundState::Image(Arc::new(bmp));
    check("image_bg", &s);
}
