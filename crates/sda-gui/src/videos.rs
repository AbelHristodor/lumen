use crate::app::{App, VideoPhase};
use crate::style;

/// Draw the video downloader window with its URL, target folder and progress.
pub fn show(ctx: &egui::Context, app: &mut App) {
    if !app.ui.videos_open {
        return;
    }
    let s = app.core.strings.clone();
    let mut open = app.ui.videos_open;
    egui::Window::new(&s.videos)
        .id(egui::Id::new("videos_window"))
        .open(&mut open)
        .resizable(true)
        .default_size([520.0, 260.0])
        .show(ctx, |ui| {
            ui.label(&s.downloader_subtitle);
            ui.add(
                egui::TextEdit::singleline(&mut app.ui.video.url)
                    .desired_width(f32::INFINITY)
                    .hint_text(&s.downloader_url_placeholder),
            );
            ui.horizontal(|ui| {
                ui.weak(app.ui.video.dir.display().to_string());
                if ui.button(&s.choose).clicked() {
                    if let Some(f) = rfd::FileDialog::new()
                        .set_directory(&app.ui.video.dir)
                        .pick_folder()
                    {
                        app.core.cfg.download_dir = Some(f.to_string_lossy().into_owned());
                        app.ui.video.dir = f;
                        app.core.save_cfg();
                    }
                }
            });
            let busy = matches!(
                app.ui.video.phase,
                VideoPhase::Resolving | VideoPhase::Downloading
            );
            ui.horizontal(|ui| {
                let ready = !busy && !app.ui.video.url.trim().is_empty();
                if ui
                    .add_enabled(
                        ready,
                        egui::Button::new(&s.download_button).fill(style::ACCENT),
                    )
                    .clicked()
                {
                    let v = &mut app.ui.video;
                    v.phase = VideoPhase::Resolving;
                    v.percent = 0.0;
                    v.title.clear();
                    v.message.clear();
                    let (url, dir) = (v.url.trim().to_owned(), v.dir.clone());
                    app.core
                        .spawn_download(url, dir, app.tx.clone(), ctx.clone());
                }
                if ui.button(&s.reveal_in_folder).clicked() {
                    if let Err(e) = open::that(&app.ui.video.dir) {
                        log::warn!("revealing the download folder failed: {e}");
                    }
                }
            });
            let v = &app.ui.video;
            if !v.title.is_empty() {
                ui.label(&v.title);
            }
            match v.phase {
                VideoPhase::Idle => {}
                VideoPhase::Resolving => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(&s.setting_up_downloader);
                    });
                }
                VideoPhase::Downloading => {
                    ui.add(
                        egui::ProgressBar::new(v.percent / 100.0)
                            .text(format!("{:.1}%  {}  {}", v.percent, v.speed, v.eta)),
                    );
                }
                VideoPhase::Done => {
                    ui.colored_label(style::ACCENT, &s.download_complete);
                }
                VideoPhase::Failed => {
                    ui.colored_label(
                        egui::Color32::LIGHT_RED,
                        format!("{}{}", s.download_failed_prefix, v.message),
                    );
                }
            }
        });
    app.ui.videos_open = open;
}
