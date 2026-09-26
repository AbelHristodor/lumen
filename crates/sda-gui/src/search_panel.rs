use crate::app::App;
use crate::style;

/// Search box with hymn actions above the virtualized result list.
pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let s = app.core.strings.clone();
    let resp = ui.add(
        egui::TextEdit::singleline(&mut app.ui.query)
            .desired_width(f32::INFINITY)
            .hint_text(&s.search_placeholder)
            .id(egui::Id::new("search")),
    );
    if app.ui.focus_search {
        resp.request_focus();
        app.ui.focus_search = false;
    }
    ui.horizontal_wrapped(|ui| {
        let has = app.selected_entry().is_some();
        if ui
            .add_enabled(has, egui::Button::new(&s.go_live).fill(style::ACCENT))
            .clicked()
        {
            let p = app.ui.preview_slide;
            app.go_live(p);
        }
        if ui
            .add_enabled(has, egui::Button::new(&s.open_in_powerpoint))
            .clicked()
        {
            if let Some(e) = app.selected_entry() {
                if let Err(err) = open::that_detached(&e.path) {
                    log::warn!("opening {} failed: {err}", e.path.display());
                }
            }
        }
        if ui
            .add_enabled(has, egui::Button::new(&s.reveal_in_folder))
            .clicked()
        {
            if let Some(dir) = app.selected_entry().and_then(|e| e.path.parent()) {
                if let Err(err) = open::that_detached(dir) {
                    log::warn!("revealing {} failed: {err}", dir.display());
                }
            }
        }
    });
    ui.separator();
    let spacing = ui.spacing();
    let row_h = (ui.text_style_height(&egui::TextStyle::Body) + 2.0 * spacing.button_padding.y)
        .max(spacing.interact_size.y);
    let stride = row_h + spacing.item_spacing.y;
    let total = app.ui.hits.len();
    let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]);
    if app.ui.scroll_to_selected {
        if let Some(sel) = app.ui.selected {
            area = area.vertical_scroll_offset(
                (sel as f32 * stride - ui.available_height() / 2.0).max(0.0),
            );
        }
        app.ui.scroll_to_selected = false;
    }
    area.show_rows(ui, row_h, total, |ui, range| {
        let mut clicked = None;
        let mut double = None;
        for row in range {
            let idx = app.ui.hits[row];
            let label = app
                .core
                .searcher
                .as_ref()
                .and_then(|searcher| searcher.label(idx))
                .unwrap_or("");
            let r =
                ui.add(egui::Button::selectable(app.ui.selected == Some(row), label).truncate());
            if r.clicked() {
                clicked = Some(row);
            }
            if r.double_clicked() {
                double = Some(row);
            }
        }
        if let Some(row) = clicked {
            app.ui.selected = Some(row);
            app.ui.preview_slide = 0;
        }
        if let Some(row) = double {
            app.ui.selected = Some(row);
            app.go_live(0);
        }
    });
}
