//! Pure presentation state. Which content is loaded, which slide is current,
//! and whether output is blanked. No I/O and no UI. The Live strip and the
//! projector read only this struct.

use crate::resolve::SlideContent;
use std::path::PathBuf;

/// Where the loaded content came from.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Source {
    Hymn(PathBuf),
    Bible { version: String, reference: String },
    #[default]
    None,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Presentation {
    pub title: String,
    pub subtitle: Option<String>,
    pub slides: Vec<SlideContent>,
    pub index: usize,
    pub blank: bool,
    pub source: Source,
}

impl Presentation {
    /// Load new content. Resets to the first slide and unblanks.
    pub fn load(&mut self, title: String, subtitle: Option<String>, slides: Vec<SlideContent>, source: Source) {
        self.title = title;
        self.subtitle = subtitle;
        self.slides = slides;
        self.source = source;
        self.index = 0;
        self.blank = false;
    }

    pub fn slide_count(&self) -> usize {
        self.slides.len()
    }

    /// The current slide, or None when nothing is loaded.
    pub fn current(&self) -> Option<&SlideContent> {
        self.slides.get(self.index)
    }

    /// Advance one slide, clamped at the last slide.
    pub fn next(&mut self) {
        if self.index + 1 < self.slides.len() {
            self.index += 1;
        }
    }

    /// Go back one slide, clamped at the first.
    pub fn prev(&mut self) {
        self.index = self.index.saturating_sub(1);
    }

    /// Jump to slide `n`, 1 based. Out of range values are ignored.
    pub fn jump(&mut self, n: usize) {
        if (1..=self.slide_count()).contains(&n) {
            self.index = n - 1;
        }
    }

    pub fn toggle_blank(&mut self) {
        self.blank = !self.blank;
    }

    pub fn is_loaded(&self) -> bool {
        !self.slides.is_empty()
    }
}
