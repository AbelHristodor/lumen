//! Videos tab state. Owned by the Videos tab task. Probe and download run
//! off the UI thread and come back as `VideosMsg` events.

use crate::app::App;
use crate::msg::{Msg, VideosMsg};
use iced::Task;
use iced::futures::channel::mpsc;
use iced::futures::{StreamExt, stream};
use iced::widget::image;
use sda_core::downloader::{self, DownloadEvent, DownloadRecord, Probe};
use sda_core::library;
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Phase {
    #[default]
    Idle,
    Probing,
    Ready,
    Downloading,
    Done(PathBuf),
    Failed,
}

#[derive(Default)]
pub struct VideosState {
    pub url: String,
    pub probe: Option<Probe>,
    pub format_id: Option<String>,
    /// Empty until first used, then the config folder or the OS Downloads.
    pub dir: PathBuf,
    pub phase: Phase,
    pub percent: f32,
    pub speed: String,
    pub eta: String,
    pub title: String,
    pub message: String,
    /// Decoded probe thumbnail. Empty until fetched or when the fetch failed.
    pub thumbnail: Option<image::Handle>,
}

/// The folder downloads go to. Falls back to the config when `dir` is unset.
pub fn dir(app: &App) -> PathBuf {
    if app.videos.dir.as_os_str().is_empty() {
        library::downloads_dir(&app.core.cfg)
    } else {
        app.videos.dir.clone()
    }
}

pub fn update(app: &mut App, msg: VideosMsg) -> Task<Msg> {
    let v = &mut app.videos;
    match msg {
        VideosMsg::Url(u) => v.url = u,
        VideosMsg::Fetch => {
            let url = v.url.trim().to_string();
            // A probe or a download is still running. Starting another probe
            // would clear the state the running job reports into.
            if url.is_empty() || matches!(v.phase, Phase::Probing | Phase::Downloading) {
                return Task::none();
            }
            v.phase = Phase::Probing;
            v.probe = None;
            v.format_id = None;
            v.thumbnail = None;
            v.message.clear();
            return probe(url);
        }
        VideosMsg::Probed(Ok(p)) => {
            v.format_id = p.qualities.first().map(|q| q.format_id.clone());
            v.title = p.title.clone();
            let thumb = p.thumbnail.clone();
            v.probe = Some(p);
            v.phase = Phase::Ready;
            if let Some(url) = thumb {
                return thumbnail(url);
            }
        }
        VideosMsg::Thumbnail(bytes) => {
            // Drop a late thumbnail when the probe it belongs to is gone.
            if v.probe.is_some() {
                v.thumbnail = bytes.map(image::Handle::from_bytes);
            }
        }
        VideosMsg::Probed(Err(e)) => {
            v.phase = Phase::Failed;
            v.message = format!("{}{e}", app.core.strings.download_failed_prefix);
        }
        VideosMsg::Quality(id) => v.format_id = Some(id),
        VideosMsg::ChooseFolder => {
            let start = dir(app);
            return Task::perform(
                async move {
                    rfd::AsyncFileDialog::new()
                        .set_directory(start)
                        .pick_folder()
                        .await
                        .map(|h| h.path().to_path_buf())
                },
                |p| Msg::Videos(VideosMsg::FolderChosen(p)),
            );
        }
        VideosMsg::FolderChosen(Some(p)) => {
            v.dir = p.clone();
            app.core.cfg.download_dir = Some(p.to_string_lossy().into_owned());
            app.core.save_cfg();
        }
        VideosMsg::FolderChosen(None) => {}
        VideosMsg::Download => {
            if v.probe.is_none() || v.phase == Phase::Downloading {
                return Task::none();
            }
            v.phase = Phase::Downloading;
            v.percent = 0.0;
            v.speed.clear();
            v.eta.clear();
            v.message.clear();
            let url = v.url.trim().to_string();
            let format_id = v.format_id.clone();
            return download(url, format_id, dir(app));
        }
        VideosMsg::Event(ev) => match ev {
            DownloadEvent::Resolving => {
                v.phase = Phase::Downloading;
                v.message = app.core.strings.setting_up_downloader.clone();
            }
            DownloadEvent::Title(t) => v.title = t,
            DownloadEvent::Progress(p) => {
                v.message.clear();
                v.percent = p.percent;
                v.speed = p.speed;
                v.eta = p.eta;
                if !p.title.is_empty() {
                    v.title = p.title;
                }
            }
            DownloadEvent::Done { path } => {
                v.percent = 100.0;
                v.phase = Phase::Done(path.clone());
                v.message = app.core.strings.download_complete.clone();
                let at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs() as i64);
                // One row per file. A repeat download moves it to the top.
                app.core
                    .downloads
                    .retain(|r| r.path != path || r.title != v.title);
                app.core.downloads.insert(
                    0,
                    DownloadRecord {
                        title: v.title.clone(),
                        path,
                        at,
                    },
                );
                app.core.save_downloads();
            }
            DownloadEvent::Failed { message } => {
                v.phase = Phase::Failed;
                v.message = format!("{}{message}", app.core.strings.download_failed_prefix);
            }
        },
        VideosMsg::Ended => {
            // The worker went away without a Done or a Failed. Never leave the
            // tab stuck on Downloading.
            if v.phase == Phase::Downloading {
                v.phase = Phase::Failed;
                v.message = format!("{}yt-dlp", app.core.strings.download_failed_prefix);
            }
        }
        VideosMsg::Reveal(p) => {
            let target = if p.is_file() {
                p.parent().map(PathBuf::from).unwrap_or(p)
            } else {
                p
            };
            if let Err(e) = open::that_detached(&target) {
                log::warn!("reveal failed for {}: {e}", target.display());
            }
        }
    }
    Task::none()
}

/// Ask yt-dlp for the video metadata on a blocking thread.
fn probe(url: String) -> Task<Msg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || downloader::probe(&url).map_err(|e| e.to_string()))
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r)
        },
        |r| Msg::Videos(VideosMsg::Probed(r)),
    )
}

/// Fetch the thumbnail bytes on a blocking thread. Any failure yields `None`.
fn thumbnail(url: String) -> Task<Msg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let mut bytes = Vec::new();
                ureq::get(&url)
                    .call()
                    .ok()?
                    .into_reader()
                    .read_to_end(&mut bytes)
                    .ok()?;
                Some(bytes)
            })
            .await
            .ok()
            .flatten()
        },
        |b| Msg::Videos(VideosMsg::Thumbnail(b)),
    )
}

/// Run yt-dlp on its own thread and stream every event into the app. A panic
/// on the worker becomes a Failed event, and the end of the stream becomes
/// `Ended` so the tab never waits on a dead worker.
fn download(url: String, format_id: Option<String>, dir: PathBuf) -> Task<Msg> {
    let (tx, rx) = mpsc::unbounded();
    std::thread::spawn(move || {
        let emit = |ev| {
            let _ = tx.unbounded_send(ev);
        };
        let run = std::panic::AssertUnwindSafe(|| {
            downloader::start_download(&url, format_id.as_deref(), &dir, emit)
        });
        if let Err(payload) = std::panic::catch_unwind(run) {
            let message = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "panic".to_string());
            let _ = tx.unbounded_send(DownloadEvent::Failed { message });
        }
    });
    let events = rx
        .map(|ev| Msg::Videos(VideosMsg::Event(ev)))
        .chain(stream::once(async { Msg::Videos(VideosMsg::Ended) }));
    Task::run(events, |m| m)
}
