use std::sync::Arc;

use egui::{Color32, FontData, FontDefinitions, FontFamily, Visuals};
use sda_core::theme::{Font, Rgba};

/// The single accent color of the UI.
pub const ACCENT: Color32 = Color32::from_rgb(79, 140, 255);

/// Register bundled fonts and apply dark visuals with the app spacing.
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let all: [(&str, &'static [u8]); 8] = [
        (
            "Inter",
            include_bytes!("../../../assets/fonts/Inter-Regular.ttf"),
        ),
        (
            "Inter Bold",
            include_bytes!("../../../assets/fonts/Inter-Bold.ttf"),
        ),
        (
            "Noto Serif",
            include_bytes!("../../../assets/fonts/NotoSerif-Regular.ttf"),
        ),
        (
            "Noto Serif Bold",
            include_bytes!("../../../assets/fonts/NotoSerif-Bold.ttf"),
        ),
        (
            "Lora",
            include_bytes!("../../../assets/fonts/Lora-Regular.ttf"),
        ),
        (
            "Lora Bold",
            include_bytes!("../../../assets/fonts/Lora-Bold.ttf"),
        ),
        (
            "Roboto Slab",
            include_bytes!("../../../assets/fonts/RobotoSlab-Regular.ttf"),
        ),
        (
            "Roboto Slab Bold",
            include_bytes!("../../../assets/fonts/RobotoSlab-Bold.ttf"),
        ),
    ];
    for (name, bytes) in all {
        fonts
            .font_data
            .insert(name.into(), Arc::new(FontData::from_static(bytes)));
        fonts
            .families
            .insert(FontFamily::Name(name.into()), vec![name.into()]);
    }
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "Inter".into());
    ctx.set_fonts(fonts);
    ctx.set_visuals(Visuals::dark());
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(12.0, 12.0);
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
        let w = &mut s.visuals.widgets;
        for w in [
            &mut w.noninteractive,
            &mut w.inactive,
            &mut w.hovered,
            &mut w.active,
        ] {
            w.corner_radius = 8.into();
        }
        s.visuals.selection.bg_fill = ACCENT;
        s.visuals.hyperlink_color = ACCENT;
    });
}

/// The registered family for a bundled font and weight.
pub fn font_family(font: Font, bold: bool) -> FontFamily {
    let base = font.label();
    FontFamily::Name(if bold {
        format!("{base} Bold").into()
    } else {
        base.into()
    })
}

/// Convert a theme color to an egui color.
pub fn color32(c: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
}

/// Convert an egui color to an opaque theme color.
pub fn rgba(c: Color32) -> Rgba {
    Rgba::new(c.r(), c.g(), c.b(), 255)
}

#[cfg(test)]
mod tests {
    #[test]
    fn fonts_install_without_panic() {
        let ctx = egui::Context::default();
        super::install(&ctx);
        let mut out = ctx.run_ui(Default::default(), |ui| {
            for f in sda_core::theme::Font::all() {
                ui.label(egui::RichText::new("Plecaţi-vă").family(super::font_family(f, true)));
            }
        });
        out.textures_delta.clear();
    }
}
