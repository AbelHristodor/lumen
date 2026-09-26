//! Theme data model: a saved set of styling applied to projected slides.
//! Serialized one-theme-per-JSON-file; see `theme_store` for persistence.

use serde::{Deserialize, Serialize};

/// RGBA colour as 4 bytes; serialized as a "#rrggbbaa" string is overkill —
/// keep it as a struct for round-trip simplicity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Format as `#RRGGBB` (uppercase, alpha dropped — the editor only edits RGB).
    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Parse `#RRGGBB` / `RRGGBB` (case-insensitive). Alpha is set to 255.
    /// Returns `None` for any non-6-hex-digit input.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VAlign {
    Top,
    Middle,
    Bottom,
}

/// A bundled font family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Font {
    Inter,
    NotoSerif,
    Lora,
    RobotoSlab,
}

impl Font {
    /// Every bundled font in display order.
    pub fn all() -> [Font; 4] {
        [Font::Inter, Font::NotoSerif, Font::Lora, Font::RobotoSlab]
    }

    /// Human readable font name.
    pub fn label(self) -> &'static str {
        match self {
            Font::Inter => "Inter",
            Font::NotoSerif => "Noto Serif",
            Font::Lora => "Lora",
            Font::RobotoSlab => "Roboto Slab",
        }
    }
}

/// Slide text size. Auto fits the text to the slide.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum FontSize {
    #[default]
    Auto,
    Pt(f32),
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

/// A saved set of styling applied to projected slides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub name: String,
    pub font: Font,
    pub size: FontSize,
    pub bold: bool,
    pub text_color: Rgba,
    pub bg_color: Rgba,
    pub h_align: HAlign,
    pub v_align: VAlign,
    pub line_spacing: f32,
    pub margin: f32,
    #[serde(deserialize_with = "footer_compat")]
    pub footer: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            name: "Default".into(),
            font: Font::Inter,
            size: FontSize::Auto,
            bold: true,
            text_color: Rgba::new(255, 255, 255, 255),
            bg_color: Rgba::new(11, 31, 58, 255),
            h_align: HAlign::Center,
            v_align: VAlign::Middle,
            line_spacing: 1.3,
            margin: 0.06,
            footer: false,
        }
    }
}

impl Theme {
    /// Serialize to pretty JSON.
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Parse JSON. Missing fields take defaults and old nested files still load.
    pub fn from_json(text: &str) -> anyhow::Result<Theme> {
        Ok(serde_json::from_str(text)?)
    }

    /// The built-in default theme cannot be deleted or overwritten on disk.
    pub fn is_builtin_default(&self) -> bool {
        self.name == "Default"
    }

    /// Stable hash of every field, used by the GUI layout cache.
    pub fn cache_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.name.hash(&mut h);
        self.font.hash(&mut h);
        self.bold.hash(&mut h);
        match self.size {
            FontSize::Auto => 0u32.hash(&mut h),
            FontSize::Pt(p) => p.to_bits().hash(&mut h),
        }
        let c = self.text_color;
        (c.r, c.g, c.b, c.a).hash(&mut h);
        let c = self.bg_color;
        (c.r, c.g, c.b, c.a).hash(&mut h);
        (self.h_align as u8, self.v_align as u8).hash(&mut h);
        self.line_spacing.to_bits().hash(&mut h);
        self.margin.to_bits().hash(&mut h);
        self.footer.hash(&mut h);
        h.finish()
    }
}

/// Named-file persistence for themes. One `<name>.json` per theme in a
/// directory; the built-in `Default` is always present and never written or
/// deleted on disk.
pub mod store {
    use super::Theme;
    use anyhow::{bail, Result};
    use std::path::Path;

    /// Sanitize a theme name into a safe file stem (alphanumerics, space, -, _).
    fn file_stem(name: &str) -> String {
        name.chars()
            .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
            .collect()
    }

    /// List all themes: the built-in default first, then every valid
    /// `<dir>/*.json`. Corrupt files are skipped (logged via eprintln).
    pub fn list_themes(dir: &Path) -> Vec<Theme> {
        let mut out = vec![Theme::default()];
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                match std::fs::read_to_string(&path).ok().and_then(|s| Theme::from_json(&s).ok()) {
                    Some(t) if !t.is_builtin_default() => out.push(t),
                    Some(_) => {}
                    None => eprintln!("skip corrupt theme {}", path.display()),
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

    /// Save a theme to `<dir>/<name>.json`. Refuses to overwrite the built-in
    /// default's reserved name.
    pub fn save_theme(dir: &Path, theme: &Theme) -> Result<()> {
        if theme.is_builtin_default() {
            bail!("the built-in Default theme cannot be saved over");
        }
        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("{}.json", file_stem(&theme.name)));
        std::fs::write(path, theme.to_json()?)?;
        Ok(())
    }

    /// Delete a theme file. Refuses to delete the built-in default.
    pub fn delete_theme(dir: &Path, name: &str) -> Result<()> {
        if name == "Default" {
            bail!("the built-in Default theme cannot be deleted");
        }
        let path = dir.join(format!("{}.json", file_stem(name)));
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}
