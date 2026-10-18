use crate::theme::{HAlign, Rgba};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// One hymn extracted from a .pptx file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hymn {
    /// Hymn number parsed from the filename stem, like "150" or "664b".
    /// `None` if the stem has no number.
    pub number: Option<String>,
    /// Title line, the first meaningful text on the title slide.
    pub title: String,
    /// Styled slides in presentation order.
    pub slides: Vec<Slide>,
    /// Absolute path to the source .pptx.
    pub path: PathBuf,
    /// Name of the library this hymn belongs to.
    pub library: String,
    /// File modification time in unix seconds, used for cache invalidation.
    pub mtime: i64,
}

/// One slide: its background color, text boxes, and speaker notes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Slide {
    pub background: Option<Rgba>,
    pub boxes: Vec<TextBox>,
    pub notes: Option<String>,
}

/// A text box placed on the slide with its paragraphs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextBox {
    pub frame: Option<Frame>,
    pub paragraphs: Vec<Paragraph>,
}

/// Box position and size as fractions of the slide size, 0 to 1.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct Frame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// One paragraph of runs with its own alignment and line spacing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Paragraph {
    pub align: Option<HAlign>,
    /// Line spacing as a multiplier, 1.0 is single spacing.
    pub line_spacing: Option<f32>,
    pub runs: Vec<Run>,
}

/// A span of text with one style. `None` means not set in the file.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Run {
    pub text: String,
    pub color: Option<Rgba>,
    pub size_pt: Option<f32>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub font: Option<String>,
}

impl Slide {
    /// Text of every paragraph joined by a newline. Runs are concatenated
    /// and each line is trimmed. Empty lines are dropped.
    pub fn plain_text(&self) -> String {
        self.boxes
            .iter()
            .flat_map(|b| b.paragraphs.iter())
            .map(|p| {
                p.runs
                    .iter()
                    .map(|r| r.text.as_str())
                    .collect::<String>()
                    .trim()
                    .to_string()
            })
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Hymn {
    /// Plain text of each slide, in order.
    pub fn plain_slides(&self) -> Vec<String> {
        self.slides.iter().map(Slide::plain_text).collect()
    }
}

/// Which field a search query matched, for display and ranking hints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchField {
    Number,
    Title,
    Filename,
    Body,
}

/// A ranked search result. Borrows the matched entry from the `Searcher` so
/// no hymn body is cloned on a keystroke. `index` identifies the entry
/// within the searcher for later lookup.
#[derive(Debug, Clone)]
pub struct SearchHit<'a> {
    pub index: usize,
    pub entry: &'a Hymn,
    pub score: u32,
    pub field: MatchField,
}
