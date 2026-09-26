//! Video Downloader tab.

use crate::state::{App, Msg};
use crate::{AppWindow, DownloadState};
use hymnal_core::downloader::{self, DownloadEvent};
use log::{info, warn};
use slint::ComponentHandle;
use std::path::PathBuf;

/// Apply one worker event to the `download` UI struct.
pub fn apply_event(ui: &AppWindow, app: &App, ev: DownloadEvent) {
    let mut s = ui.get_download();
    match ev {
        DownloadEvent::Resolving => s.status = "resolving".into(),
        DownloadEvent::Title(t) => s.title = t.into(),
        DownloadEvent::Progress(p) => {
            s.status = "downloading".into();
            s.percent = p.percent;
            s.speed = p.speed.into();
            s.eta = p.eta.into();
        }
        DownloadEvent::Done { .. } => {
            s.status = "done".into();
            s.percent = 100.0;
            app.download_in_flight.set(false);
        }
        DownloadEvent::Failed { message } => {
            s.status = "failed".into();
            s.message = message.into();
            app.download_in_flight.set(false);
        }
    }
    ui.set_download(s);
}

pub fn wire(ui: &AppWindow, app: &App) {
    ui.set_download_dir(
        hymnal_core::library::downloads_dir(&app.cfg.borrow())
            .to_string_lossy()
            .to_string()
            .into(),
    );

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_choose_folder(move || {
        let Some(ui) = weak.upgrade() else { return };
        let start = ui.get_download_dir().to_string();
        let start = if start.is_empty() { ".".to_string() } else { start };
        if let Some(folder) = rfd::FileDialog::new().set_directory(start).pick_folder() {
            let s = folder.to_string_lossy().to_string();
            ui.set_download_dir(s.clone().into());
            a.update_cfg(|c| c.download_dir = Some(s));
        }
    });

    let (a, weak) = (app.clone(), ui.as_weak());
    ui.on_start_download(move || {
        let Some(ui) = weak.upgrade() else { return };
        let url = ui.get_download_url().to_string();
        if url.trim().is_empty() || a.download_in_flight.get() {
            return;
        }
        a.download_in_flight.set(true);
        let dir = PathBuf::from(ui.get_download_dir().to_string());
        info!("starting download: {url} -> {}", dir.display());
        ui.set_download(DownloadState {
            status: "resolving".into(),
            ..Default::default()
        });
        let tx = a.tx.clone();
        std::thread::spawn(move || {
            downloader::download(&url, &dir, |ev| {
                let _ = tx.send(Msg::Download(ev));
            });
        });
    });

    let weak = ui.as_weak();
    ui.on_reveal_download(move || {
        let Some(ui) = weak.upgrade() else { return };
        let dir = ui.get_download_dir().to_string();
        if !dir.is_empty() {
            if let Err(e) = open::that(&dir) {
                warn!("failed to reveal {dir}: {e}");
            }
        }
    });
}
