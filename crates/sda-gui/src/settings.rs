use sda_core::i18n::Language;
use sda_core::library;

use crate::app::App;
use crate::theme_editor;

/// Draw the settings window with language, library sync, user folders and themes.
pub fn show(ctx: &egui::Context, app: &mut App) {
    if !app.ui.settings_open {
        return;
    }
    let s = app.core.strings.clone();
    let mut open = app.ui.settings_open;
    egui::Window::new(&s.settings)
        .id(egui::Id::new("settings_window"))
        .open(&mut open)
        .resizable(true)
        .default_size([720.0, 600.0])
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label(format!("{}{}", s.version_prefix, env!("CARGO_PKG_VERSION")));
                ui.label(&app.status.update);
                ui.separator();
                ui.heading(&s.language_heading);
                let mut lang = app.core.language;
                ui.horizontal(|ui| {
                    for l in Language::all() {
                        ui.selectable_value(&mut lang, l, l.label());
                    }
                });
                if lang != app.core.language {
                    app.core.set_language(lang);
                }
                ui.separator();
                ui.heading(&s.library_heading);
                ui.label(&s.force_sync_description);
                let label = if app.status.syncing {
                    &s.syncing_button
                } else {
                    &s.force_sync_button
                };
                if ui
                    .add_enabled(!app.status.syncing, egui::Button::new(label))
                    .clicked()
                {
                    app.status.syncing = true;
                    app.status.sync = s.status_re_cloning.clone();
                    app.core.spawn_force_sync(app.tx.clone(), ctx.clone());
                }
                ui.label(&app.status.sync);
                ui.separator();
                ui.heading(&s.user_libraries_heading);
                ui.label(&s.user_libraries_description);
                let mut changed = false;
                let mut remove: Option<String> = None;
                for lib in &mut app.core.cfg.libraries {
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut lib.enabled, &lib.name).changed() {
                            changed = true;
                        }
                        if !library::library_available(&lib.path) {
                            ui.weak(&s.library_unavailable_suffix);
                        }
                        ui.weak(&lib.path);
                        if !lib.managed_by_git && ui.small_button(&s.remove).clicked() {
                            remove = Some(lib.path.clone());
                        }
                    });
                }
                if let Some(p) = remove {
                    library::remove_user_library(&mut app.core.cfg, &p);
                    changed = true;
                }
                if ui.button(&s.add_folder_button).clicked() {
                    if let Some(f) = rfd::FileDialog::new().pick_folder() {
                        match library::add_user_library(&mut app.core.cfg, &f) {
                            Ok(()) => changed = true,
                            Err(e) => app.ui.library_status = e.to_string(),
                        }
                    }
                }
                if changed {
                    app.core.save_cfg();
                    app.ui.library_status = s.status_indexing.clone();
                    app.core.spawn_reindex(app.tx.clone(), ctx.clone());
                }
                ui.label(&app.ui.library_status);
                ui.separator();
                ui.heading(&s.themes_heading);
                theme_editor::show(ui, app);
            });
        });
    app.ui.settings_open = open;
}
