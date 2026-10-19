//! Themes tab messages. Owned by the Themes tab task.

use sda_core::theme::{Easing, Font, HAlign, Rgba, TransitionKind, VAlign};
use std::path::PathBuf;

/// Which background shape the segmented control picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgKind {
    Solid,
    Gradient,
    Particles,
    Image,
    Video,
}

/// One color control in the form. The hex input for each one is tracked so
/// a half typed value does not snap back to the last valid color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorField {
    Text,
    Shadow,
    Outline,
    BgSolid,
    BgStop(usize),
    BgParticleColor,
}

impl ColorField {
    /// The draft edit that applies a parsed color to this field.
    pub fn edit(self, c: Rgba) -> DraftEdit {
        match self {
            ColorField::Text => DraftEdit::Color(c),
            ColorField::Shadow => DraftEdit::ShadowColor(c),
            ColorField::Outline => DraftEdit::OutlineColor(c),
            ColorField::BgSolid => DraftEdit::BgSolid(c),
            ColorField::BgStop(i) => DraftEdit::BgStop(i, c),
            ColorField::BgParticleColor => DraftEdit::BgParticleColor(c),
        }
    }
}

/// One form control changed on the draft theme.
#[derive(Debug, Clone, PartialEq)]
pub enum DraftEdit {
    Name(String),
    FontInherit(bool),
    Font(Font),
    SizeInherit(bool),
    SizeAuto(bool),
    SizePt(f32),
    BoldInherit(bool),
    Bold(bool),
    ColorInherit(bool),
    Color(Rgba),
    HAlignInherit(bool),
    HAlign(HAlign),
    VAlign(VAlign),
    SpacingInherit(bool),
    Spacing(f32),
    MarginInherit(bool),
    Margin(f32),
    Opacity(f32),
    ShadowOn(bool),
    ShadowColor(Rgba),
    ShadowOffset(f32),
    OutlineOn(bool),
    OutlineColor(Rgba),
    OutlineWidth(f32),
    BgKind(BgKind),
    BgSolid(Rgba),
    BgStop(usize, Rgba),
    BgAngle(f32),
    BgDrift(f32),
    BgParticleColor(Rgba),
    BgParticleCount(u32),
    BgParticleSpeed(f32),
    BgOpacity(f32),
    TransitionKind(TransitionKind),
    Duration(u32),
    Easing(Easing),
    MinSize(f32),
    ShowReference(bool),
    ShowNumbers(bool),
    Footer(bool),
}

#[derive(Debug, Clone)]
pub enum ThemesMsg {
    Noop,
    /// Load the theme at this position of the sorted list into the form.
    Select(usize),
    New,
    Duplicate,
    Delete,
    Save,
    /// Make the named theme the one used for projection.
    SetActive(String),
    Favorite(bool),
    Draft(DraftEdit),
    /// Raw text typed into a color field. Applied once it parses.
    Hex(ColorField, String),
    PickImage,
    PickVideo,
    FilePicked(Option<PathBuf>),
    /// One step of the preview loop.
    Tick,
}
