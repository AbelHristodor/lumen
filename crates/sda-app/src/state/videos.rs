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
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Download records kept, newest first.
pub const DOWNLOADS_CAP: usize = 50;

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
    /// The file chosen to present.
    pub picked: Option<PathBuf>,
    /// The preview player. `Present` hands this same `Arc` to the live state.
    pub player: Option<Arc<sda_media::Player>>,
    pub opening: bool,
    /// Why the last open failed, shown under the picker.
    pub error: Option<String>,
    /// Playback volume, 0.0 to 1.0, from the config.
    pub volume: f32,
    /// Seconds under the seek slider while it is dragged.
    pub scrub: Option<f32>,
}

impl Default for VideosState {
    fn default() -> Self {
        VideosState {
            url: String::new(),
            probe: None,
            format_id: None,
            dir: PathBuf::new(),
            phase: Phase::default(),
            percent: 0.0,
            speed: String::new(),
            eta: String::new(),
            title: String::new(),
            message: String::new(),
            thumbnail: None,
            picked: None,
            player: None,
            opening: false,
            error: None,
            volume: 1.0,
            scrub: None,
        }
    }
}

/// The largest picture the live output needs, the chosen display or 1080p.
fn projector_size(app: &App) -> (u32, u32) {
    app.projector_display
        .and_then(|i| app.displays.get(i))
        .map(|d| {
            let (_, _, w, h) = d.rect;
            ((w as f32 * d.scale) as u32, (h as f32 * d.scale) as u32)
        })
        .unwrap_or((1920, 1080))
}

/// True while the preview player plays. When it is also the live player the
/// live state keeps the frame clock on too, which does no harm.
pub fn preview_playing(app: &App) -> bool {
    app.videos
        .player
        .as_ref()
        .is_some_and(|p| p.state() == sda_media::State::Playing)
}

/// Per frame. The live state ticks the same player when it is live. A
/// second tick in one frame is harmless.
pub fn tick(app: &App) {
    if let Some(p) = &app.videos.player {
        p.tick();
    }
}

/// Let go of the preview player.
fn release_preview(app: &mut App) {
    // Live holds its own clone, so dropping here never stops a live video.
    app.videos.player = None;
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
                app.core.downloads.truncate(DOWNLOADS_CAP);
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
        VideosMsg::OpenFile => {
            let start = v
                .picked
                .as_ref()
                .and_then(|p| p.parent().map(Path::to_path_buf))
                .unwrap_or_else(|| dir(app));
            return Task::perform(
                async move {
                    rfd::AsyncFileDialog::new()
                        .set_directory(start)
                        .add_filter("Video", &["mp4", "mkv", "mov", "webm", "avi", "m4v"])
                        .pick_file()
                        .await
                        .map(|h| h.path().to_path_buf())
                },
                |p| Msg::Videos(VideosMsg::FileChosen(p)),
            );
        }
        VideosMsg::FileChosen(None) => {}
        VideosMsg::FileChosen(Some(path)) => {
            release_preview(app);
            let v = &mut app.videos;
            v.picked = Some(path.clone());
            v.error = None;
            v.scrub = None;
            v.opening = true;
            return crate::workers::open_video(path, projector_size(app));
        }
        // A result for an older pick is dropped. The newer pick is still opening.
        VideosMsg::Opened(Ok(player)) => {
            if v.picked.as_deref() == Some(player.file()) {
                v.opening = false;
                v.scrub = None;
                player.set_volume(v.volume);
                v.player = Some(player);
            }
        }
        VideosMsg::Opened(Err(e)) => {
            v.scrub = None;
            if v.opening {
                v.opening = false;
                v.error = Some(e);
            }
        }
        VideosMsg::Toggle => {
            v.scrub = None;
            if let Some(p) = &v.player {
                p.toggle();
            }
        }
        VideosMsg::Scrub(secs) => v.scrub = Some(secs),
        VideosMsg::Seek(secs) => {
            // The release value from the view can lag the last scrub in one batch.
            let target = v.scrub.take().unwrap_or(secs);
            if let Some(p) = &v.player {
                p.seek(Duration::from_secs_f32(target.max(0.0)));
            }
        }
        VideosMsg::Step(by) => {
            v.scrub = None;
            if let Some(p) = &v.player {
                p.step(by);
            }
        }
        VideosMsg::Volume(vol) => {
            v.volume = vol.clamp(0.0, 1.0);
            if let Some(p) = &v.player {
                p.set_volume(v.volume);
            }
            app.core.cfg.video_volume = v.volume;
            app.core.save_cfg();
        }
        VideosMsg::Present => {
            let player = v.player.clone();
            if let Some(p) = player {
                let title = p
                    .file()
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                app.live.go_video(p, title);
            }
        }
        VideosMsg::Reveal(p) => {
            if !p.exists() {
                app.core.prune_downloads();
                return Task::none();
            }
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
                ureq::AgentBuilder::new()
                    .timeout(Duration::from_secs(30))
                    .build()
                    .get(&url)
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
