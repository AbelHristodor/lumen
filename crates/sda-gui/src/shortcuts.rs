use egui::{Key, Modifiers};

use crate::app::App;

/// Apply the keyboard table to `app` using the input of the viewport that owns `ui`.
pub fn handle(ui: &egui::Ui, app: &mut App) {
    let search_focused = ui.memory(|m| m.focused()) == Some(egui::Id::new("search"));
    let typing = ui.ctx().text_edit_focused();
    let pressed = |k: Key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, k));
    if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::K)) {
        app.ui.focus_search = true;
    }
    if pressed(Key::Escape) {
        app.stop_projector();
    }
    if search_focused || !typing {
        if pressed(Key::ArrowDown) {
            app.move_selection(1);
        }
        if pressed(Key::ArrowUp) {
            app.move_selection(-1);
        }
    }
    if search_focused && pressed(Key::Enter) {
        let s = app.ui.preview_slide;
        app.go_live(s);
    }
    if typing {
        return;
    }
    if pressed(Key::ArrowRight) || pressed(Key::Space) {
        app.live_next();
    }
    if pressed(Key::ArrowLeft) {
        app.live_prev();
    }
    if pressed(Key::B) {
        app.live_blank();
    }
    let nums = [
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
        Key::Num7,
        Key::Num8,
        Key::Num9,
    ];
    for (n, k) in nums.into_iter().enumerate() {
        if pressed(k) {
            app.live_jump(n + 1);
        }
    }
}
