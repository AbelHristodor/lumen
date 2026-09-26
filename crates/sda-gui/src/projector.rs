use crate::app::App;
use crate::{shortcuts, slide, style};

/// One output screen the projector can open on.
pub struct Display {
    pub index: usize,
    pub label: String,
    pub is_primary: bool,
}

/// Every connected display, or a single primary placeholder if none can be read.
pub fn list_displays() -> Vec<Display> {
    match display_info::DisplayInfo::all() {
        Ok(l) if !l.is_empty() => l
            .into_iter()
            .enumerate()
            .map(|(i, d)| Display {
                index: i,
                label: format!("{} ({}x{})", d.name, d.width, d.height),
                is_primary: d.is_primary,
            })
            .collect(),
        other => {
            if let Err(e) = other {
                log::warn!("display enumeration failed: {e}");
            }
            vec![Display {
                index: 0,
                label: "Primary".into(),
                is_primary: true,
            }]
        }
    }
}

/// The first secondary display, else the first display.
pub fn default_display(displays: &[Display]) -> usize {
    displays
        .iter()
        .find(|d| !d.is_primary)
        .or_else(|| displays.first())
        .map_or(0, |d| d.index)
}

/// Draw the fullscreen projector viewport while `app.projector` is set.
pub fn show(ctx: &egui::Context, app: &mut App) {
    let Some(display) = app.projector.as_ref().map(|p| p.display) else {
        return;
    };
    let builder = egui::ViewportBuilder::default()
        .with_title("Projector")
        .with_decorations(false)
        .with_monitor(display);
    let mut close = false;
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("projector"),
        builder,
        |ui, _class| {
            if ui.input(|i| i.viewport().close_requested()) {
                close = true;
            }
            shortcuts::handle(ui, app);
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
                .show(ui, |ui| {
                    let rect = ui.max_rect();
                    let present = &app.core.present;
                    if present.blank {
                        ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);
                        return;
                    }
                    match present.current_slide() {
                        Some(text) => {
                            let f = slide::footer_text(present.number.as_deref(), &present.title);
                            slide::paint(
                                ui.painter(),
                                rect,
                                &app.core.active_theme,
                                text,
                                Some(&f),
                                &mut app.ui.fit,
                            );
                        }
                        None => {
                            ui.painter().rect_filled(
                                rect,
                                0.0,
                                style::color32(app.core.active_theme.bg_color),
                            );
                        }
                    }
                });
        },
    );
    if close {
        app.projector = None;
    }
}
