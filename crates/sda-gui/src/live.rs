use crate::app::App;

/// Draw this panel.
pub fn show(ui: &mut egui::Ui, app: &mut App) {
    // SEAM
    let _ = (
        app.displays.first().map(|d| &d.label),
        crate::projector::default_display,
        App::open_projector,
    );
    ui.label("SEAM");
}
