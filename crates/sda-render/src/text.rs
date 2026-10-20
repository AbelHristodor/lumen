//! CPU text rasterizer. Turns a `ResolvedSlide` into one RGBA image the size
//! of the target. Color is encoded sRGB with straight alpha. The GPU then
//! draws the image as a single textured quad.

use crate::fonts::fonts;
use cosmic_text::{
    Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Style, SwashCache,
    SwashContent as Content, SwashImage, Weight, Wrap,
};
use image::RgbaImage;
use sda_core::resolve::{ResolvedBox, ResolvedLine, ResolvedRun, ResolvedSlide};
use sda_core::theme::{BundledFont, Font, HAlign, Rgba, TextSize, VAlign};

/// Width of a PPTX slide in points. A `Pt` size is a point on a slide this
/// wide, so pixels per point is the scene width over this.
const SLIDE_WIDTH_PT: f32 = 720.0;
/// Smallest Auto size in points on a 720 pt wide slide. Scaled to the
/// scene width like `Pt` sizes so a thumbnail does not overflow.
const AUTO_MIN_PT: f32 = 12.0;
/// Floor for the scaled minimum, in pixels.
const AUTO_MIN_PX: f32 = 4.0;
const AUTO_MAX_PT: f32 = 400.0;
/// Superscript runs at this fraction of the line's size, raised by
/// `SUPERSCRIPT_RAISE` of the line height.
const SUPERSCRIPT_SCALE: f32 = 0.6;
const SUPERSCRIPT_RAISE: f32 = 0.3;
const FOOTER_SCALE: f32 = 0.03;
const FOOTER_MARGIN: f32 = 0.02;
const FOOTER_OPACITY: f32 = 0.7;

pub struct TextRaster {
    font_system: FontSystem,
    swash: SwashCache,
}

struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Clone, Copy)]
enum Pass {
    Shadow,
    Outline,
    Fill,
}

impl TextRaster {
    /// Loads system fonts and the bundled faces. Slow, call once.
    pub fn new() -> Self {
        let mut font_system = FontSystem::new();
        for (_, bytes) in fonts() {
            font_system.db_mut().load_font_data(bytes.to_vec());
        }
        TextRaster {
            font_system,
            swash: SwashCache::new(),
        }
    }

    pub fn rasterize(&mut self, slide: &ResolvedSlide, size: (u32, u32)) -> RgbaImage {
        let mut img = RgbaImage::new(size.0, size.1);
        let (w, h) = (size.0 as f32, size.1 as f32);
        let px_per_pt = w / SLIDE_WIDTH_PT;
        for b in &slide.boxes {
            let rect = Rect {
                x: b.frame.x * w,
                y: b.frame.y * h,
                w: b.frame.w * w,
                h: b.frame.h * h,
            };
            self.draw_box(&mut img, b, &rect, px_per_pt);
        }
        if let Some(footer) = &slide.footer {
            self.draw_footer(&mut img, footer, (w, h));
        }
        img
    }

    /// Draw one text box. `px_per_pt` turns a `Pt` size into pixels for this
    /// scene width. Auto sizes fit the box and ignore it.
    fn draw_box(&mut self, img: &mut RgbaImage, b: &ResolvedBox, rect: &Rect, px_per_pt: f32) {
        let runs: Vec<&ResolvedRun> = b.lines.iter().flat_map(|l| l.runs.iter()).collect();
        if runs.is_empty() {
            return;
        }
        let spacing = b.lines.first().map_or(1.1, |l| l.spacing);
        let auto = runs.iter().all(|r| matches!(r.size, TextSize::Auto));
        let fixed_pt = runs
            .iter()
            .filter_map(|r| match r.size {
                TextSize::Pt(p) => Some(p * px_per_pt),
                TextSize::Auto => None,
            })
            .fold(0.0, f32::max)
            .max(1.0);

        // One span per run. Runs keep their index in `metadata` so painting
        // can look up color, opacity, shadow, and outline per glyph. `k`
        // scales every fixed size in the box by the same factor so mixed
        // sizes keep their ratios. `auto_pt` is the trial size of an auto
        // box, where only superscript runs carry their own metrics.
        let spans_at = |auto_pt: Option<f32>, k: f32| -> Vec<(&str, Attrs)> {
            let mut spans = Vec::new();
            let mut idx = 0;
            for (li, line) in b.lines.iter().enumerate() {
                if li > 0 {
                    spans.push(("\n", Attrs::new()));
                }
                for run in &line.runs {
                    let mut a = Attrs::new()
                        .family(Family::Name(run.font.family()))
                        .weight(if run.bold {
                            Weight::BOLD
                        } else {
                            Weight::NORMAL
                        })
                        .style(if run.italic {
                            Style::Italic
                        } else {
                            Style::Normal
                        })
                        .metadata(idx);
                    let base = match (auto_pt, run.size) {
                        (Some(p), _) => p,
                        (None, TextSize::Pt(p)) => p * px_per_pt * k,
                        (None, TextSize::Auto) => fixed_pt * k,
                    };
                    let pt = if run.superscript {
                        base * SUPERSCRIPT_SCALE
                    } else {
                        base
                    };
                    if !auto || run.superscript {
                        a = a.metrics(Metrics::new(pt, base * line.spacing));
                    }
                    spans.push((run.text.as_str(), a));
                    idx += 1;
                }
            }
            spans
        };

        let fs = &mut self.font_system;
        let mut buffer = Buffer::new(fs, Metrics::new(fixed_pt, fixed_pt * spacing));
        buffer.set_size(fs, Some(rect.w), None);
        // Hymn boxes keep the author's line breaks and let width bound the
        // auto size. Verse boxes and fixed sizes wrap so a long line stays
        // on screen.
        let wrap = if auto && !b.wrap {
            Wrap::None
        } else {
            Wrap::Word
        };
        buffer.set_wrap(fs, wrap);
        if auto {
            let mut lay = |pt: f32| {
                buffer.set_rich_text(
                    fs,
                    spans_at(Some(pt), 1.0),
                    &Attrs::new(),
                    Shaping::Advanced,
                    None,
                );
                buffer.set_metrics(fs, Metrics::new(pt, pt * spacing));
                buffer.shape_until_scroll(fs, false);
                measure(&buffer)
            };
            let min = (AUTO_MIN_PT * px_per_pt).max(AUTO_MIN_PX);
            let max = b.max_pt.map_or(AUTO_MAX_PT, |p| p * px_per_pt).max(min);
            let pt = sda_core::fit::fit_size(&mut lay, (rect.w, rect.h), min, max);
            lay(pt);
        } else {
            // Fixed sizes shrink as one block when the wrapped text is
            // taller than the box or a word is wider. Never grow above the
            // author's size. The search runs in percent because `fit_size`
            // stops at half a unit.
            let mut lay = |pct: f32| {
                let k = pct / 100.0;
                buffer.set_rich_text(
                    fs,
                    spans_at(None, k),
                    &Attrs::new(),
                    Shaping::Advanced,
                    None,
                );
                let pt = fixed_pt * k;
                buffer.set_metrics(fs, Metrics::new(pt, pt * spacing));
                buffer.shape_until_scroll(fs, false);
                measure(&buffer)
            };
            let pct = sda_core::fit::fit_size(&mut lay, (rect.w, rect.h), 20.0, 100.0);
            lay(pct);
        }
        buffer.shape_until_scroll(fs, false);

        let (_, total_h) = measure(&buffer);
        // A block still taller than the box sits on the top edge so the
        // first line is never clipped.
        let offset = if total_h > rect.h {
            0.0
        } else {
            match b.v_align {
                VAlign::Top => 0.0,
                VAlign::Middle => (rect.h - total_h) / 2.0,
                VAlign::Bottom => rect.h - total_h,
            }
        };
        let placed = Rect {
            x: rect.x,
            y: rect.y + offset,
            w: rect.w,
            h: rect.h,
        };
        for pass in [Pass::Shadow, Pass::Outline, Pass::Fill] {
            self.paint(img, &buffer, &b.lines, &runs, &placed, pass);
        }
    }

    fn draw_footer(&mut self, img: &mut RgbaImage, text: &str, (w, h): (f32, f32)) {
        let pt = h * FOOTER_SCALE;
        let run = ResolvedRun {
            text: text.into(),
            font: Font::Bundled(BundledFont::Inter),
            size: TextSize::Pt(pt),
            bold: false,
            italic: false,
            color: Rgba::WHITE,
            opacity: FOOTER_OPACITY,
            shadow: None,
            outline: None,
            superscript: false,
        };
        let line = ResolvedLine {
            align: HAlign::Right,
            spacing: 1.2,
            runs: Vec::new(),
        };
        let fs = &mut self.font_system;
        let mut buffer = Buffer::new(fs, Metrics::new(pt, pt * line.spacing));
        let attrs = Attrs::new().family(Family::Name(run.font.family()));
        buffer.set_text(fs, text, &attrs, Shaping::Advanced, None);
        buffer.set_size(fs, None, None);
        buffer.shape_until_scroll(fs, false);
        let (_, th) = measure(&buffer);
        let margin = h * FOOTER_MARGIN;
        let rect = Rect {
            x: margin,
            y: h - margin - th,
            w: w - 2.0 * margin,
            h,
        };
        self.paint(img, &buffer, &[line], &[&run], &rect, Pass::Fill);
    }

    /// Stamp every glyph of `buffer` for one pass. `lines` gives the
    /// alignment per source line and `runs` the style per glyph. `rect.y`
    /// is the top of the first line.
    fn paint(
        &mut self,
        img: &mut RgbaImage,
        buffer: &Buffer,
        lines: &[ResolvedLine],
        runs: &[&ResolvedRun],
        rect: &Rect,
        pass: Pass,
    ) {
        for lrun in buffer.layout_runs() {
            let align = lines.get(lrun.line_i).map_or(HAlign::Left, |l| l.align);
            let x0 = rect.x
                + match align {
                    HAlign::Left => 0.0,
                    HAlign::Center => (rect.w - lrun.line_w) / 2.0,
                    HAlign::Right => rect.w - lrun.line_w,
                };
            for g in lrun.glyphs {
                let Some(run) = runs.get(g.metadata) else {
                    continue;
                };
                let stamps = stamps(run, pass);
                if stamps.is_empty() {
                    continue;
                }
                let raise = if run.superscript {
                    lrun.line_height * SUPERSCRIPT_RAISE
                } else {
                    0.0
                };
                let phys = g.physical((x0, rect.y + lrun.line_y - raise), 1.0);
                let Some(image) = self.swash.get_image(&mut self.font_system, phys.cache_key)
                else {
                    continue;
                };
                for (dx, dy, color) in stamps {
                    let gx = phys.x + image.placement.left + dx;
                    let gy = phys.y - image.placement.top + dy;
                    blit(img, image, gx, gy, color, run.opacity);
                }
            }
        }
    }
}

impl Default for TextRaster {
    fn default() -> Self {
        Self::new()
    }
}

/// Max line width and total height of the laid out text.
fn measure(buffer: &Buffer) -> (f32, f32) {
    let mut w = 0.0f32;
    let mut h = 0.0f32;
    for run in buffer.layout_runs() {
        w = w.max(run.line_w);
        h = h.max(run.line_top + run.line_height);
    }
    (w, h)
}

/// Alpha of each of the 9 stamps a blurred shadow is built from. Where they
/// all overlap the coverage reaches about 97 percent.
const SHADOW_TAP_ALPHA: u8 = 96;

/// Pixel offsets and colors a run needs for one pass. A shadow with blur
/// above zero is stamped on a 3 by 3 grid spread by the blur radius with a
/// lighter color so the edge fades. ponytail: a true gaussian would need a
/// separate blur buffer, add one if the grid shows.
fn stamps(run: &ResolvedRun, pass: Pass) -> Vec<(i32, i32, Rgba)> {
    match pass {
        Pass::Shadow => run
            .shadow
            .iter()
            .flat_map(|s| {
                let (ox, oy) = (s.offset.0.round() as i32, s.offset.1.round() as i32);
                if s.blur <= 0.0 {
                    return vec![(ox, oy, s.color)];
                }
                let d = s.blur.round().max(1.0) as i32;
                let soft = Rgba::new(
                    s.color.r,
                    s.color.g,
                    s.color.b,
                    mul(s.color.a, SHADOW_TAP_ALPHA),
                );
                (-1..=1)
                    .flat_map(|y| (-1..=1).map(move |x| (ox + x * d, oy + y * d, soft)))
                    .collect()
            })
            .collect(),
        Pass::Outline => run
            .outline
            .iter()
            .flat_map(|o| {
                let d = o.width.round().max(1.0) as i32;
                [
                    (-1, -1),
                    (0, -1),
                    (1, -1),
                    (-1, 0),
                    (1, 0),
                    (-1, 1),
                    (0, 1),
                    (1, 1),
                ]
                .map(|(x, y)| (x * d, y * d, o.color))
            })
            .collect(),
        Pass::Fill => vec![(0, 0, run.color)],
    }
}

/// Composite one glyph image at `gx, gy` with `color` and `opacity`.
fn blit(img: &mut RgbaImage, glyph: &SwashImage, gx: i32, gy: i32, color: Rgba, opacity: f32) {
    let (w, h) = img.dimensions();
    let pw = glyph.placement.width as i32;
    let ph = glyph.placement.height as i32;
    for oy in 0..ph {
        for ox in 0..pw {
            let x = gx + ox;
            let y = gy + oy;
            if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                continue;
            }
            let i = (oy * pw + ox) as usize;
            let src = match glyph.content {
                Content::Mask => Rgba::new(color.r, color.g, color.b, mul(color.a, glyph.data[i])),
                Content::Color => {
                    let d = &glyph.data[i * 4..i * 4 + 4];
                    Rgba::new(d[0], d[1], d[2], d[3])
                }
                Content::SubpixelMask => continue,
            };
            over(img.get_pixel_mut(x as u32, y as u32), src, opacity);
        }
    }
}

fn mul(a: u8, b: u8) -> u8 {
    ((a as u32 * b as u32 + 127) / 255) as u8
}

/// Straight alpha source over straight alpha destination.
fn over(dst: &mut image::Rgba<u8>, src: Rgba, opacity: f32) {
    let sa = src.a as f32 / 255.0 * opacity.clamp(0.0, 1.0);
    if sa <= 0.0 {
        return;
    }
    let da = dst[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    for (c, s) in [src.r, src.g, src.b].into_iter().enumerate() {
        let d = dst[c] as f32;
        dst[c] = ((s as f32 * sa + d * da * (1.0 - sa)) / out_a).round() as u8;
    }
    dst[3] = (out_a * 255.0).round() as u8;
}
