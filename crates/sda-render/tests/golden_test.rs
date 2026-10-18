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

#[test]
fn gradient_drifts_with_time() {
    let g = BackgroundState::Gradient {
        stops: vec![
            (0.0, Rgba::new(20, 0, 60, 255)),
            (1.0, Rgba::new(0, 120, 200, 255)),
        ],
        angle: 30.0,
        drift: 0.3,
    };
    let mut a = base();
    a.background = g.clone();
    check("gradient_t0", &a);
    let mut b = a.clone();
    b.now_secs = 2.0;
    check("gradient_t2", &b);
    let (Some(ia), Some(ib)) = (headless::render_to_image(&a), headless::render_to_image(&b))
    else {
        return;
    };
    assert_ne!(ia.get_pixel(50, 50), ib.get_pixel(50, 50));
}

#[test]
fn particles_draw_bright_points() {
    let mut s = base();
    s.background = BackgroundState::Particles {
        base: Rgba::new(0, 0, 20, 255),
        color: Rgba::new(255, 255, 255, 255),
        count: 120,
        speed: 0.4,
    };
    let Some(img) = headless::render_to_image(&s) else {
        return;
    };
    assert!(img.pixels().filter(|p| p[0] > 180).count() > 50);
    check("particles_t0", &s);
}

#[test]
fn decoder_fails_cleanly_without_ffmpeg() {
    assert!(
        spawn_decoder(
            std::path::Path::new("/nonexistent/ffmpeg"),
            std::path::Path::new("/x.mp4"),
            (64, 36)
        )
        .is_err()
    );
}

#[test]
fn verses() {
    let slide = sda_core::resolve::resolve(
        &sda_core::theme::Theme::default(),
        &sda_core::resolve::SlideContent::Verses {
            reference: "John 3:16".into(),
            verses: vec![
                (16, "For God so loved the world".into()),
                (17, "that he gave".into()),
            ],
        },
        None,
    )
    .unwrap();
    let mut s = base();
    s.current = Some(Layer::Text(Arc::new(slide)));
    check("verses", &s);
}

#[test]
fn many_scenes_load_the_target() {
    let mut red = base();
    red.background = BackgroundState::Solid(Rgba::new(255, 0, 0, 255));
    let mut green = base();
    green.background = BackgroundState::Solid(Rgba::new(0, 255, 0, 255));
    let blue = wgpu::Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    let scenes = [
        (
            red,
            Viewport {
                x: 0,
                y: 0,
                w: 100,
                h: 100,
            },
        ),
        (
            green,
            Viewport {
                x: 200,
                y: 80,
                w: 100,
                h: 100,
            },
        ),
    ];
    let Some(img) = headless::render_many((320, 180), blue, &scenes) else {
        return;
    };
    assert_eq!(img.get_pixel(50, 50).0, [255, 0, 0, 255]);
    assert_eq!(img.get_pixel(99, 99).0, [255, 0, 0, 255]);
    assert_eq!(img.get_pixel(250, 130).0, [0, 255, 0, 255]);
    assert_eq!(img.get_pixel(150, 20).0, [0, 0, 255, 255]);
    assert_eq!(img.get_pixel(100, 100).0, [0, 0, 255, 255]);
}

/// Columns holding at least one bright pixel, as a width.
fn bright_width(img: &image::RgbaImage) -> u32 {
    let cols: Vec<u32> = (0..img.width())
        .filter(|x| (0..img.height()).any(|y| img.get_pixel(*x, y)[0] > 200))
        .collect();
    match (cols.first(), cols.last()) {
        (Some(a), Some(b)) => b - a + 1,
        _ => 0,
    }
}

#[test]
fn text_pt_scales_with_width() {
    let mut small = base();
    small.current = Some(Layer::Text(styled(
        "Plecaţi-vă",
        HAlign::Center,
        TextSize::Pt(40.0),
        None,
    )));
    let mut large = small.clone();
    large.size = (640, 360);
    let (Some(a), Some(b)) = (
        headless::render_to_image(&small),
        headless::render_to_image(&large),
    ) else {
        return;
    };
    let (wa, wb) = (bright_width(&a) as f32, bright_width(&b) as f32);
    assert!(wa > 20.0, "small text too narrow: {wa}");
    let ratio = wb / wa;
    assert!((1.8..=2.2).contains(&ratio), "ratio {ratio}, {wa} vs {wb}");
}

#[test]
fn scissor_limits_the_draw_but_not_the_mapping() {
    // Left half red, right half green, same aspect as the viewport. With a
    // squeeze the green half would land inside the scissor.
    let mut bmp = image::RgbaImage::new(200, 100);
    for (x, _, p) in bmp.enumerate_pixels_mut() {
        *p = if x < 100 {
            image::Rgba([255, 0, 0, 255])
        } else {
            image::Rgba([0, 255, 0, 255])
        };
    }
    let mut red = base();
    red.current = Some(Layer::Bitmap(Arc::new(bmp)));
    let blue = wgpu::Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    let viewport = Viewport {
        x: 40,
        y: 20,
        w: 200,
        h: 100,
    };
    let left_half = Viewport {
        x: 40,
        y: 20,
        w: 100,
        h: 100,
    };
    let Some(img) = headless::render_clipped((320, 180), blue, &red, viewport, left_half) else {
        return;
    };
    assert_eq!(img.get_pixel(50, 60).0, [255, 0, 0, 255]);
    assert_eq!(img.get_pixel(135, 60).0, [255, 0, 0, 255]);
    assert_eq!(img.get_pixel(140, 60).0, [0, 0, 255, 255]);
    assert_eq!(img.get_pixel(230, 60).0, [0, 0, 255, 255]);
    assert_eq!(img.get_pixel(10, 10).0, [0, 0, 255, 255]);
}

/// Bounding box of bright pixels as (left, top, right, bottom), inclusive.
fn bright_bbox(img: &image::RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let mut b: Option<(u32, u32, u32, u32)> = None;
    for (x, y, p) in img.enumerate_pixels() {
        if p[0] > 200 {
            b = Some(match b {
                None => (x, y, x, y),
                Some((l, t, r, d)) => (l.min(x), t.min(y), r.max(x), d.max(y)),
            });
        }
    }
    b
}

fn tall_pt_scene() -> Scene {
    let mut s = base();
    s.current = Some(Layer::Text(styled(
        "Cerul şi pământul\nse vor topi\nca fumul",
        HAlign::Center,
        TextSize::Pt(120.0),
        None,
    )));
    s
}

#[test]
fn text_pt_shrinks_to_fit() {
    let s = tall_pt_scene();
    let Some(img) = headless::render_to_image(&s) else {
        return;
    };
    let (l, t, r, d) = bright_bbox(&img).expect("text drawn");
    let (bl, bt) = ((0.06 * 320.0) as u32, (0.06 * 180.0) as u32);
    let (br, bd) = ((0.94 * 320.0) as u32, (0.94 * 180.0) as u32);
    assert!(l + 2 >= bl && r <= br + 2, "x span {l}..{r} outside box {bl}..{br}");
    assert!(t + 2 >= bt && d <= bd + 2, "y span {t}..{d} outside box {bt}..{bd}");
    assert!(t > bt, "first row {t} must sit below the box top {bt}");
    check("text_pt_shrinks_to_fit", &s);
}

#[test]
fn text_pt_keeps_full_size_on_projector() {
    let small = tall_pt_scene();
    let mut large = small.clone();
    large.size = (1920, 1080);
    let (Some(a), Some(b)) = (
        headless::render_to_image(&small),
        headless::render_to_image(&large),
    ) else {
        return;
    };
    let ha = bright_bbox(&a).map_or(0, |(_, t, _, d)| d - t + 1) as f32;
    let hb = bright_bbox(&b).map_or(0, |(_, t, _, d)| d - t + 1) as f32;
    assert!(ha > 0.0, "small render drew nothing");
    let ratio = hb / ha;
    assert!(ratio > 4.0, "ratio {ratio}, {ha} vs {hb}");
}
