//! Background work. Every function returns a `Task` that runs the blocking
//! core call on a tokio blocking thread and ends in exactly one `Msg`.

use crate::msg::{BackgroundAsset, BgTarget, Handoff, Msg, SettingsMsg};
use crate::state::core::Core;
use crate::state::live::LiveState;
use iced::Task;
use sda_core::asis;
use sda_core::downloader;
use sda_core::library::Config;
use sda_core::refresh;
use sda_core::search::Searcher;
use sda_core::sync::SyncOutcome;
use sda_core::theme::Background;
use sda_core::update::{self, UpdateOutcome};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Output size of video backgrounds. The renderer covers the target.
pub const VIDEO_SIZE: (u32, u32) = (1920, 1080);

/// Run `f` off the UI thread. A panic inside becomes an `Err` string.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| format!("worker failed: {e}"))
}

fn searcher(hymns: Vec<sda_core::model::Hymn>) -> Handoff<Searcher> {
    Handoff::new(Searcher::new(hymns))
}

/// Index from disk, download the hymns zip if it changed, then check for a
/// new app version. On a first run the library is downloaded before indexing.
/// Converter detection and the font list run beside that chain.
pub fn boot(core: &mut Core) -> Task<Msg> {
    let generation = core.next_index_gen();
    let cfg = core.cfg.clone();
    let present = refresh::default_library_present();
    let index = {
        let cfg = cfg.clone();
        Task::perform(
            blocking(move || {
                if present {
                    refresh::load_local(cfg)
                } else {
                    refresh::load_library(cfg, false)
                }
            }),
            move |r| Msg::Indexed(generation, r.map(searcher)),
        )
    };
    let sync = if present {
        Task::perform(blocking(move || sync_if_changed(&cfg)), move |r| {
            Msg::Synced(generation, r.and_then(|inner| inner))
        })
    } else {
        Task::none()
    };
    Task::batch([
        index.chain(sync).chain(update_check(core)),
        converter_detect(),
        fonts_list(),
    ])
}

/// Download when the ETag changed and reindex. `Ok(None)` means unchanged.
fn sync_if_changed(cfg: &Config) -> Result<Option<Handoff<Searcher>>, String> {
    match refresh::sync_default(cfg) {
        Some(Ok(SyncOutcome::Downloaded)) => Ok(Some(searcher(refresh::load_local(cfg.clone())))),
        Some(Ok(SyncOutcome::Unchanged)) => Ok(None),
        Some(Err(e)) => Err(e),
        None => Err("no data directory".into()),
    }
}

/// Wipe the built in library and cache, then download and index again.
pub fn force_sync(core: &mut Core) -> Task<Msg> {
    let generation = core.next_index_gen();
    let cfg = core.cfg.clone();
    Task::perform(
        blocking(move || match refresh::force_clean() {
            Ok(()) => Ok(Some(searcher(refresh::load_library(cfg, true)))),
            Err(e) => Err(format!("force clean failed: {e}")),
        }),
        move |r| Msg::Synced(generation, r.and_then(|inner| inner)),
    )
}

/// Reindex after the library folder list changed.
pub fn reindex(core: &mut Core) -> Task<Msg> {
    let generation = core.next_index_gen();
    let cfg = core.cfg.clone();
    Task::perform(blocking(move || refresh::load_local(cfg)), move |r| {
        Msg::Reindexed(generation, r.map(searcher))
    })
}

/// Ask GitHub for a newer release and stage it. The message is already
/// translated so `update` only stores it.
pub fn update_check(core: &Core) -> Task<Msg> {
    let s = core.strings.clone();
    Task::perform(blocking(update::check_and_stage_update), move |r| {
        let (text, staged) = match r {
            Ok(Ok(UpdateOutcome::UpToDate)) => (s.update_up_to_date, false),
            Ok(Ok(UpdateOutcome::Updated { version })) => {
                (s.update_staged_fmt.replace("{}", &version), true)
            }
            Ok(Ok(UpdateOutcome::Available { version })) => {
                (s.update_ready_fmt.replace("{}", &version), true)
            }
            Ok(Ok(UpdateOutcome::CheckUnavailable)) => (s.update_unavailable, false),
            Ok(Err(e)) => {
                log::warn!("update check failed: {e}");
                (s.update_failed, false)
            }
            Err(e) => {
                log::warn!("{e}");
                (s.update_failed, false)
            }
        };
        Msg::UpdateChecked(text, staged)
    })
}

/// Probe for a wgpu device once. False turns on the text fallbacks.
pub fn gpu_probe() -> Task<Msg> {
    Task::perform(blocking(|| sda_render::headless::device().is_some()), |r| {
        Msg::GpuProbed(r.unwrap_or(false))
    })
}

/// Load what `live`'s background needs from disk: start ffmpeg for a
/// video, decode an image. `Task::none()` when nothing is pending or a
/// worker already runs for this background.
pub fn background(target: BgTarget, live: &mut LiveState) -> Task<Msg> {
    if !live.background_pending() {
        return Task::none();
    }
    live.background_started();
    let bg = live.theme.background.clone();
    let work = bg.clone();
    Task::perform(blocking(move || load_background(&work)), move |r| {
        Msg::BackgroundReady(target, bg.clone(), r.and_then(|inner| inner))
    })
}

fn load_background(bg: &Background) -> Result<BackgroundAsset, String> {
    match bg {
        Background::Image(path) => still(path),
        Background::Video { path } => {
            let spawned = downloader::find_existing("ffmpeg")
                .ok_or_else(|| "ffmpeg not found".to_string())
                .and_then(|ffmpeg| {
                    sda_render::spawn_decoder(&ffmpeg, path, VIDEO_SIZE).map_err(|e| e.to_string())
                });
            match spawned {
                Ok(src) => Ok(BackgroundAsset::Video(src)),
                Err(e) => {
                    log::warn!("video background {} failed: {e}", path.display());
                    still(path).map_err(|_| e)
                }
            }
        }
        _ => Err("background needs no file".into()),
    }
}

fn still(path: &Path) -> Result<BackgroundAsset, String> {
    image::open(path)
        .map(|img| BackgroundAsset::Still(path.to_path_buf(), Arc::new(img.to_rgba8())))
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Find LibreOffice or PowerPoint for as is slides.
pub fn converter_detect() -> Task<Msg> {
    Task::perform(blocking(asis::detect), |r| {
        Msg::Settings(SettingsMsg::Detected(r.ok().flatten()))
    })
}

/// Sorted unique font family names on this machine, for the theme editor.
pub fn fonts_list() -> Task<Msg> {
    Task::perform(
        blocking(|| {
            let system = cosmic_text::FontSystem::new();
            let mut names: Vec<String> = system
                .db()
                .faces()
                .flat_map(|f| f.families.iter().map(|(name, _)| name.clone()))
                .collect();
            names.sort();
            names.dedup();
            names
        }),
        |r| Msg::FontsListed(r.unwrap_or_default()),
    )
}

/// Open `file` in a player sized for `max`. Downloads ffmpeg first when it
/// is missing, the same way the downloader does.
pub fn open_video(file: PathBuf, max: (u32, u32)) -> Task<Msg> {
    Task::perform(
        blocking(move || {
            let ffmpeg = downloader::ensure_ffmpeg().map_err(|e| e.to_string())?;
            sda_media::Player::open(&ffmpeg, &file, max)
                .map(Arc::new)
                .map_err(|e| e.to_string())
        }),
        |r| Msg::Videos(crate::msg::VideosMsg::Opened(r.and_then(|inner| inner))),
    )
}
