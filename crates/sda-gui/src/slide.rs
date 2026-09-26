use std::collections::HashMap;

use egui::text::LayoutJob;
use egui::{pos2, vec2, Align, FontId, Painter, Rect};
use sda_core::fit::fit_size;
use sda_core::theme::{FontSize, HAlign, Theme, VAlign};

use crate::style;

/// Auto font sizes keyed by area size and text, reset when the theme changes.
#[derive(Default)]
pub struct FitCache {
    theme_key: u64,
    sizes: HashMap<(u32, u32, String), f32>,
}

/// Footer line shown under a slide.
pub fn footer_text(number: Option<&str>, title: &str) -> String {
    match number {
        Some(n) => format!("{n} · {title}"),
        None => title.to_string(),
    }
}

/// Paint one slide filling `rect`. The only place slide text is drawn.
pub fn paint(
    painter: &Painter,
    rect: Rect,
    theme: &Theme,
    text: &str,
    footer: Option<&str>,
    cache: &mut FitCache,
) {
    painter.rect_filled(rect, 0.0, style::color32(theme.bg_color));
    let short = rect.width().min(rect.height());
    let m = short * theme.margin;
    let footer_h = if theme.footer && footer.is_some() {
        short * 0.05
    } else {
        0.0
    };
    let area = Rect::from_min_max(rect.min + vec2(m, m), rect.max - vec2(m, m + footer_h));
    let fam = style::font_family(theme.font, theme.bold);
    let color = style::color32(theme.text_color);
    let halign = match theme.h_align {
        HAlign::Left => Align::LEFT,
        HAlign::Center => Align::Center,
        HAlign::Right => Align::RIGHT,
    };
    let job_at = |pt: f32| {
        let mut job = LayoutJob::simple(
            text.to_owned(),
            FontId::new(pt, fam.clone()),
            color,
            area.width(),
        );
        job.halign = halign;
        for s in &mut job.sections {
            s.format.line_height = Some(pt * theme.line_spacing);
        }
        job
    };
    let pt = match theme.size {
        FontSize::Pt(p) => p,
        FontSize::Auto => {
            if cache.theme_key != theme.cache_key() || cache.sizes.len() > 64 {
                cache.sizes.clear();
                cache.theme_key = theme.cache_key();
            }
            let key = (area.width() as u32, area.height() as u32, text.to_owned());
            *cache.sizes.entry(key).or_insert_with(|| {
                fit_size(
                    |p| {
                        let g = painter.layout_job(job_at(p));
                        (g.size().x, g.size().y)
                    },
                    (area.width(), area.height()),
                    8.0,
                    short * 0.5,
                )
            })
        }
    };
    let galley = painter.layout_job(job_at(pt));
    let size = galley.size();
    let x = match theme.h_align {
        HAlign::Left => area.left(),
        HAlign::Center => area.center().x,
        HAlign::Right => area.right(),
    };
    let y = match theme.v_align {
        VAlign::Top => area.top(),
        VAlign::Middle => area.center().y - size.y / 2.0,
        VAlign::Bottom => area.bottom() - size.y,
    };
    painter.galley(pos2(x, y), galley, color);
    if let (true, Some(f)) = (theme.footer, footer) {
        let fpt = (short * 0.025).max(14.0);
        let mut job = LayoutJob::simple(
            f.to_owned(),
            FontId::new(fpt, fam),
            color.gamma_multiply(0.8),
            area.width(),
        );
        job.halign = Align::Center;
        let g = painter.layout_job(job);
        painter.galley(
            pos2(area.center().x, rect.bottom() - m - g.size().y),
            g,
            color,
        );
    }
}
