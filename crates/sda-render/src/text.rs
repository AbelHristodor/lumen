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

const AUTO_MIN_PT: f32 = 12.0;
const AUTO_MAX_PT: f32 = 400.0;
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
        for b in &slide.boxes {
            let rect = Rect {
                x: b.frame.x * w,
                y: b.frame.y * h,
                w: b.frame.w * w,
                h: b.frame.h * h,
            };
            self.draw_box(&mut img, b, &rect);
        }
        if let Some(footer) = &slide.footer {
            self.draw_footer(&mut img, footer, (w, h));
        }
        img
    }

    fn draw_box(&mut self, img: &mut RgbaImage, b: &ResolvedBox, rect: &Rect) {
        let runs: Vec<&ResolvedRun> = b.lines.iter().flat_map(|l| l.runs.iter()).collect();
        if runs.is_empty() {
            return;
        }
        let spacing = b.lines.first().map_or(1.1, |l| l.spacing);
        let auto = runs.iter().all(|r| matches!(r.size, TextSize::Auto));
        let fixed_pt = runs
            .iter()
            .filter_map(|r| match r.size {
                TextSize::Pt(p) => Some(p),
                TextSize::Auto => None,
            })
            .fold(0.0, f32::max)
            .max(1.0);

        // One span per run. Runs keep their index in `metadata` so painting
        // can look up color, opacity, shadow, and outline per glyph.
        let mut spans: Vec<(&str, Attrs)> = Vec::new();
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
                if !auto {
                    let pt = match run.size {
                        TextSize::Pt(p) => p,
                        TextSize::Auto => fixed_pt,
                    };
                    a = a.metrics(Metrics::new(pt, pt * line.spacing));
                }
                spans.push((run.text.as_str(), a));
                idx += 1;
            }
        }

        let fs = &mut self.font_system;
        let mut buffer = Buffer::new(fs, Metrics::new(fixed_pt, fixed_pt * spacing));
        buffer.set_rich_text(fs, spans, &Attrs::new(), Shaping::Advanced, None);
        buffer.set_size(fs, Some(rect.w), None);
        // Auto boxes keep the author's line breaks and let width bound the
        // size. Fixed sizes wrap so a long line stays on screen.
        buffer.set_wrap(fs, if auto { Wrap::None } else { Wrap::Word });
        let pt = if auto {
            sda_core::fit::fit_size(
                |pt| {
                    buffer.set_metrics(fs, Metrics::new(pt, pt * spacing));
                    buffer.shape_until_scroll(fs, false);
                    measure(&buffer)
                },
                (rect.w, rect.h),
                AUTO_MIN_PT,
                AUTO_MAX_PT,
            )
        } else {
            fixed_pt
        };
        buffer.set_metrics(fs, Metrics::new(pt, pt * spacing));
        buffer.shape_until_scroll(fs, false);

        let (_, total_h) = measure(&buffer);
        let placed = Rect {
            x: rect.x,
            y: rect.y
                + match b.v_align {
                    VAlign::Top => 0.0,
                    VAlign::Middle => (rect.h - total_h) / 2.0,
                    VAlign::Bottom => rect.h - total_h,
                },
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
                let phys = g.physical((x0, rect.y + lrun.line_y), 1.0);
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

/// Pixel offsets and colors a run needs for one pass. Shadow blur is not
/// applied. ponytail: hard shadow only, add a blur pass if themes use it.
fn stamps(run: &ResolvedRun, pass: Pass) -> Vec<(i32, i32, Rgba)> {
    match pass {
        Pass::Shadow => run
            .shadow
            .iter()
            .map(|s| {
                (
                    s.offset.0.round() as i32,
                    s.offset.1.round() as i32,
                    s.color,
                )
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
