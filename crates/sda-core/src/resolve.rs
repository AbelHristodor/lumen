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
    /// A video file. Drawn by the player, no theme applies.
    Video(PathBuf),
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
    /// Drawn smaller and raised, like a verse number in a printed Bible.
    pub superscript: bool,
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
    /// Wrap long lines at words. Off keeps the author's line breaks and lets
    /// the width bound the auto size, which suits hymn slides.
    pub wrap: bool,
    /// Ceiling for an auto size, in points on a 720 by 405 slide. One short
    /// verse would otherwise fill the frame.
    pub max_pt: Option<f32>,
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
/// Horizontal inset of the verse column when the theme margin is `Inherit`.
/// Wider than `FALLBACK_MARGIN` so a flowing paragraph reads comfortably.
pub const VERSE_MARGIN_X: f32 = 0.10;
pub const REFERENCE_SCALE: f32 = 0.6;
/// Slide height fraction the reference strip takes above or below the verses.
pub const REF_BAND: f32 = 0.12;
/// Fraction of the band the reference line may fill. An auto sized line
/// grows to its box, so a full band reads as big as the verses.
pub const REF_TEXT: f32 = 0.5;
/// Largest auto size for verses. A fixed theme size is not capped.
pub const VERSE_MAX_PT: f32 = 40.0;
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
        SlideContent::Bitmap(_) | SlideContent::Video(_) => None,
        SlideContent::Styled(slide) => Some(ResolvedSlide {
            background: solid_background(theme, slide.background),
            boxes: {
                let solo = slide.boxes.len() == 1;
                slide
                    .boxes
                    .iter()
                    .map(|b| resolve_box(&theme.text, b, solo))
                    .collect()
            },
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

/// `margin_frame` with the wider verse inset on the sides when the theme
/// margin is `Inherit`. A set margin wins on every side.
fn verse_margin_frame(text: &TextStyle) -> Frame {
    let m = text.margin.resolve(None, FALLBACK_MARGIN);
    let mx = match text.margin {
        Prop::Set(m) => m,
        Prop::Inherit => VERSE_MARGIN_X,
    };
    Frame {
        x: mx,
        y: m,
        w: 1.0 - 2.0 * mx,
        h: 1.0 - 2.0 * m,
    }
}

/// Hymn files pad lines with spaces and tabs and place the frame off
/// center. Trim the padding, drop blank edge lines, and center a lone
/// box's frame on the slide. `solo` is true when the slide has one box.
fn resolve_box(text: &TextStyle, b: &TextBox, solo: bool) -> ResolvedBox {
    let mut frame = match (&text.margin, b.frame) {
        (Prop::Inherit, Some(f)) => f,
        _ => margin_frame(text),
    };
    let centered = b
        .paragraphs
        .iter()
        .all(|p| text.h_align.resolve(p.align, HAlign::Center) == HAlign::Center);
    if solo && centered && b.frame.is_some() {
        frame.x = (1.0 - frame.w) / 2.0;
        frame.y = (1.0 - frame.h) / 2.0;
    }
    let mut lines: Vec<ResolvedLine> = b
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
                    superscript: false,
                })
                .collect(),
        })
        .collect();
    for line in &mut lines {
        trim_runs(&mut line.runs);
    }
    let blank = |l: &ResolvedLine| l.runs.is_empty();
    while lines.first().is_some_and(blank) {
        lines.remove(0);
    }
    while lines.last().is_some_and(blank) {
        lines.pop();
    }
    ResolvedBox {
        frame,
        v_align: text.v_align,
        lines,
        wrap: false,
        max_pt: None,
    }
}

/// Strip whitespace from the ends of a line and drop runs left empty.
fn trim_runs(runs: &mut Vec<ResolvedRun>) {
    while let Some(first) = runs.first_mut() {
        first.text = first.text.trim_start().to_string();
        if first.text.is_empty() {
            runs.remove(0);
        } else {
            break;
        }
    }
    while let Some(last) = runs.last_mut() {
        last.text = last.text.trim_end().to_string();
        if last.text.is_empty() {
            runs.pop();
        } else {
            break;
        }
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
    let full = verse_margin_frame(&theme.text);
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
        superscript: false,
    }
}

fn plain_line(text: &TextStyle, r: ResolvedRun) -> ResolvedLine {
    ResolvedLine {
        align: text.h_align.resolve(None, HAlign::Center),
        spacing: text.line_spacing.resolve(None, FALLBACK_SPACING),
        runs: vec![r],
    }
}

/// Verses flow into one wrapped paragraph. A verse number is a superscript
/// run joined to its text by a no-break space so it never ends a line.
fn verse_runs(theme: &Theme, verses: &[(u32, String)]) -> Vec<ResolvedRun> {
    let text = &theme.text;
    let mut runs = Vec::new();
    for (i, (n, v)) in verses.iter().enumerate() {
        let sep = if i == 0 { "" } else { " " };
        if theme.verses.show_numbers {
            let mut number = plain_run(text, format!("{sep}{n}"));
            number.superscript = true;
            runs.push(number);
            runs.push(plain_run(text, format!("\u{a0}{v}")));
        } else {
            runs.push(plain_run(text, format!("{sep}{v}")));
        }
    }
    runs
}

fn resolve_verses(theme: &Theme, reference: &str, verses: &[(u32, String)]) -> Vec<ResolvedBox> {
    let text = &theme.text;
    let lines = vec![ResolvedLine {
        align: text.h_align.resolve(None, HAlign::Center),
        spacing: text.line_spacing.resolve(None, FALLBACK_SPACING),
        runs: verse_runs(theme, verses),
    }];
    let full = verse_margin_frame(text);
    if !theme.verses.show_reference {
        return vec![ResolvedBox {
            frame: full,
            v_align: text.v_align,
            lines,
            wrap: true,
            max_pt: Some(VERSE_MAX_PT),
        }];
    }
    let style = reference_style(theme);
    // One cut feeds both boxes so the band and the verses never overlap.
    // `reference.margin` sets the band's horizontal inset only.
    let (strip, rest) = full.cut(style.v_align, REF_BAND);
    // An inherited reference margin follows the verse column.
    let inset = match theme.reference.margin {
        Prop::Set(_) => margin_frame(&style),
        Prop::Inherit => full,
    };
    let mut boxes = vec![ResolvedBox {
        frame: rest,
        v_align: text.v_align,
        lines,
        wrap: true,
        max_pt: Some(VERSE_MAX_PT),
    }];
    // Middle keeps the whole frame and overlays the verses, by choice.
    let h = if style.v_align == VAlign::Middle {
        strip.h
    } else {
        strip.h * REF_TEXT
    };
    boxes.push(ResolvedBox {
        frame: Frame {
            x: inset.x,
            w: inset.w,
            y: strip.y + (strip.h - h) / 2.0,
            h,
        },
        v_align: style.v_align,
        lines: vec![plain_line(&style, plain_run(&style, reference.to_string()))],
        wrap: false,
        max_pt: None,
    });
    boxes
}
