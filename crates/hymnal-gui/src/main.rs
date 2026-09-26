// On Windows, don't allocate a console window for this GUI app. Only affects
// release builds (debug keeps the console so `RUST_LOG` output is visible);
// no-op on macOS/Linux.
#![windows_subsystem = "windows"]

slint::include_modules!();

mod control;
mod downloads;
mod fonts;
mod i18n;
mod library;
mod projector;
mod settings;
mod state;
mod themes;

use hymnal_core::i18n::Strings;
use hymnal_core::refresh;
use hymnal_core::search::Searcher;
use hymnal_core::sync::SyncOutcome;
use hymnal_core::update::{self, UpdateOutcome};
use log::{debug, info, warn};
use slint::ComponentHandle;
use state::{App, Msg};
use std::sync::mpsc::Receiver;

/// Index from disk first so search is usable at once, then sync the hymns zip
/// and re-index only if it changed, then check for an app update.
fn spawn_boot(ui: &AppWindow, app: &App) {
    let cfg = app.cfg.borrow().clone();
    let strings: Strings = app.strings.borrow().clone();
    let tx = app.tx.clone();
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let set_status = |s: String| {
            let _ = weak.upgrade_in_event_loop(move |ui| ui.set_status(s.into()));
        };
        if refresh::default_library_present() {
            let _ = tx.send(Msg::Index(Searcher::new(refresh::load_local(cfg.clone()))));
            set_status(strings.status_library_ready.clone());
            if refresh::sync_default(&cfg) == Some(SyncOutcome::Downloaded) {
                info!("hymns updated; re-indexing");
                let _ = tx.send(Msg::Index(Searcher::new(refresh::load_local(cfg))));
            }
        } else {
            let _ = tx.send(Msg::Index(Searcher::new(refresh::load_library(cfg, false))));
            set_status(strings.status_library_ready.clone());
        }

        let msg = match update::check_and_stage_update() {
            Ok(UpdateOutcome::UpToDate) => strings.update_up_to_date,
            Ok(UpdateOutcome::Updated { version }) => strings.update_staged_fmt.replace("{}", &version),
            Ok(UpdateOutcome::CheckUnavailable) => {
                debug!("update check unavailable (offline or rate-limited)");
                strings.update_unavailable
            }
            Err(e) => {
                warn!("update check failed: {e}");
                strings.update_failed
            }
        };
        let _ = weak.upgrade_in_event_loop(move |ui| ui.set_update_status(msg.into()));
    });
}

/// Apply every pending worker message. Runs on the UI thread from the timer.
fn drain(ui: &AppWindow, app: &App, rx: &Receiver<Msg>) {
    while let Ok(msg) = rx.try_recv() {
        let strings = app.strings.borrow();
        match msg {
            Msg::Index(s) => {
                info!("searcher ready with {} hymns", s.len());
                ui.set_status(strings.status_library_ready.clone().into());
                drop(strings);
                app.install_searcher(ui, s);
            }
            Msg::Synced(result) => {
                ui.set_syncing(false);
                match result {
                    Ok(s) => {
                        ui.set_sync_status(strings.status_synced_fmt.replace("{}", &s.len().to_string()).into());
                        drop(strings);
                        app.install_searcher(ui, s);
                    }
                    Err(e) => ui.set_sync_status(strings.status_sync_failed_fmt.replace("{}", &e).into()),
                }
            }
            Msg::Reindexed(result) => match result {
                Ok(s) => {
                    ui.set_library_status(strings.status_indexed_fmt.replacen("{}", &s.len().to_string(), 1).into());
                    drop(strings);
                    app.install_searcher(ui, s);
                }
                Err(e) => ui.set_library_status(strings.status_indexing_failed_fmt.replacen("{}", &e, 1).into()),
            },
            Msg::Download(ev) => {
                drop(strings);
                downloads::apply_event(ui, app, ev);
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("hymnal_gui=info,hymnal_core=info"),
    )
    .init();
    info!("starting SDA Manager");

    let ui = AppWindow::new()?;
    let (app, rx) = App::load();
    *app.strings.borrow_mut() = i18n::apply_language(&ui, app.language());
    ui.set_status(app.strings.borrow().status_loading.clone().into());

    library::wire(&ui, &app);
    downloads::wire(&ui, &app);
    settings::wire(&ui, &app);
    themes::wire(&ui, &app);
    control::wire(&ui, &app);

    spawn_boot(&ui, &app);

    // Keep the timer alive for the lifetime of the app; dropping it stops polling.
    let timer = slint::Timer::default();
    let (weak, app_timer) = (ui.as_weak(), app.clone());
    timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(200), move || {
        if let Some(ui) = weak.upgrade() {
            drain(&ui, &app_timer, &rx);
        }
    });

    ui.run()?;
    Ok(())
}
