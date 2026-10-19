//! Theme data model. A theme is a saved set of styling applied to projected
//! slides. Text properties can be `Inherit`, which keeps the value found in
//! the pptx, or `Set`, which replaces it. One theme is one JSON file. See
//! `store` for persistence.

use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

/// RGBA color as four bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Rgba { r, g, b, a }
    }

    pub const BLACK: Rgba = Rgba::new(0, 0, 0, 255);
    pub const WHITE: Rgba = Rgba::new(255, 255, 255, 255);

    /// Format as `#RRGGBB` in uppercase. Alpha is dropped.
    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Parse `#RRGGBB` or `RRGGBB` in any case. Alpha is set to 255.
    /// Returns `None` unless the input is exactly six hex digits.
    pub fn from_hex(s: &str) -> Option<Rgba> {
        let h = s.strip_prefix('#').unwrap_or(s);
        if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let r = u8::from_str_radix(&h[0..2], 16).ok()?;
        let g = u8::from_str_radix(&h[2..4], 16).ok()?;
        let b = u8::from_str_radix(&h[4..6], 16).ok()?;
        Some(Rgba::new(r, g, b, 255))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VAlign {
    Top,
    Middle,
    Bottom,
}

/// A theme property that either keeps the pptx value or replaces it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Prop<T> {
    #[default]
    Inherit,
    Set(T),
}

impl<T: Clone> Prop<T> {
    /// `Set` wins, then the inherited value, then the fallback.
    pub fn resolve(&self, inherited: Option<T>, fallback: T) -> T {
        match self {
            Prop::Set(v) => v.clone(),
            Prop::Inherit => inherited.unwrap_or(fallback),
        }
    }
}

/// A font family shipped with the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BundledFont {
    Inter,
    NotoSerif,
    Lora,
    RobotoSlab,
}

impl BundledFont {
    /// Every bundled font in display order.
    pub fn all() -> [BundledFont; 4] {
        [
            BundledFont::Inter,
            BundledFont::NotoSerif,
            BundledFont::Lora,
            BundledFont::RobotoSlab,
        ]
    }

    /// Human readable font name.
    pub fn label(self) -> &'static str {
        self.family()
    }

    /// Family name as registered with the text engine.
    pub fn family(self) -> &'static str {
        match self {
            BundledFont::Inter => "Inter",
            BundledFont::NotoSerif => "Noto Serif",
            BundledFont::Lora => "Lora",
            BundledFont::RobotoSlab => "Roboto Slab",
        }
    }
}

/// A bundled font or a family name found in the pptx or on the system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Font {
    Bundled(BundledFont),
    System(String),
}

impl Font {
    pub fn family(&self) -> &str {
        match self {
            Font::Bundled(f) => f.family(),
            Font::System(s) => s,
        }
    }
}

/// Slide text size. Auto fits the text to its box.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum TextSize {
    #[default]
    Auto,
    Pt(f32),
}

impl Hash for TextSize {
    fn hash<H: Hasher>(&self, h: &mut H) {
        match self {
            TextSize::Auto => 0u8.hash(h),
            TextSize::Pt(p) => {
                1u8.hash(h);
                p.to_bits().hash(h);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    pub color: Rgba,
    pub offset: (f32, f32),
    pub blur: f32,
}

impl Hash for Shadow {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.color.hash(h);
        self.offset.0.to_bits().hash(h);
        self.offset.1.to_bits().hash(h);
        self.blur.to_bits().hash(h);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Outline {
    pub color: Rgba,
    pub width: f32,
}

impl Hash for Outline {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.color.hash(h);
        self.width.to_bits().hash(h);
    }
}

/// How slide text is styled. Every `Prop` field falls back to the pptx value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextStyle {
    pub font: Prop<Font>,
    pub size: Prop<TextSize>,
    pub bold: Prop<bool>,
    pub color: Prop<Rgba>,
    pub h_align: Prop<HAlign>,
    pub v_align: VAlign,
    pub line_spacing: Prop<f32>,
    /// Margin as a fraction of the short side of the slide.
    pub margin: Prop<f32>,
    pub opacity: f32,
    pub shadow: Option<Shadow>,
    pub outline: Option<Outline>,
}

impl Default for TextStyle {
    fn default() -> Self {
        TextStyle {
            font: Prop::Inherit,
            size: Prop::Inherit,
            bold: Prop::Inherit,
            color: Prop::Inherit,
            h_align: Prop::Inherit,
            v_align: VAlign::Middle,
            line_spacing: Prop::Inherit,
            margin: Prop::Inherit,
            opacity: 1.0,
            shadow: None,
            outline: None,
        }
    }
}

fn hash_prop_f32<H: Hasher>(p: &Prop<f32>, h: &mut H) {
    match p {
        Prop::Inherit => 0u8.hash(h),
        Prop::Set(v) => {
            1u8.hash(h);
            v.to_bits().hash(h);
        }
    }
}

impl Hash for TextStyle {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.font.hash(h);
        self.size.hash(h);
        self.bold.hash(h);
        self.color.hash(h);
        self.h_align.hash(h);
        self.v_align.hash(h);
        hash_prop_f32(&self.line_spacing, h);
        hash_prop_f32(&self.margin, h);
        self.opacity.to_bits().hash(h);
        self.shadow.hash(h);
        self.outline.hash(h);
    }
}

/// What is drawn behind the text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Background {
    Solid(Rgba),
    Gradient {
        stops: Vec<(f32, Rgba)>,
        angle: f32,
        drift: f32,
    },
    Particles {
        base: Rgba,
        color: Rgba,
        count: u32,
        speed: f32,
    },
    Image(PathBuf),
    Video {
        path: PathBuf,
    },
}

impl Background {
    pub fn kind_label(&self) -> &'static str {
        match self {
            Background::Solid(_) => "Solid",
            Background::Gradient { .. } => "Gradient",
            Background::Particles { .. } => "Particles",
            Background::Image(_) => "Image",
            Background::Video { .. } => "Video",
        }
    }

    /// True when the background changes over time and needs redraws.
    pub fn is_animated(&self) -> bool {
        match self {
            Background::Gradient { drift, .. } => *drift > 0.0,
            Background::Particles { .. } | Background::Video { .. } => true,
            Background::Solid(_) | Background::Image(_) => false,
        }
    }
}

impl Hash for Background {
    fn hash<H: Hasher>(&self, h: &mut H) {
        match self {
            Background::Solid(c) => {
                0u8.hash(h);
                c.hash(h);
            }
            Background::Gradient {
                stops,
                angle,
                drift,
            } => {
                1u8.hash(h);
                for (pos, c) in stops {
                    pos.to_bits().hash(h);
                    c.hash(h);
                }
                angle.to_bits().hash(h);
                drift.to_bits().hash(h);
            }
            Background::Particles {
                base,
                color,
                count,
                speed,
            } => {
                2u8.hash(h);
                base.hash(h);
                color.hash(h);
                count.hash(h);
                speed.to_bits().hash(h);
            }
            Background::Image(p) => {
                3u8.hash(h);
                p.hash(h);
            }
            Background::Video { path } => {
                4u8.hash(h);
                path.hash(h);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransitionKind {
    None,
    Fade,
    Slide(Direction),
    Scale,
    Blur,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Easing {
    Linear,
    EaseInOut,
    EaseOut,
}

impl Easing {
    /// Map progress `t` in 0 to 1 onto the eased curve.
    pub fn apply(self, t: f32) -> f32 {
        match self {
            Easing::Linear => t,
            Easing::EaseInOut => 0.5 - 0.5 * (std::f32::consts::PI * t).cos(),
            Easing::EaseOut => 1.0 - (1.0 - t).powi(3),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Transition {
    pub kind: TransitionKind,
    pub duration_ms: u32,
    pub easing: Easing,
}

impl Default for Transition {
    fn default() -> Self {
        Transition {
            kind: TransitionKind::Fade,
            duration_ms: 400,
            easing: Easing::EaseInOut,
        }
    }
}

/// How Bible verses are split and labeled. Verses pack into a slide while
/// they fit at `min_size` points, see `bible::pack_verses`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VerseLayout {
    pub min_size: f32,
    pub show_reference: bool,
    pub show_numbers: bool,
}

impl Default for VerseLayout {
    fn default() -> Self {
        VerseLayout {
            min_size: 28.0,
            show_reference: true,
            show_numbers: true,
        }
    }
}

impl Hash for VerseLayout {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.min_size.to_bits().hash(h);
        self.show_reference.hash(h);
        self.show_numbers.hash(h);
    }
}

/// A saved set of styling applied to projected slides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, from = "ThemeV2OrV3")]
pub struct Theme {
    pub name: String,
    pub text: TextStyle,
    pub reference: TextStyle,
    pub background: Background,
    /// Blended over black. 1.0 shows the background as is.
    pub background_opacity: f32,
    pub transition: Transition,
    pub footer: bool,
    pub verses: VerseLayout,
    pub favorite: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            name: "Default".into(),
            text: TextStyle::default(),
            reference: reference_default(),
            background: Background::Solid(Rgba::BLACK),
            background_opacity: 1.0,
            transition: Transition::default(),
            footer: true,
            verses: VerseLayout::default(),
            favorite: false,
        }
    }
}

impl Hash for Theme {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.name.hash(h);
        self.text.hash(h);
        self.reference.hash(h);
        self.background.hash(h);
        self.background_opacity.to_bits().hash(h);
        self.transition.hash(h);
        self.footer.hash(h);
        self.verses.hash(h);
        self.favorite.hash(h);
    }
}

impl Theme {
    /// Serialize to pretty JSON.
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Parse JSON. Missing fields take defaults. Flat 2.0 files still load.
    pub fn from_json(text: &str) -> anyhow::Result<Theme> {
        Ok(serde_json::from_str(text)?)
    }

    /// The built-in default theme cannot be deleted or overwritten on disk.
    pub fn is_builtin_default(&self) -> bool {
        self.name == "Default"
    }

    /// Stable hash of every field, used by layout caches.
    pub fn cache_key(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut h);
        h.finish()
    }
}

/// The 3.0 file shape. `text` is required so a flat 2.0 file falls through
/// to `ThemeV2` instead of loading with every property on Inherit.
#[derive(Deserialize)]
struct ThemeV3 {
    #[serde(default = "default_name")]
    name: String,
    text: TextStyle,
    #[serde(default = "reference_default")]
    reference: TextStyle,
    #[serde(default = "default_background")]
    background: Background,
    #[serde(default = "one")]
    background_opacity: f32,
    #[serde(default)]
    transition: Transition,
    #[serde(default = "yes")]
    footer: bool,
    #[serde(default)]
    verses: VerseLayout,
    #[serde(default)]
    favorite: bool,
}

/// The reference sits under the verses unless a theme moves it.
fn reference_default() -> TextStyle {
    TextStyle {
        v_align: VAlign::Bottom,
        ..TextStyle::default()
    }
}

fn default_name() -> String {
    "Default".into()
}
fn default_background() -> Background {
    Background::Solid(Rgba::BLACK)
}
fn one() -> f32 {
    1.0
}
fn yes() -> bool {
    true
}

fn footer_compat<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum F {
        Flag(bool),
        Old { show: bool },
    }
    Ok(match F::deserialize(d)? {
        F::Flag(b) => b,
        F::Old { show } => show,
    })
}

/// The flat 2.0 file shape. Every value becomes `Set` in the new theme.
#[derive(Deserialize)]
#[serde(default)]
struct ThemeV2 {
    name: String,
    font: BundledFont,
    size: TextSize,
    bold: bool,
    text_color: Rgba,
    bg_color: Rgba,
    h_align: HAlign,
    v_align: VAlign,
    line_spacing: f32,
    margin: f32,
    #[serde(deserialize_with = "footer_compat")]
    footer: bool,
}

impl Default for ThemeV2 {
    fn default() -> Self {
        ThemeV2 {
            name: "Default".into(),
            font: BundledFont::Inter,
            size: TextSize::Auto,
            bold: true,
            text_color: Rgba::WHITE,
            bg_color: Rgba::new(11, 31, 58, 255),
            h_align: HAlign::Center,
            v_align: VAlign::Middle,
            line_spacing: 1.3,
            margin: 0.06,
            footer: false,
        }
    }
}

// Lives only while one file parses, so the size gap between variants is fine.
#[allow(clippy::large_enum_variant)]
#[derive(Deserialize)]
#[serde(untagged)]
enum ThemeV2OrV3 {
    V3(ThemeV3),
    V2(ThemeV2),
}

impl From<ThemeV2OrV3> for Theme {
    fn from(v: ThemeV2OrV3) -> Self {
        match v {
            ThemeV2OrV3::V3(t) => Theme {
                name: t.name,
                text: t.text,
                reference: t.reference,
                background: t.background,
                background_opacity: t.background_opacity,
                transition: t.transition,
                footer: t.footer,
                verses: t.verses,
                favorite: t.favorite,
            },
            ThemeV2OrV3::V2(t) => Theme {
                name: t.name,
                text: TextStyle {
                    font: Prop::Set(Font::Bundled(t.font)),
                    size: Prop::Set(t.size),
                    bold: Prop::Set(t.bold),
                    color: Prop::Set(t.text_color),
                    h_align: Prop::Set(t.h_align),
                    v_align: t.v_align,
                    line_spacing: Prop::Set(t.line_spacing),
                    margin: Prop::Set(t.margin),
                    ..TextStyle::default()
                },
                background: Background::Solid(t.bg_color),
                footer: t.footer,
                reference: reference_default(),
                ..Theme::default()
            },
        }
    }
}

/// Named file persistence for themes. One `<name>.json` per theme in a
/// directory. The built-in `Default` is always present and never written or
/// deleted on disk.
pub mod store {
    use super::Theme;
    use anyhow::{bail, Result};
    use std::path::Path;

    /// Sanitize a theme name into a safe file stem. Keeps alphanumerics,
    /// space, hyphen, and underscore.
    fn file_stem(name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    /// List all themes. The built-in default first, then every valid
    /// `<dir>/*.json`. Corrupt files are skipped and logged.
    pub fn list_themes(dir: &Path) -> Vec<Theme> {
        let mut out = vec![Theme::default()];
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                match std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|s| Theme::from_json(&s).ok())
                {
                    Some(t) if !t.is_builtin_default() => out.push(t),
                    Some(_) => {}
                    None => log::warn!("skip corrupt theme {}", path.display()),
                }
            }
        }
        out
    }

    /// Load one theme by name. Returns the built-in default for "Default".
    pub fn load_theme(dir: &Path, name: &str) -> Result<Theme> {
        if name == "Default" {
            return Ok(Theme::default());
        }
        let path = dir.join(format!("{}.json", file_stem(name)));
        let text = std::fs::read_to_string(&path)?;
        Theme::from_json(&text)
    }

    /// Save a theme to `<dir>/<name>.json`. Refuses the reserved default name.
    pub fn save_theme(dir: &Path, theme: &Theme) -> Result<()> {
        if theme.is_builtin_default() {
            bail!("the built in Default theme cannot be saved over");
        }
        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("{}.json", file_stem(&theme.name)));
        std::fs::write(path, theme.to_json()?)?;
        Ok(())
    }

    /// Delete a theme file. Refuses to delete the built in default.
    pub fn delete_theme(dir: &Path, name: &str) -> Result<()> {
        if name == "Default" {
            bail!("the built in Default theme cannot be deleted");
        }
        let path = dir.join(format!("{}.json", file_stem(name)));
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}
