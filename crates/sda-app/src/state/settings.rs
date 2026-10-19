//! Settings tab state. Owned by the Settings tab task.

use crate::app::App;
use crate::msg::settings::Section;
use crate::msg::{Msg, SettingsMsg};
use crate::state::projector;
use crate::workers;
use iced::Task;
use sda_core::library;
use std::path::PathBuf;

#[derive(Default)]
pub struct SettingsState {
    pub section: Section,
    /// Why the last folder add was refused, shown under the table.
    pub folder_error: Option<String>,
}

/// Where the app keeps config, hymns, themes and Bibles.
pub fn data_dir(app: &App) -> PathBuf {
    app.core
        .cfg_path
        .as_deref()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .or_else(|| library::default_library_dir().and_then(|d| d.parent().map(PathBuf::from)))
        .unwrap_or_else(std::env::temp_dir)
}

/// Codes of every `*.json` in the Bibles dir except the index, sorted.
/// The Bible tab keeps its own copy, this one avoids a cross tab import.
pub fn installed_bibles(dir: &std::path::Path) -> Vec<String> {
    let mut codes: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                .filter(|s| s != "index")
                .collect()
        })
        .unwrap_or_default();
    codes.sort();
    codes
}

fn pick_folder() -> Task<Msg> {
    Task::perform(
        async {
            rfd::AsyncFileDialog::new()
                .pick_folder()
                .await
                .map(|h| h.path().to_path_buf())
        },
        |p| Msg::Settings(SettingsMsg::FolderPicked(p)),
    )
}

/// Persist the folder list and rebuild the index.
fn folders_changed(app: &mut App) -> Task<Msg> {
    app.core.save_cfg();
    app.status.library = app.core.strings.status_indexing.clone();
    workers::reindex(&mut app.core)
}

pub fn update(app: &mut App, msg: SettingsMsg) -> Task<Msg> {
    match msg {
        SettingsMsg::Noop => {}
        SettingsMsg::Section(s) => {
            app.settings.section = s;
            if s == Section::Display {
                return projector::refresh_displays();
            }
        }
        SettingsMsg::Language(lang) => app.core.set_language(lang),
        SettingsMsg::CheckUpdate => {
            app.status.update = Some(app.core.strings.update_checking.clone());
            return workers::update_check(&app.core);
        }
        SettingsMsg::ForceSync => return Task::done(Msg::ForceSync),
        SettingsMsg::AddFolder => return pick_folder(),
        SettingsMsg::FolderPicked(None) => {}
        SettingsMsg::FolderPicked(Some(path)) => {
            match library::add_user_library(&mut app.core.cfg, &path) {
                Ok(()) => {
                    app.settings.folder_error = None;
                    return folders_changed(app);
                }
                Err(e) => app.settings.folder_error = Some(e.to_string()),
            }
        }
        SettingsMsg::RemoveFolder(path) => {
            library::remove_user_library(&mut app.core.cfg, &path);
            return folders_changed(app);
        }
        SettingsMsg::EnableFolder(path, on) => {
            library::set_library_enabled(&mut app.core.cfg, &path, on);
            return folders_changed(app);
        }
        SettingsMsg::DefaultDisplay(i) => {
            app.projector_display = Some(i);
            app.core.cfg.output_display = Some(i as i32);
            app.core.save_cfg();
        }
        SettingsMsg::Identify => return Task::done(Msg::Identify),
        SettingsMsg::DetectConverter => return workers::converter_detect(),
        SettingsMsg::Detected(converter) => app.core.converter = converter,
        SettingsMsg::BibleDefault(code) => {
            app.core.cfg.bible_version = Some(code);
            app.core.save_cfg();
        }
        SettingsMsg::RevealData => {
            if let Err(e) = open::that_detached(data_dir(app)) {
                log::warn!("opening the data folder failed: {e}");
            }
        }
    }
    Task::none()
}
