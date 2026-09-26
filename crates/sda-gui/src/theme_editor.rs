use crate::app::App;

/// Draw this panel.
pub fn show(ui: &mut egui::Ui, app: &mut App) {
    // SEAM
    let _ = (
        &app.ui.draft,
        app.ui.draft_index,
        crate::style::rgba,
        crate::core::Core::set_active_theme,
    );
    ui.label("SEAM");
}
