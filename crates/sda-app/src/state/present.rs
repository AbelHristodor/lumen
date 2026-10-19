//! Present tab state: search, selection, preview scenes, as is overrides,
//! and go live. `app.rs` routes the top level search and live messages to
//! the functions here by name. Tab specific messages come through `update`.

use crate::app::App;
use crate::msg::present::Bitmap;
use crate::msg::{Msg, PresentMsg};
use iced::Task;
use iced::widget::{self, operation, scrollable::RelativeOffset};
use sda_core::asis::{self, Converter};
use sda_core::history::{self, HistoryEntry};
use sda_core::model::Hymn;
use sda_core::present::{Presentation, Source};
use sda_core::resolve::{ResolvedSlide, SlideContent, resolve};
use sda_core::theme::{Background, Rgba, Theme};
use sda_render::{BackgroundState, Layer, Scene};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Id of the search box, so Cmd+K can focus it.
pub const SEARCH_ID: &str = "search";
/// Output size for as is renders. The projector size lands with Task 18.
pub const ASIS_SIZE: (u32, u32) = (1920, 1080);
/// How many slide thumbnails the strip shows before the `+N` label.
pub const MAX_THUMBS: usize = 12;
/// How many history entries the Recent section shows.
pub const RECENT_COUNT: usize = 10;
/// Resolved slides kept for the preview before the cache is dropped.
const RESOLVED_CAP: usize = 256;

/// Cache key of a resolved preview slide: hymn path and mtime, slide
/// index, theme key. Paths survive a searcher swap, indices do not.
type ResolvedKey = (PathBuf, i64, usize, u64);

pub struct PresentState {
    pub query: String,
    /// Searcher indices of the current hits, best first.
    pub hits: Vec<usize>,
    /// Position inside `hits`.
    pub selected: Option<usize>,
    pub preview_slide: usize,
    pub scroll: widget::Id,
    pub as_is_busy: bool,
    /// Why the last as is render or load failed. Shown under the toggle.
    pub as_is_error: Option<String>,
    /// A bitmap load `go_live` could not start itself. `pending_load`
    /// turns it into a task.
    pending: Option<(PathBuf, Vec<PathBuf>)>,
    /// Resolved slides for the preview. The renderer keys its textures by
    /// `Arc` address, so the view must hand it the same `Arc` every frame.
    resolved: Mutex<HashMap<ResolvedKey, Arc<ResolvedSlide>>>,
    /// Sample slide per theme key for the theme picker thumbnails.
    samples: Mutex<HashMap<u64, Arc<ResolvedSlide>>>,
}

impl Default for PresentState {
    fn default() -> Self {
        PresentState {
            query: String::new(),
            hits: Vec::new(),
            selected: None,
            preview_slide: 0,
            scroll: widget::Id::new("results"),
            as_is_busy: false,
            as_is_error: None,
            pending: None,
            resolved: Mutex::new(HashMap::new()),
            samples: Mutex::new(HashMap::new()),
        }
    }
}

impl PresentState {
    /// True when `go_live` queued a bitmap load that no task has started.
    pub fn has_pending_load(&self) -> bool {
        self.pending.is_some()
    }
}

/// Run `f` off the UI thread. A panic inside becomes an `Err` string.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| format!("worker failed: {e}"))
}

/// Line shown as the presentation title and the slide footer.
pub fn title_of(hymn: &Hymn) -> String {
    match &hymn.number {
        Some(n) => format!("{n} {}", hymn.title),
        None => hymn.title.clone(),
    }
}

/// Searcher index and entry of the selected hit.
pub fn selected_hymn(app: &App) -> Option<(usize, &Hymn)> {
    let idx = *app.present.hits.get(app.present.selected?)?;
    Some((idx, app.core.searcher.as_ref()?.entry(idx)?))
}

pub fn update(app: &mut App, msg: PresentMsg) -> Task<Msg> {
    let pending = pending_load(app);
    let task = match msg {
        PresentMsg::AsIsToggle(on) => {
            let Some((_, hymn)) = selected_hymn(app) else {
                return pending;
            };
            let hymn = hymn.clone();
            app.present.as_is_error = None;
            if on {
                app.core.overrides.as_is.insert(hymn.path.clone(), true);
            } else {
                app.core.overrides.as_is.remove(&hymn.path);
            }
            app.core.save_overrides();
            if !on || app.present.as_is_busy {
                return pending;
            }
            match asis_paths(app, &hymn) {
                Some(pngs) if bitmaps_missing(app, &pngs) => {
                    app.present.as_is_busy = true;
                    asis_load(hymn.path, pngs)
                }
                Some(_) => Task::none(),
                None => {
                    let Some(conv) = app.core.converter.clone() else {
                        return pending;
                    };
                    app.present.as_is_busy = true;
                    let out = asis_dir(app, &hymn);
                    asis_render(conv, hymn.path, out)
                }
            }
        }
        PresentMsg::AsIsRendered(result) => {
            app.present.as_is_busy = false;
            match result {
                Ok((path, bitmaps)) => {
                    let pngs: Vec<PathBuf> = bitmaps.iter().map(|(p, _)| p.clone()).collect();
                    app.live.bitmaps.extend(bitmaps);
                    let still_on = app.core.overrides.as_is.get(&path) == Some(&true);
                    if still_on && app.live.present.source == Source::Hymn(path) {
                        let live = &mut app.live.present;
                        live.slides = pngs.into_iter().map(SlideContent::Bitmap).collect();
                        live.index = live.index.min(live.slides.len().saturating_sub(1));
                    }
                }
                Err(e) => {
                    log::warn!("as is render failed: {e}");
                    app.present.as_is_error = Some(e);
                }
            }
            Task::none()
        }
    };
    Task::batch([pending, task])
}

/// Start the bitmap load `go_live` queued, if any. `go_live` returns `()`
/// so it cannot start a task itself. `app.rs` can return this after it.
pub fn pending_load(app: &mut App) -> Task<Msg> {
    match app.present.pending.take() {
        Some((path, pngs)) => asis_load(path, pngs),
        None => Task::none(),
    }
}

/// Decode `pngs` in order. Any failure fails the whole load.
fn decode(pngs: Vec<PathBuf>) -> Result<Vec<Bitmap>, String> {
    pngs.into_iter()
        .map(|p| {
            image::open(&p)
                .map(|img| (p.clone(), Arc::new(img.to_rgba8())))
                .map_err(|e| format!("{}: {e}", p.display()))
        })
        .collect()
}

/// Wrap a worker result as `AsIsRendered` for `hymn`.
fn rendered(hymn: PathBuf, r: Result<Result<Vec<Bitmap>, String>, String>) -> Msg {
    Msg::Present(PresentMsg::AsIsRendered(
        r.and_then(|inner| inner).map(|b| (hymn, b)),
    ))
}

/// Render every slide of `pptx` into `out` and decode them on a blocking
/// thread.
fn asis_render(conv: Converter, pptx: PathBuf, out: PathBuf) -> Task<Msg> {
    let source = pptx.clone();
    Task::perform(
        blocking(move || {
            asis::render(&conv, &pptx, &out, ASIS_SIZE)
                .map_err(|e| e.to_string())
                .and_then(decode)
        }),
        move |r| rendered(source.clone(), r),
    )
}

/// Decode an existing cache for `pptx` on a blocking thread.
fn asis_load(pptx: PathBuf, pngs: Vec<PathBuf>) -> Task<Msg> {
    Task::perform(blocking(move || decode(pngs)), move |r| {
        rendered(pptx.clone(), r)
    })
}

fn asis_dir(app: &App, hymn: &Hymn) -> PathBuf {
    asis::cache_dir_for(
        &app.core.paths.asis_cache,
        &hymn.path,
        hymn.mtime,
        ASIS_SIZE,
    )
}

/// True when the Original toggle is on for this hymn.
pub fn as_is_on(app: &App, hymn: &Hymn) -> bool {
    app.core.overrides.as_is.get(&hymn.path) == Some(&true)
}

/// Finished as is PNGs for `hymn` when the override is on and a converter
/// exists, else `None`.
pub fn asis_paths(app: &App, hymn: &Hymn) -> Option<Vec<PathBuf>> {
    if app.core.converter.is_none() || !as_is_on(app, hymn) {
        return None;
    }
    asis::cached(&asis_dir(app, hymn))
}

/// True when any of `pngs` is not decoded yet.
fn bitmaps_missing(app: &App, pngs: &[PathBuf]) -> bool {
    pngs.iter().any(|p| !app.live.bitmaps.contains_key(p))
}

/// Rank hymns against `app.present.query` and select the first hit.
pub fn run_search(app: &mut App) {
    app.present.as_is_error = None;
    let Some(searcher) = app.core.searcher.as_mut() else {
        app.present.hits.clear();
        app.present.selected = None;
        return;
    };
    app.present.hits = searcher
        .search(&app.present.query)
        .into_iter()
        .map(|h| h.index)
        .collect();
    app.present.selected = (!app.present.hits.is_empty()).then_some(0);
    app.present.preview_slide = 0;
}

pub fn select(app: &mut App, i: usize) {
    if i < app.present.hits.len() {
        app.present.selected = Some(i);
        app.present.preview_slide = 0;
        app.present.as_is_error = None;
    }
}

/// Move the selection by `delta` rows, clamped to the hit list.
pub fn move_selection(app: &mut App, delta: i32) {
    let len = app.present.hits.len();
    if len == 0 {
        return;
    }
    let cur = app.present.selected.unwrap_or(0) as i64;
    let next = (cur + delta as i64).clamp(0, len as i64 - 1) as usize;
    select(app, next);
}

/// Scroll the result list so the selected row is in view. `app.rs` can
/// return this after `move_selection` once it forwards tasks.
pub fn snap_to_selection(app: &App) -> Task<Msg> {
    let len = app.present.hits.len();
    let Some(sel) = app.present.selected.filter(|_| len > 1) else {
        return Task::none();
    };
    let y = sel as f32 / (len - 1) as f32;
    operation::snap_to(app.present.scroll.clone(), RelativeOffset { x: 0.0, y })
}

/// Put the selected hymn live starting at `slide`. With the Original
/// override on and a finished cache whose PNGs are not decoded yet, the
/// styled slides go live now and a bitmap load is queued. `AsIsRendered`
/// swaps the slides when it lands.
pub fn go_live(app: &mut App, slide: usize) {
    let Some((_, hymn)) = selected_hymn(app) else {
        return;
    };
    let hymn = hymn.clone();
    let styled = || {
        hymn.slides
            .iter()
            .cloned()
            .map(SlideContent::Styled)
            .collect::<Vec<_>>()
    };
    let slides = match asis_paths(app, &hymn) {
        Some(pngs) if !bitmaps_missing(app, &pngs) => {
            pngs.into_iter().map(SlideContent::Bitmap).collect()
        }
        Some(pngs) => {
            if !app.present.as_is_busy {
                app.present.as_is_busy = true;
                app.present.pending = Some((hymn.path.clone(), pngs));
            }
            styled()
        }
        None => styled(),
    };
    let mut p = Presentation::default();
    p.load(
        title_of(&hymn),
        Some(hymn.library.clone()),
        slides,
        Source::Hymn(hymn.path.clone()),
    );
    p.jump(slide + 1);
    app.live.go(p);
    let opened_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    history::record(
        &mut app.core.history,
        HistoryEntry {
            path: hymn.path,
            number: hymn.number,
            title: hymn.title,
            opened_at,
        },
    );
    app.core.save_history();
}

/// The theme background without video. Previews show a still.
fn background(theme: &Theme, bitmaps: &HashMap<PathBuf, Arc<image::RgbaImage>>) -> BackgroundState {
    match &theme.background {
        Background::Solid(c) => BackgroundState::Solid(*c),
        Background::Gradient {
            stops,
            angle,
            drift,
        } => BackgroundState::Gradient {
            stops: stops.clone(),
            angle: *angle,
            drift: *drift,
        },
        Background::Particles {
            base,
            color,
            count,
            speed,
        } => BackgroundState::Particles {
            base: *base,
            color: *color,
            count: *count,
            speed: *speed,
        },
        Background::Image(path) => bitmaps
            .get(path)
            .map(|b| BackgroundState::Image(b.clone()))
            .unwrap_or(BackgroundState::Solid(Rgba::BLACK)),
        Background::Video { .. } => BackgroundState::Solid(Rgba::BLACK),
    }
}

pub(crate) fn still(
    theme: &Theme,
    bitmaps: &HashMap<PathBuf, Arc<image::RgbaImage>>,
    layer: Layer,
    size: (u32, u32),
) -> Arc<Scene> {
    Arc::new(Scene {
        size,
        background: background(theme, bitmaps),
        background_opacity: theme.background_opacity,
        current: Some(layer),
        outgoing: None,
        transition: theme.transition,
        blank: false,
        now_secs: 0.0,
    })
}

/// Slide `slide` of `hymn` resolved under `theme`, through the preview
/// cache so the renderer sees one `Arc` per slide and theme.
fn resolve_slide(
    app: &App,
    hymn: &Hymn,
    slide: usize,
    theme: &Theme,
) -> Option<Arc<ResolvedSlide>> {
    let content = SlideContent::Styled(hymn.slides.get(slide)?.clone());
    let key = (hymn.path.clone(), hymn.mtime, slide, theme.cache_key());
    let mut cache = app.present.resolved.lock().ok()?;
    if cache.len() > RESOLVED_CAP {
        cache.clear();
    }
    if let Some(r) = cache.get(&key) {
        return Some(r.clone());
    }
    let footer = theme.footer.then(|| title_of(hymn));
    let r = Arc::new(resolve(theme, &content, footer.as_deref())?);
    cache.insert(key, r.clone());
    Some(r)
}

/// A still frame of slide `slide` of the selected hymn under the live theme.
pub fn slide_scene(app: &App, slide: usize, size: (u32, u32)) -> Option<Arc<Scene>> {
    let (_, hymn) = selected_hymn(app)?;
    let theme = &app.live.theme;
    if let Some(pngs) = asis_paths(app, hymn)
        && let Some(bitmap) = pngs.get(slide).and_then(|p| app.live.bitmaps.get(p))
    {
        return Some(still(
            theme,
            &app.live.bitmaps,
            Layer::Bitmap(bitmap.clone()),
            size,
        ));
    }
    let resolved = resolve_slide(app, hymn, slide, theme)?;
    Some(still(theme, &app.live.bitmaps, Layer::Text(resolved), size))
}

/// The slide a theme picker row shows under `theme`: the previewed slide
/// of the selected hymn, else the shared sample, both through a cache so
/// the renderer sees one `Arc` per theme.
pub fn theme_slide(app: &App, theme: &Theme) -> Option<Arc<ResolvedSlide>> {
    if let Some((_, hymn)) = selected_hymn(app) {
        return resolve_slide(app, hymn, app.present.preview_slide, theme);
    }
    let key = theme.cache_key();
    let mut cache = app.present.samples.lock().ok()?;
    if let Some(r) = cache.get(&key) {
        return Some(r.clone());
    }
    let (content, title, _) = crate::state::themes::hymn_sample(app);
    let r = Arc::new(crate::state::themes::thumb_slide(theme, &content, &title)?);
    cache.insert(key, r.clone());
    Some(r)
}

/// Position in `hits` of the hymn at `path`, for the Recent rows.
pub fn hit_position(app: &App, path: &Path) -> Option<usize> {
    let searcher = app.core.searcher.as_ref()?;
    app.present
        .hits
        .iter()
        .position(|&i| searcher.entry(i).is_some_and(|h| h.path == path))
}
