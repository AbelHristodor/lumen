use sda_core::library;
use sda_core::theme::{store, Font, FontSize, HAlign, Theme, VAlign};

use crate::app::App;
use crate::{slide, style};

/// Draw the theme picker and the draft editor with its live preview.
pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let s = app.core.strings.clone();
    egui::ScrollArea::horizontal()
        .id_salt("thumbs")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, t) in app.core.themes.iter().enumerate() {
                    let (rect, r) =
                        ui.allocate_exact_size(egui::vec2(160.0, 90.0), egui::Sense::click());
                    slide::paint(
                        ui.painter(),
                        rect,
                        t,
                        &s.theme_preview_sample,
                        None,
                        &mut app.ui.fit,
                    );
                    if app.ui.draft_index == i {
                        ui.painter().rect_stroke(
                            rect,
                            8.0,
                            egui::Stroke::new(2.0, style::ACCENT),
                            egui::StrokeKind::Outside,
                        );
                    }
                    let name = if t.name == app.core.active_theme.name {
                        format!("{} {}", t.name, s.active_suffix)
                    } else {
                        t.name.clone()
                    };
                    ui.painter().text(
                        rect.left_bottom() + egui::vec2(6.0, -4.0),
                        egui::Align2::LEFT_BOTTOM,
                        name,
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );
                    if r.clicked() {
                        app.ui.draft = t.clone();
                        app.ui.draft_index = i;
                    }
                }
                if ui.button(&s.theme_new).clicked() {
                    app.ui.draft = Theme {
                        name: "Custom".into(),
                        ..Theme::default()
                    };
                    app.ui.draft_index = usize::MAX;
                }
            });
        });
    let d = &mut app.ui.draft;
    egui::Grid::new("theme_form")
        .num_columns(4)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label(&s.theme_name_label);
            ui.text_edit_singleline(&mut d.name);
            ui.label(&s.theme_font_label);
            egui::ComboBox::from_id_salt("font")
                .selected_text(d.font.label())
                .show_ui(ui, |ui| {
                    for f in Font::all() {
                        ui.selectable_value(&mut d.font, f, f.label());
                    }
                });
            ui.end_row();
            ui.label(&s.theme_size_label);
            ui.horizontal(|ui| {
                let mut auto = d.size == FontSize::Auto;
                if ui.checkbox(&mut auto, &s.theme_auto_size).changed() {
                    d.size = if auto {
                        FontSize::Auto
                    } else {
                        FontSize::Pt(44.0)
                    };
                }
                if let FontSize::Pt(p) = &mut d.size {
                    ui.add(egui::Slider::new(p, 12.0..=160.0));
                }
            });
            ui.label(&s.theme_bold_label);
            ui.checkbox(&mut d.bold, "");
            ui.end_row();
            ui.label(&s.theme_text_color_label);
            let mut c = style::color32(d.text_color);
            if ui.color_edit_button_srgba(&mut c).changed() {
                d.text_color = style::rgba(c);
            }
            ui.label(&s.theme_bg_color_label);
            let mut b = style::color32(d.bg_color);
            if ui.color_edit_button_srgba(&mut b).changed() {
                d.bg_color = style::rgba(b);
            }
            ui.end_row();
            ui.label(&s.theme_align_label);
            ui.horizontal(|ui| {
                for (a, l) in [
                    (HAlign::Left, "⬅"),
                    (HAlign::Center, "⬌"),
                    (HAlign::Right, "➡"),
                ] {
                    ui.selectable_value(&mut d.h_align, a, l);
                }
            });
            ui.label(&s.theme_valign_label);
            ui.horizontal(|ui| {
                for (a, l) in [
                    (VAlign::Top, "⬆"),
                    (VAlign::Middle, "⬍"),
                    (VAlign::Bottom, "⬇"),
                ] {
                    ui.selectable_value(&mut d.v_align, a, l);
                }
            });
            ui.end_row();
            ui.label(&s.theme_line_spacing_label);
            ui.add(egui::Slider::new(&mut d.line_spacing, 1.0..=2.0));
            ui.label(&s.theme_margin_label);
            ui.add(egui::Slider::new(&mut d.margin, 0.02..=0.2));
            ui.end_row();
            ui.label(&s.theme_footer_label);
            ui.checkbox(&mut d.footer, "");
            ui.end_row();
        });
    let w = ui.available_width().min(640.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, w * 9.0 / 16.0), egui::Sense::hover());
    slide::paint(
        ui.painter(),
        rect,
        &app.ui.draft,
        &s.theme_preview_sample,
        Some("150 · Plecaţi-vă"),
        &mut app.ui.fit,
    );
    ui.horizontal(|ui| {
        let builtin = app.ui.draft.is_builtin_default();
        if ui
            .add_enabled(!builtin, egui::Button::new(&s.theme_save))
            .clicked()
        {
            if app.ui.draft.name.trim().is_empty() {
                app.ui.draft.name = "Custom".into();
            }
            if let Some(dir) = library::themes_dir() {
                if let Err(e) = store::save_theme(&dir, &app.ui.draft) {
                    log::warn!("save theme: {e}");
                }
            }
            app.core.reload_themes();
            app.ui.draft_index = app
                .core
                .themes
                .iter()
                .position(|t| t.name == app.ui.draft.name)
                .unwrap_or(0);
            if app.core.active_theme.name == app.ui.draft.name {
                let d = app.ui.draft.clone();
                app.core.set_active_theme(d);
            }
        }
        if ui
            .add_enabled(!builtin, egui::Button::new(&s.theme_delete))
            .clicked()
        {
            let name = app.ui.draft.name.clone();
            if let Some(dir) = library::themes_dir() {
                if let Err(e) = store::delete_theme(&dir, &name) {
                    log::warn!("delete theme: {e}");
                }
            }
            app.core.reload_themes();
            if app.core.active_theme.name == name {
                app.core.set_active_theme(Theme::default());
            }
            app.ui.draft = Theme::default();
            app.ui.draft_index = 0;
        }
        if ui
            .add(egui::Button::new(&s.theme_set_active).fill(style::ACCENT))
            .clicked()
        {
            let d = app.ui.draft.clone();
            app.core.set_active_theme(d);
        }
    });
}
