use crate::app::App;

/// Draw this window.
pub fn show(ctx: &egui::Context, app: &mut App) {
    // SEAM
    let _ = (
        &app.ui.video.url,
        &app.ui.video.dir,
        crate::core::Core::spawn_download,
    );
    if !app.ui.videos_open {
        return;
    }
    egui::Window::new("Videos")
        .open(&mut app.ui.videos_open)
        .show(ctx, |ui| {
            ui.label("SEAM");
        });
}
