//! Pure resolution of a theme plus slide content into concrete values the
//! renderer can draw. No I/O. `Inherit` properties take the pptx value when
//! the file has one and a fallback otherwise.

use crate::model::{Frame, Slide, TextBox};
use crate::theme::{
    Background, BundledFont, Font, HAlign, Outline, Prop, Rgba, Shadow, TextSize, TextStyle, Theme,
    VAlign,
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
pub const REFERENCE_SCALE: f32 = 0.6;
/// Slide height fraction the reference strip takes above or below the verses.
pub const REF_BAND: f32 = 0.12;
/// A 16 by 9 slide measured in points, the unit `TextSize::Pt` uses.
pub const SLIDE_PT: (f32, f32) = (720.0, 405.0);

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
            boxes: resolve_verses(theme, reference, verses),
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

/// The reference style with every `Prop` resolved. `Inherit` takes the
/// verse text value, size at `REFERENCE_SCALE` of the verse size.
pub fn reference_style(theme: &Theme) -> TextStyle {
    let t = &theme.text;
    let r = &theme.reference;
    let verse_size = t.size.resolve(None, TextSize::Auto);
    let small = match verse_size {
        TextSize::Pt(p) => TextSize::Pt(p * REFERENCE_SCALE),
        TextSize::Auto => TextSize::Auto,
    };
    TextStyle {
        font: Prop::Set(r.font.resolve(
            Some(t.font.resolve(None, Font::Bundled(BundledFont::Inter))),
            Font::Bundled(BundledFont::Inter),
        )),
        size: Prop::Set(r.size.resolve(Some(small), TextSize::Auto)),
        bold: Prop::Set(r.bold.resolve(Some(t.bold.resolve(None, false)), false)),
        color: Prop::Set(
            r.color
                .resolve(Some(t.color.resolve(None, FALLBACK_COLOR)), FALLBACK_COLOR),
        ),
        h_align: Prop::Set(r.h_align.resolve(
            Some(t.h_align.resolve(None, HAlign::Center)),
            HAlign::Center,
        )),
        v_align: r.v_align,
        line_spacing: Prop::Set(r.line_spacing.resolve(
            Some(t.line_spacing.resolve(None, FALLBACK_SPACING)),
            FALLBACK_SPACING,
        )),
        margin: Prop::Set(r.margin.resolve(
            Some(t.margin.resolve(None, FALLBACK_MARGIN)),
            FALLBACK_MARGIN,
        )),
        opacity: r.opacity,
        shadow: r.shadow,
        outline: r.outline,
    }
}

/// The frame the verses fill after the reference band is taken off.
pub fn verse_frame(theme: &Theme) -> Frame {
    let full = margin_frame(&theme.text);
    if !theme.verses.show_reference {
        return full;
    }
    full.cut(theme.reference.v_align, REF_BAND).1
}

/// `verse_frame` in points on a 720 by 405 slide, for `bible::fits_heuristic`.
pub fn verse_box_pt(theme: &Theme) -> (f32, f32) {
    let f = verse_frame(theme);
    (f.w * SLIDE_PT.0, f.h * SLIDE_PT.1)
}

fn plain_run(text: &TextStyle, s: String) -> ResolvedRun {
    ResolvedRun {
        text: s,
        font: text.font.resolve(None, Font::Bundled(BundledFont::Inter)),
        size: text.size.resolve(None, TextSize::Auto),
        bold: text.bold.resolve(None, false),
        italic: false,
        color: text.color.resolve(None, FALLBACK_COLOR),
        opacity: text.opacity,
        shadow: text.shadow,
        outline: text.outline,
    }
}

fn plain_line(text: &TextStyle, r: ResolvedRun) -> ResolvedLine {
    ResolvedLine {
        align: text.h_align.resolve(None, HAlign::Center),
        spacing: text.line_spacing.resolve(None, FALLBACK_SPACING),
        runs: vec![r],
    }
}

fn resolve_verses(theme: &Theme, reference: &str, verses: &[(u32, String)]) -> Vec<ResolvedBox> {
    let text = &theme.text;
    let lines = verses
        .iter()
        .map(|(n, v)| {
            let s = if theme.verses.show_numbers {
                format!("{n} {v}")
            } else {
                v.clone()
            };
            plain_line(text, plain_run(text, s))
        })
        .collect();
    let mut boxes = vec![ResolvedBox {
        frame: verse_frame(theme),
        v_align: text.v_align,
        lines,
    }];
    if theme.verses.show_reference {
        let style = reference_style(theme);
        let frame = margin_frame(&style).cut(style.v_align, REF_BAND).0;
        boxes.push(ResolvedBox {
            frame,
            v_align: style.v_align,
            lines: vec![plain_line(&style, plain_run(&style, reference.to_string()))],
        });
    }
    boxes
}
