use crate::app::App;
use crate::{projector, slide, style};

/// Draw the live card with the slide on the wall, slide controls and the display picker.
pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let s = app.core.strings.clone();
    ui.horizontal(|ui| {
        ui.heading(&s.live);
        if app.projector.is_some() {
            ui.colored_label(style::ACCENT, "●");
        }
    });
    let present = &app.core.present;
    if let Some(text) = present.current_slide() {
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, w * 9.0 / 16.0), egui::Sense::hover());
        if present.blank {
            ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);
        } else {
            let footer = slide::footer_text(present.number.as_deref(), &present.title);
            slide::paint(
                ui.painter(),
                rect,
                &app.core.active_theme,
                text,
                Some(&footer),
                &mut app.ui.fit,
            );
        }
        ui.label(format!(
            "{}  {}/{}",
            present.title,
            present.index + 1,
            present.slide_count()
        ));
        if let Some(next) = present.next_slide() {
            ui.small(format!(
                "{}: {}",
                s.next_label,
                next.lines().next().unwrap_or("")
            ));
        }
    } else {
        ui.label(&s.nothing_live);
    }
    ui.horizontal(|ui| {
        if ui.button("◀").clicked() {
            app.live_prev();
        }
        if ui.button("▶").clicked() {
            app.live_next();
        }
        if ui
            .selectable_label(app.core.present.blank, format!("{} (B)", s.blank))
            .clicked()
        {
            app.live_blank();
        }
    });
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(&s.display);
        let open = app.projector.as_ref().map(|p| p.display);
        let saved = app
            .core
            .cfg
            .output_display
            .and_then(|i| usize::try_from(i).ok())
            .filter(|&i| i < app.displays.len());
        let current = open
            .or(saved)
            .unwrap_or_else(|| projector::default_display(&app.displays));
        let mut picked = current;
        egui::ComboBox::from_id_salt("display")
            .selected_text(app.displays.get(current).map_or("?", |d| d.label.as_str()))
            .show_ui(ui, |ui| {
                for d in &app.displays {
                    ui.selectable_value(&mut picked, d.index, &d.label);
                }
            });
        match open {
            None => {
                if picked != current {
                    app.core.cfg.output_display = i32::try_from(picked).ok();
                    app.core.save_cfg();
                }
                if ui
                    .add(egui::Button::new(&s.projector_on).fill(style::ACCENT))
                    .clicked()
                {
                    app.open_projector(picked);
                }
            }
            Some(_) => {
                if ui.button(&s.projector_off).clicked() {
                    app.stop_projector();
                } else if picked != current {
                    app.open_projector(picked);
                }
            }
        }
    });
}
