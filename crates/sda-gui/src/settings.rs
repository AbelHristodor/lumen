use crate::app::App;

/// Draw this window.
pub fn show(ctx: &egui::Context, app: &mut App) {
    // SEAM
    let _ = (
        app.core.language,
        &app.tx,
        crate::core::Core::set_language,
        crate::core::Core::save_cfg,
        crate::core::Core::spawn_force_sync,
        crate::core::Core::spawn_reindex,
        crate::theme_editor::show,
    );
    if !app.ui.settings_open {
        return;
    }
    egui::Window::new("Settings")
        .open(&mut app.ui.settings_open)
        .show(ctx, |ui| {
            ui.label("SEAM");
        });
}
