//! Pure resolution of a theme plus slide content into concrete values the
//! renderer can draw. No I/O. `Inherit` properties take the pptx value when
//! the file has one and a fallback otherwise.

use crate::model::{Frame, Slide, TextBox};
use crate::theme::{
    Background, BundledFont, Font, HAlign, Outline, Prop, RefPos, Rgba, Shadow, TextSize,
    TextStyle, Theme, VAlign,
};
use std::path::PathBuf;

/// What a presentation slide holds before resolution.
#[derive(Debug, Clone, PartialEq)]
pub enum SlideContent {
    Styled(Slide),
    Verses {
        reference: String,
        verses: Vec<(u32, String)>,
    },
    Bitmap(PathBuf),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedRun {
    pub text: String,
    pub font: Font,
    pub size: TextSize,
    pub bold: bool,
    pub italic: bool,
    pub color: Rgba,
    pub opacity: f32,
    pub shadow: Option<Shadow>,
    pub outline: Option<Outline>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedLine {
    pub align: HAlign,
    pub spacing: f32,
    pub runs: Vec<ResolvedRun>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedBox {
    pub frame: Frame,
    pub v_align: VAlign,
    pub lines: Vec<ResolvedLine>,
}

/// A slide ready to draw. `background` is the solid color only, used when the
/// full theme background is not available such as in thumbnails.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSlide {
    pub background: Rgba,
    pub boxes: Vec<ResolvedBox>,
    pub footer: Option<String>,
}

pub const FALLBACK_COLOR: Rgba = Rgba::WHITE;
pub const FALLBACK_SPACING: f32 = 1.1;
pub const FALLBACK_MARGIN: f32 = 0.06;
const REFERENCE_SCALE: f32 = 0.6;

/// Resolve `content` under `theme`. Returns `None` for bitmaps, which the
/// renderer draws as they are.
pub fn resolve(
    theme: &Theme,
    content: &SlideContent,
    footer: Option<&str>,
) -> Option<ResolvedSlide> {
    let footer = footer.map(str::to_string);
    match content {
        SlideContent::Bitmap(_) => None,
        SlideContent::Styled(slide) => Some(ResolvedSlide {
            background: solid_background(theme, slide.background),
            boxes: slide
                .boxes
                .iter()
                .map(|b| resolve_box(&theme.text, b))
                .collect(),
            footer,
        }),
        SlideContent::Verses { reference, verses } => Some(ResolvedSlide {
            background: solid_background(theme, None),
            boxes: vec![resolve_verses(theme, reference, verses)],
            footer,
        }),
    }
}

fn solid_background(theme: &Theme, slide: Option<Rgba>) -> Rgba {
    match theme.background {
        Background::Solid(c) => c,
        _ => slide.unwrap_or(Rgba::BLACK),
    }
}

fn margin_frame(text: &TextStyle) -> Frame {
    let m = text.margin.resolve(None, FALLBACK_MARGIN);
    Frame {
        x: m,
        y: m,
        w: 1.0 - 2.0 * m,
        h: 1.0 - 2.0 * m,
    }
}

fn resolve_box(text: &TextStyle, b: &TextBox) -> ResolvedBox {
    let frame = match (&text.margin, b.frame) {
        (Prop::Inherit, Some(f)) => f,
        _ => margin_frame(text),
    };
    let lines = b
        .paragraphs
        .iter()
        .map(|p| ResolvedLine {
            align: text.h_align.resolve(p.align, HAlign::Center),
            spacing: text.line_spacing.resolve(p.line_spacing, FALLBACK_SPACING),
            runs: p
                .runs
                .iter()
                .map(|r| ResolvedRun {
                    text: r.text.clone(),
                    font: text.font.resolve(
                        r.font.clone().map(Font::System),
                        Font::Bundled(BundledFont::Inter),
                    ),
                    size: text
                        .size
                        .resolve(r.size_pt.map(TextSize::Pt), TextSize::Auto),
                    bold: text.bold.resolve(r.bold, false),
                    italic: r.italic.unwrap_or(false),
                    color: text.color.resolve(r.color, FALLBACK_COLOR),
                    opacity: text.opacity,
                    shadow: text.shadow,
                    outline: text.outline,
                })
                .collect(),
        })
        .collect();
    ResolvedBox {
        frame,
        v_align: text.v_align,
        lines,
    }
}

fn resolve_verses(theme: &Theme, reference: &str, verses: &[(u32, String)]) -> ResolvedBox {
    let text = &theme.text;
    let size = text.size.resolve(None, TextSize::Auto);
    let run = |s: String, size: TextSize| ResolvedRun {
        text: s,
        font: text.font.resolve(None, Font::Bundled(BundledFont::Inter)),
        size,
        bold: text.bold.resolve(None, false),
        italic: false,
        color: text.color.resolve(None, FALLBACK_COLOR),
        opacity: text.opacity,
        shadow: text.shadow,
        outline: text.outline,
    };
    let line = |r: ResolvedRun| ResolvedLine {
        align: text.h_align.resolve(None, HAlign::Center),
        spacing: text.line_spacing.resolve(None, FALLBACK_SPACING),
        runs: vec![r],
    };
    let mut lines: Vec<ResolvedLine> = verses
        .iter()
        .map(|(n, v)| {
            let s = if theme.verses.show_numbers {
                format!("{n} {v}")
            } else {
                v.clone()
            };
            line(run(s, size))
        })
        .collect();
    if theme.verses.show_reference {
        let small = match size {
            TextSize::Pt(p) => TextSize::Pt(p * REFERENCE_SCALE),
            TextSize::Auto => TextSize::Auto,
        };
        let r = line(run(reference.to_string(), small));
        match theme.verses.reference_at {
            RefPos::Top => lines.insert(0, r),
            RefPos::Bottom => lines.push(r),
        }
    }
    ResolvedBox {
        frame: margin_frame(text),
        v_align: text.v_align,
        lines,
    }
}
