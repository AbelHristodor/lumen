//! Settings tab: force sync, UI language, user library folders.

use crate::state::{App, Msg};
use crate::{AppWindow, LibraryRow};
use hymnal_core::i18n::Language;
use hymnal_core::library::{self, Config};
use hymnal_core::refresh;
use hymnal_core::search::Searcher;
use slint::{ComponentHandle, ModelRc, VecModel};
use std::rc::Rc;

fn library_rows(cfg: &Config) -> ModelRc<LibraryRow> {
    let rows: Vec<LibraryRow> = cfg
        .libraries
        .iter()
        .map(|l| LibraryRow {
            name: l.name.clone().into(),
            path: l.path.clone().into(),
            enabled: l.enabled,
            removable: !l.managed_by_git,
            available: library::library_available(&l.path),
        })
        .collect();
    ModelRc::from(Rc::new(VecModel::from(rows)))
}

/// Persist the edited library list, refresh the rows and re-index off thread.
fn libraries_changed(ui: &AppWindow, app: &App) {
    app.save_cfg();
    ui.set_libraries(library_rows(&app.cfg.borrow()));
    ui.set_library_status(app.strings.borrow().status_indexing.clone().into());
    let cfg = app.cfg.borrow().clone();
    let tx = app.tx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(Msg::Reindexed(Ok(Searcher::new(refresh::load_local(cfg)))));
    });
}

pub fn wire(ui: &AppWindow, app: &App) {
    ui.set_app_version(env!("CARGO_PKG_VERSION").into());
    ui.set_update_status(app.strings.borrow().update_checking.clone().into());
    ui.set_sync_status("".into());
    ui.set_library_status("".into());
    ui.set_libraries(library_rows(&app.cfg.borrow()));
    ui.set_active_language(app.language().code().into());

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_force_sync(move || {
        let Some(ui) = weak.upgrade() else { return };
        if ui.get_syncing() {
            return;
        }
        ui.set_syncing(true);
        ui.set_sync_status(a.strings.borrow().status_re_cloning.clone().into());
        let cfg = a.cfg.borrow().clone();
        let tx = a.tx.clone();
        std::thread::spawn(move || {
            let result = match refresh::force_clean() {
                Ok(()) => Ok(Searcher::new(refresh::load_library(cfg, true))),
                Err(e) => Err(format!("force clean failed: {e}")),
            };
            let _ = tx.send(Msg::Synced(result));
        });
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_set_language(move |code| {
        let Some(ui) = weak.upgrade() else { return };
        let lang = Language::from_code(&code);
        *a.strings.borrow_mut() = crate::i18n::apply_language(&ui, lang);
        ui.set_active_language(lang.code().into());
        a.update_cfg(|c| c.language = Some(lang.code().to_string()));
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_add_library(move || {
        let Some(ui) = weak.upgrade() else { return };
        let Some(folder) = rfd::FileDialog::new().pick_folder() else { return };
        let added = library::add_user_library(&mut a.cfg.borrow_mut(), &folder);
        match added {
            Ok(()) => libraries_changed(&ui, &a),
            Err(e) => ui.set_library_status(e.to_string().into()),
        }
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_remove_library(move |path| {
        let Some(ui) = weak.upgrade() else { return };
        library::remove_user_library(&mut a.cfg.borrow_mut(), &path);
        libraries_changed(&ui, &a);
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_set_library_enabled(move |path, enabled| {
        let Some(ui) = weak.upgrade() else { return };
        library::set_library_enabled(&mut a.cfg.borrow_mut(), &path, enabled);
        libraries_changed(&ui, &a);
    });
}
