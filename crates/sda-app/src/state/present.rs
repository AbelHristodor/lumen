//! Present tab state: search, selection, preview scenes, as is overrides,
//! and go live. `app.rs` routes the top level search and live messages to
//! the functions here by name. Tab specific messages come through `update`.

use crate::app::App;
use crate::msg::{Msg, PresentMsg};
use iced::Task;
use iced::widget::{self, operation, scrollable::RelativeOffset};
use sda_core::asis::{self, Converter};
use sda_core::history::{self, HistoryEntry};
use sda_core::model::{Hymn, Paragraph, Run, Slide, TextBox};
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

pub struct PresentState {
    pub query: String,
    /// Searcher indices of the current hits, best first.
    pub hits: Vec<usize>,
    /// Position inside `hits`.
    pub selected: Option<usize>,
    pub preview_slide: usize,
    pub scroll: widget::Id,
    pub as_is_busy: bool,
    /// Resolved slides by hymn index, slide index and theme key. The
    /// renderer keys its textures by `Arc` address, so the view must hand
    /// it the same `Arc` every frame.
    resolved: Mutex<HashMap<(usize, usize, u64), Arc<ResolvedSlide>>>,
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
            resolved: Mutex::new(HashMap::new()),
            samples: Mutex::new(HashMap::new()),
        }
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
    match msg {
        PresentMsg::AsIsToggle(on) => {
            let Some((_, hymn)) = selected_hymn(app) else {
                return Task::none();
            };
            let hymn = hymn.clone();
            if on {
                app.core.overrides.as_is.insert(hymn.path.clone(), true);
            } else {
                app.core.overrides.as_is.remove(&hymn.path);
            }
            app.core.save_overrides();
            if on
                && asis_paths(app, &hymn).is_none()
                && let Some(conv) = app.core.converter.clone()
            {
                app.present.as_is_busy = true;
                let out = asis_dir(app, &hymn);
                return asis_render(conv, hymn.path, out);
            }
            Task::none()
        }
        PresentMsg::AsIsRendered(result) => {
            app.present.as_is_busy = false;
            match result {
                Ok((path, pngs)) => {
                    ensure_bitmaps(app, &pngs);
                    if app.live.present.source == Source::Hymn(path) {
                        let live = &mut app.live.present;
                        live.slides = pngs.into_iter().map(SlideContent::Bitmap).collect();
                        live.index = live.index.min(live.slides.len().saturating_sub(1));
                    }
                }
                Err(e) => log::warn!("as is render failed: {e}"),
            }
            Task::none()
        }
    }
}

/// Render every slide of `pptx` into `out` on a blocking thread.
fn asis_render(conv: Converter, pptx: PathBuf, out: PathBuf) -> Task<Msg> {
    let source = pptx.clone();
    Task::perform(
        blocking(move || {
            asis::render(&conv, &pptx, &out, ASIS_SIZE)
                .map(|pngs| (pptx, pngs))
                .map_err(|e| e.to_string())
        }),
        move |r| {
            let r = r
                .and_then(|inner| inner)
                .map(|(_, pngs)| (source.clone(), pngs));
            Msg::Present(PresentMsg::AsIsRendered(r))
        },
    )
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

/// Decode any PNG not yet in `app.live.bitmaps`.
fn ensure_bitmaps(app: &mut App, pngs: &[PathBuf]) {
    for p in pngs {
        if app.live.bitmaps.contains_key(p) {
            continue;
        }
        match image::open(p) {
            Ok(img) => {
                app.live.bitmaps.insert(p.clone(), Arc::new(img.to_rgba8()));
            }
            Err(e) => log::warn!("as is slide {} failed to load: {e}", p.display()),
        }
    }
}

/// Rank hymns against `app.present.query` and select the first hit.
pub fn run_search(app: &mut App) {
    if let Ok(mut c) = app.present.resolved.lock() {
        c.clear();
    }
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

/// Put the selected hymn live starting at `slide`.
pub fn go_live(app: &mut App, slide: usize) {
    let Some((_, hymn)) = selected_hymn(app) else {
        return;
    };
    let hymn = hymn.clone();
    let slides: Vec<SlideContent> = match asis_paths(app, &hymn) {
        Some(pngs) => {
            ensure_bitmaps(app, &pngs);
            pngs.into_iter().map(SlideContent::Bitmap).collect()
        }
        None => hymn
            .slides
            .iter()
            .cloned()
            .map(SlideContent::Styled)
            .collect(),
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

fn still(
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

/// A still frame of slide `slide` of the selected hymn under the live theme.
pub fn slide_scene(app: &App, slide: usize, size: (u32, u32)) -> Option<Arc<Scene>> {
    let (idx, hymn) = selected_hymn(app)?;
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
    let content = SlideContent::Styled(hymn.slides.get(slide)?.clone());
    let key = (idx, slide, theme.cache_key());
    let mut cache = app.present.resolved.lock().ok()?;
    if cache.len() > RESOLVED_CAP {
        cache.clear();
    }
    let resolved = match cache.get(&key) {
        Some(r) => r.clone(),
        None => {
            let footer = theme.footer.then(|| title_of(hymn));
            let r = Arc::new(resolve(theme, &content, footer.as_deref())?);
            cache.insert(key, r.clone());
            r
        }
    };
    Some(still(theme, &app.live.bitmaps, Layer::Text(resolved), size))
}

/// A still frame of `sample` text under `theme`, for the theme picker.
pub fn sample_scene(
    app: &App,
    theme: &Theme,
    sample: &str,
    size: (u32, u32),
) -> Option<Arc<Scene>> {
    let key = theme.cache_key();
    let mut cache = app.present.samples.lock().ok()?;
    let resolved = match cache.get(&key) {
        Some(r) => r.clone(),
        None => {
            let slide = Slide {
                background: None,
                boxes: vec![TextBox {
                    frame: None,
                    paragraphs: vec![Paragraph {
                        align: None,
                        line_spacing: None,
                        runs: vec![Run {
                            text: sample.to_string(),
                            ..Run::default()
                        }],
                    }],
                }],
                notes: None,
            };
            let r = Arc::new(resolve(theme, &SlideContent::Styled(slide), None)?);
            cache.insert(key, r.clone());
            r
        }
    };
    Some(still(theme, &app.live.bitmaps, Layer::Text(resolved), size))
}

/// Position in `hits` of the hymn at `path`, for the Recent rows.
pub fn hit_position(app: &App, path: &Path) -> Option<usize> {
    let searcher = app.core.searcher.as_ref()?;
    app.present
        .hits
        .iter()
        .position(|&i| searcher.entry(i).is_some_and(|h| h.path == path))
}
