use crate::app::App;
use crate::{slide, style};

/// Draw the selected hymn slide by slide with step buttons and Go Live.
pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let s = app.core.strings.clone();
    ui.heading(&s.preview);
    let Some(e) = app.selected_entry().cloned() else {
        return;
    };
    let count = e.slides.len();
    if count == 0 {
        ui.label(s.slide_zero_fmt.replace("{}", &e.title));
        return;
    }
    app.ui.preview_slide = app.ui.preview_slide.min(count - 1);
    let idx = app.ui.preview_slide;
    let controls_h = 3.0 * ui.spacing().interact_size.y;
    let max_h = (ui.available_height() - controls_h).max(80.0);
    let w = ui.available_width().min(max_h * 16.0 / 9.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), w * 9.0 / 16.0), egui::Sense::hover());
    let rect = egui::Rect::from_center_size(rect.center(), egui::vec2(w, w * 9.0 / 16.0));
    let footer = slide::footer_text(e.number.as_deref(), &e.title);
    slide::paint(
        ui.painter(),
        rect,
        &app.core.active_theme,
        &e.slides[idx],
        Some(&footer),
        &mut app.ui.fit,
    );
    ui.horizontal(|ui| {
        if ui
            .add_enabled(idx > 0, egui::Button::new(&s.prev_slide))
            .clicked()
        {
            app.ui.preview_slide -= 1;
        }
        let titled = match &e.number {
            Some(n) => format!("{n}. {}", e.title),
            None => e.title.clone(),
        };
        ui.label(
            s.slide_counter_fmt
                .replacen("{}", &titled, 1)
                .replacen("{}", &(idx + 1).to_string(), 1)
                .replacen("{}", &count.to_string(), 1),
        );
        if ui
            .add_enabled(idx + 1 < count, egui::Button::new(&s.next_slide))
            .clicked()
        {
            app.ui.preview_slide += 1;
        }
        if ui
            .add(egui::Button::new(&s.go_live).fill(style::ACCENT))
            .clicked()
        {
            app.go_live(idx);
        }
    });
}
