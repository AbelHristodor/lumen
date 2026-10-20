//! The theme suite shipped with the app. `seed` writes it into the themes
//! dir once, as ordinary editable theme files, so a user can tune or delete
//! any of them. Photos are CC0, see `assets/backgrounds/CREDITS.md`.

use super::{
    Background, BundledFont, Easing, Font, HAlign, Prop, Rgba, Shadow, TextSize, TextStyle, Theme,
    Transition, TransitionKind, VAlign, VerseLayout,
};
use std::path::{Path, PathBuf};

/// Bump when the suite changes in a way existing users should receive.
/// Seeding never overwrites a file that exists, so edits survive.
const VERSION: &str = "2";
const MARKER: &str = ".bundled";
/// Photos live under the themes dir so one path covers the whole suite.
const BACKGROUNDS: &str = "backgrounds";

const PHOTOS: [(&str, &[u8]); 5] = [
    (
        "forest.jpg",
        include_bytes!("../../../../assets/backgrounds/forest.jpg"),
    ),
    (
        "dawn.jpg",
        include_bytes!("../../../../assets/backgrounds/dawn.jpg"),
    ),
    (
        "stone.jpg",
        include_bytes!("../../../../assets/backgrounds/stone.jpg"),
    ),
    (
        "sanctuary.jpg",
        include_bytes!("../../../../assets/backgrounds/sanctuary.jpg"),
    ),
    (
        "glass.jpg",
        include_bytes!("../../../../assets/backgrounds/glass.jpg"),
    ),
];

/// Every bundled theme in suite order. `photos` is where the images are.
pub fn all(photos: &Path) -> Vec<Theme> {
    vec![
        clean(),
        paper(),
        slate(),
        midnight(),
        ember(),
        gold(),
        ocean(),
        forest(photos),
        dawn(photos),
        stone(photos),
        aurora(),
        candlelight(),
        sanctuary(photos),
        glass(photos),
    ]
}

/// Write the suite into `themes_dir` unless the marker says it is there.
/// Existing files are never replaced. Errors are logged, never fatal.
pub fn seed(themes_dir: &Path) {
    let marker = themes_dir.join(MARKER);
    if std::fs::read_to_string(&marker).is_ok_and(|v| v.trim() == VERSION) {
        return;
    }
    let photos = themes_dir.join(BACKGROUNDS);
    if let Err(e) = std::fs::create_dir_all(&photos) {
        log::warn!("bundled themes: {}: {e}", photos.display());
        return;
    }
    for (name, bytes) in PHOTOS {
        let p = photos.join(name);
        if p.exists() {
            continue;
        }
        if let Err(e) = std::fs::write(&p, bytes) {
            log::warn!("bundled themes: {}: {e}", p.display());
        }
    }
    for t in all(&photos) {
        let p = themes_dir.join(format!("{}.json", t.name));
        if p.exists() {
            continue;
        }
        match t.to_json() {
            Ok(json) => {
                if let Err(e) = std::fs::write(&p, json) {
                    log::warn!("bundled themes: {}: {e}", p.display());
                }
            }
            Err(e) => log::warn!("bundled themes: {}: {e}", t.name),
        }
    }
    if let Err(e) = std::fs::write(&marker, VERSION) {
        log::warn!("bundled themes: {}: {e}", marker.display());
    }
}

fn rgb(hex: u32) -> Rgba {
    Rgba::new((hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255)
}

/// A soft drop shadow. Blur stays small, a wide blur renders as a haze.
fn shadow(alpha: u8) -> Option<Shadow> {
    Some(Shadow {
        color: Rgba::new(0, 0, 0, alpha),
        offset: (0.0, 2.0),
        blur: 4.0,
    })
}

/// Verse text: a font, a color, auto size, centered unless told otherwise.
fn text(font: BundledFont, color: Rgba, bold: bool) -> TextStyle {
    TextStyle {
        font: Prop::Set(Font::Bundled(font)),
        size: Prop::Set(TextSize::Auto),
        bold: Prop::Set(bold),
        color: Prop::Set(color),
        h_align: Prop::Set(HAlign::Center),
        v_align: VAlign::Middle,
        line_spacing: Prop::Set(1.2),
        margin: Prop::Inherit,
        opacity: 1.0,
        shadow: None,
        outline: None,
    }
}

/// The reference line: inherits everything, sits under the verses, a touch
/// faded so the verses stay the focus.
fn reference(color: Option<Rgba>) -> TextStyle {
    TextStyle {
        color: color.map_or(Prop::Inherit, Prop::Set),
        bold: Prop::Set(false),
        v_align: VAlign::Bottom,
        opacity: 0.85,
        ..TextStyle::default()
    }
}

fn fade(ms: u32) -> Transition {
    Transition {
        kind: TransitionKind::Fade,
        duration_ms: ms,
        easing: Easing::EaseInOut,
    }
}

fn blur(ms: u32) -> Transition {
    Transition {
        kind: TransitionKind::Blur,
        duration_ms: ms,
        easing: Easing::EaseOut,
    }
}

fn gradient(from: Rgba, to: Rgba, angle: f32, drift: f32) -> Background {
    Background::Gradient {
        stops: vec![(0.0, from), (1.0, to)],
        angle,
        drift,
    }
}

fn theme(
    name: &str,
    text: TextStyle,
    reference: TextStyle,
    background: Background,
    opacity: f32,
    transition: Transition,
) -> Theme {
    Theme {
        name: name.into(),
        text,
        reference,
        background,
        background_opacity: opacity,
        transition,
        footer: true,
        verses: VerseLayout::default(),
        favorite: false,
    }
}

/// Near black, white Inter. The sober one.
fn clean() -> Theme {
    theme(
        "Clean",
        text(BundledFont::Inter, Rgba::WHITE, false),
        reference(Some(rgb(0xB8BCC8))),
        Background::Solid(rgb(0x0B0B0F)),
        1.0,
        fade(400),
    )
}

/// Warm off white, dark ink, Lora. For bright rooms and a printed feel.
fn paper() -> Theme {
    theme(
        "Paper",
        text(BundledFont::Lora, rgb(0x2B2A28), false),
        reference(Some(rgb(0x7A6F5E))),
        Background::Solid(rgb(0xF5F1E8)),
        1.0,
        fade(350),
    )
}

/// Charcoal gradient, left aligned Inter, reference in the top right.
fn slate() -> Theme {
    let mut t = text(BundledFont::Inter, rgb(0xF2F4F8), false);
    t.h_align = Prop::Set(HAlign::Left);
    let mut r = reference(Some(rgb(0x9AA3B5)));
    r.h_align = Prop::Set(HAlign::Right);
    r.v_align = VAlign::Top;
    theme(
        "Slate",
        t,
        r,
        gradient(rgb(0x1C2128), rgb(0x30363D), 160.0, 0.0),
        1.0,
        fade(300),
    )
}

/// Navy to indigo with a slow drift, bold Inter, a soft shadow.
fn midnight() -> Theme {
    let mut t = text(BundledFont::Inter, Rgba::WHITE, true);
    t.shadow = shadow(140);
    theme(
        "Midnight",
        t,
        reference(Some(rgb(0xC7CBF5))),
        gradient(rgb(0x0B1437), rgb(0x3A2E7A), 35.0, 0.25),
        1.0,
        fade(500),
    )
}

/// Burgundy to charcoal, Noto Serif, cream text.
fn ember() -> Theme {
    theme(
        "Ember",
        text(BundledFont::NotoSerif, rgb(0xF7EBD8), false),
        reference(Some(rgb(0xD9B8A0))),
        gradient(rgb(0x4A0F1C), rgb(0x1A1416), 60.0, 0.15),
        1.0,
        fade(500),
    )
}

/// Black, Lora, gold. The elegant classic.
fn gold() -> Theme {
    let mut t = text(BundledFont::Lora, rgb(0xE8C872), false);
    t.line_spacing = Prop::Set(1.3);
    theme(
        "Gold",
        t,
        reference(Some(rgb(0xA8935B))),
        Background::Solid(Rgba::BLACK),
        1.0,
        blur(600),
    )
}

/// Drifting particles on deep teal.
fn ocean() -> Theme {
    let mut t = text(BundledFont::Inter, Rgba::WHITE, false);
    t.shadow = shadow(120);
    theme(
        "Ocean",
        t,
        reference(Some(rgb(0xA9DDE6))),
        Background::Particles {
            base: rgb(0x05242E),
            color: rgb(0x7FD1E0),
            count: 90,
            speed: 0.6,
        },
        1.0,
        fade(450),
    )
}

/// Text over a photo: white, bold, a shadow for legibility.
fn photo_text(font: BundledFont) -> TextStyle {
    let mut t = text(font, Rgba::WHITE, true);
    t.shadow = shadow(190);
    t
}

fn photo(dir: &Path, file: &str) -> Background {
    Background::Image(PathBuf::from(dir).join(file))
}

/// Misty mountains dimmed to 55 percent.
fn forest(photos: &Path) -> Theme {
    theme(
        "Forest",
        photo_text(BundledFont::Inter),
        reference(Some(rgb(0xD6E2DC))),
        photo(photos, "forest.jpg"),
        0.55,
        fade(500),
    )
}

/// Mountain sunrise, Roboto Slab, warm white.
fn dawn(photos: &Path) -> Theme {
    let mut t = photo_text(BundledFont::RobotoSlab);
    t.color = Prop::Set(rgb(0xFFF6E8));
    theme(
        "Dawn",
        t,
        reference(Some(rgb(0xF2D9B8))),
        photo(photos, "dawn.jpg"),
        0.5,
        fade(500),
    )
}

/// Stone wall, Noto Serif. Classic and quiet.
fn stone(photos: &Path) -> Theme {
    let mut t = photo_text(BundledFont::NotoSerif);
    t.bold = Prop::Set(false);
    theme(
        "Stone",
        t,
        reference(Some(rgb(0xD8D8D8))),
        photo(photos, "stone.jpg"),
        0.4,
        blur(600),
    )
}

/// Teal to violet gradient that keeps drifting. Modern and alive.
fn aurora() -> Theme {
    let mut t = text(BundledFont::Inter, Rgba::WHITE, true);
    t.shadow = shadow(120);
    theme(
        "Aurora",
        t,
        reference(Some(rgb(0xBFE9F2))),
        gradient(rgb(0x042F3A), rgb(0x3B1D6E), 120.0, 0.7),
        1.0,
        fade(500),
    )
}

/// Warm sparks rising on a dark brown base, Lora in candle white.
fn candlelight() -> Theme {
    let mut t = text(BundledFont::Lora, rgb(0xFFF3DC), false);
    t.shadow = shadow(120);
    theme(
        "Candlelight",
        t,
        reference(Some(rgb(0xD9B06A))),
        Background::Particles {
            base: rgb(0x1A0F08),
            color: rgb(0xFFB347),
            count: 60,
            speed: 0.35,
        },
        1.0,
        blur(700),
    )
}

/// A lit cathedral nave, Noto Serif. The church one.
fn sanctuary(photos: &Path) -> Theme {
    let mut t = photo_text(BundledFont::NotoSerif);
    t.bold = Prop::Set(false);
    t.color = Prop::Set(rgb(0xFFF8EC));
    theme(
        "Sanctuary",
        t,
        reference(Some(rgb(0xE6CFA3))),
        photo(photos, "sanctuary.jpg"),
        0.45,
        blur(600),
    )
}

/// Blurred stained glass, a mosaic of color behind white Inter.
fn glass(photos: &Path) -> Theme {
    theme(
        "Glass",
        photo_text(BundledFont::Inter),
        reference(Some(rgb(0xE8E0F0))),
        photo(photos, "glass.jpg"),
        0.4,
        fade(500),
    )
}
