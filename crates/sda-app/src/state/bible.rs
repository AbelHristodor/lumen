//! Bible tab state: installed versions, the loaded Bible, the browsing
//! position, the verse selection, search, and the Manage view.

use crate::app::App;
use crate::msg::{BibleMsg, Msg};
use iced::Task;
use sda_core::bible::{
    self, Bible, Reference, VersionInfo, fits_heuristic, format_reference, pack_verses,
    parse_reference, verse_slides,
};
use sda_core::present::{Presentation, Source};
use sda_core::resolve::{SlideContent, verse_box_pt};
use sda_core::sync::sync_file;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct BibleState {
    pub input: String,
    /// Code of the loaded version, for example `en_kjv`.
    pub version: Option<String>,
    pub bible: Option<Arc<Bible>>,
    pub book: usize,
    pub chapter: u32,
    pub selected: BTreeSet<u32>,
    /// Last verse clicked without Shift. Shift click and drag range from it.
    pub anchor: Option<u32>,
    /// True between `Press` and `Release`. `Enter` extends the range then.
    pub dragging: bool,
    /// The selection right after the press that began the drag. Each
    /// `Enter` rebuilds the selection from it plus the drag range.
    pub drag_base: BTreeSet<u32>,
    /// Text mode: the bar searches verse text instead of parsing a reference.
    pub text_mode: bool,
    /// Which packed slide the preview pane shows.
    pub preview_slide: usize,
    pub results: Vec<(Reference, String)>,
    pub manage_open: bool,
    pub index: Vec<VersionInfo>,
    /// Codes of the version files on disk.
    pub installed: Vec<String>,
    /// Code of the version being downloaded or loaded.
    pub busy: Option<String>,
    pub error: Option<String>,
}

impl Default for BibleState {
    fn default() -> Self {
        BibleState {
            input: String::new(),
            version: None,
            bible: None,
            book: 0,
            chapter: 1,
            selected: BTreeSet::new(),
            anchor: None,
            dragging: false,
            drag_base: BTreeSet::new(),
            text_mode: false,
            preview_slide: 0,
            results: Vec::new(),
            manage_open: false,
            index: Vec::new(),
            installed: Vec::new(),
            busy: None,
            error: None,
        }
    }
}

const SEARCH_LIMIT: usize = 50;

impl BibleState {
    /// Upper case abbreviation of a version code, from the index when known
    /// and from the code suffix otherwise, so `en_kjv` reads `KJV`.
    pub fn abbreviation(&self, code: &str) -> String {
        self.index
            .iter()
            .find(|v| v.code == code)
            .map(|v| v.abbreviation.as_str())
            .unwrap_or_else(|| code.rsplit('_').next().unwrap_or(code))
            .to_uppercase()
    }

    /// The selection as one reference. A gap inside the selection still
    /// prints as the outer range, the presented verses are the chosen ones.
    pub fn reference(&self) -> Reference {
        let verses = match (self.selected.first(), self.selected.last()) {
            (Some(a), Some(b)) => Some((*a, *b)),
            _ => None,
        };
        Reference {
            book: self.book,
            chapter: self.chapter,
            verses,
        }
    }

    /// Number of chapters in the current book.
    pub fn chapter_count(&self) -> u32 {
        self.bible
            .as_ref()
            .and_then(|b| b.books.get(self.book))
            .map(|b| b.chapters.len() as u32)
            .unwrap_or(0)
    }

    /// Verses of the current chapter as `(number, text)`.
    pub fn chapter_verses(&self) -> Vec<(u32, String)> {
        let Some(b) = &self.bible else {
            return Vec::new();
        };
        bible::verses(
            b,
            &Reference {
                book: self.book,
                chapter: self.chapter,
                verses: None,
            },
        )
    }

    fn jump(&mut self, r: Reference) {
        self.book = r.book;
        self.chapter = r.chapter.max(1);
        self.selected = match r.verses {
            Some((a, b)) => (a..=b).collect(),
            None => BTreeSet::new(),
        };
        self.anchor = None;
        self.selection_changed();
    }

    /// The preview starts over on the first slide.
    fn selection_changed(&mut self) {
        self.preview_slide = 0;
    }

    /// Replace the drag range. Verses selected before the drag began stay.
    pub fn drag_to(&mut self, n: u32) {
        let Some(a) = self.anchor else { return };
        self.selected = self.drag_base.clone();
        self.selected.extend(a.min(n)..=a.max(n));
        self.selection_changed();
    }
}

fn version_path(dir: &Path, code: &str) -> PathBuf {
    dir.join(format!("{code}.json"))
}

fn stamp_path(dir: &Path, code: &str) -> PathBuf {
    dir.join(format!("{code}.etag"))
}

/// Codes of every `*.json` in the bibles dir except the index, sorted.
pub fn installed_versions(dir: &Path) -> Vec<String> {
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

/// Run `f` off the UI thread. A panic inside becomes an `Err` string.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| format!("worker failed: {e}"))
}

fn flatten<T>(r: Result<Result<T, String>, String>) -> Result<T, String> {
    r.and_then(|inner| inner)
}

/// Download the pinned version index and parse it.
pub fn bible_index(dir: PathBuf) -> Task<Msg> {
    Task::perform(
        blocking(move || {
            let dest = dir.join("index.json");
            sync_file(&bible::index_url(), &dest, &dir.join("index.etag"))
                .map_err(|e| e.to_string())?;
            read_index(&dest)
        }),
        |r| Msg::Bible(BibleMsg::IndexLoaded(flatten(r))),
    )
}

/// Parse the index file. The real repository index stores the whole code
/// in `abbreviation`, for example `en_kjv`, which `parse_index` then
/// prefixes again. Undo that here so codes match the download URLs.
fn read_index(path: &Path) -> Result<Vec<VersionInfo>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut versions = bible::parse_index(&text).map_err(|e| e.to_string())?;
    for v in &mut versions {
        if let Some((_, short)) = v.abbreviation.split_once('_') {
            v.code = v.abbreviation.clone();
            v.abbreviation = short.to_string();
            v.public_domain = bible::public_domain(&v.code);
        }
    }
    Ok(versions)
}

/// Download one version file.
pub fn bible_download(dir: PathBuf, code: String) -> Task<Msg> {
    Task::perform(
        blocking(move || {
            sync_file(
                &bible::version_url(&code),
                &version_path(&dir, &code),
                &stamp_path(&dir, &code),
            )
            .map(|_| code)
            .map_err(|e| e.to_string())
        }),
        |r| Msg::Bible(BibleMsg::Downloaded(flatten(r))),
    )
}

/// Parse one installed version file.
pub fn bible_load(dir: PathBuf, code: String) -> Task<Msg> {
    let for_code = code.clone();
    Task::perform(
        blocking(move || {
            bible::load(&version_path(&dir, &code))
                .map(Arc::new)
                .map_err(|e| e.to_string())
        }),
        move |r| Msg::Bible(BibleMsg::Loaded(for_code.clone(), flatten(r))),
    )
}

/// Start loading `code` and remember it as the chosen version.
fn switch_version(app: &mut App, code: String) -> Task<Msg> {
    let dir = app.core.paths.bibles_dir.clone();
    app.bible.error = None;
    app.bible.busy = Some(code.clone());
    app.bible.version = Some(code.clone());
    if app.core.cfg.bible_version.as_deref() != Some(&code) {
        app.core.cfg.bible_version = Some(code.clone());
        app.core.save_cfg();
    }
    bible_load(dir, code)
}

/// Load the saved version when nothing is loaded yet. Safe to call often.
/// `app.rs` calls this on `Msg::Tab(Tab::Bible)`.
pub fn opened(app: &mut App) -> Task<Msg> {
    let dir = app.core.paths.bibles_dir.clone();
    app.bible.installed = installed_versions(&dir);
    if app.bible.index.is_empty()
        && let Ok(index) = read_index(&dir.join("index.json"))
    {
        app.bible.index = index;
    }
    if app.bible.bible.is_some() || app.bible.busy.is_some() {
        return Task::none();
    }
    let saved = app
        .bible
        .version
        .clone()
        .or_else(|| app.core.cfg.bible_version.clone())
        .filter(|c| app.bible.installed.contains(c))
        // With versions on disk and no saved pick, the first one is better
        // than an empty tab that claims nothing is installed.
        .or_else(|| app.bible.installed.first().cloned());
    match saved {
        Some(code) => switch_version(app, code),
        None => Task::none(),
    }
}

fn run_search(app: &mut App) {
    app.bible.results = match &app.bible.bible {
        Some(b) if app.bible.text_mode && !app.bible.input.trim().is_empty() => {
            bible::search(b, &app.bible.input, SEARCH_LIMIT)
        }
        _ => Vec::new(),
    };
}

fn present(app: &mut App) {
    let Some(b) = app.bible.bible.clone() else {
        return;
    };
    let slides = preview_slides(app);
    if slides.is_empty() {
        return;
    }
    let reference = format_reference(&app.bible.reference(), app.core.language);
    let version = app.bible.version.clone().unwrap_or_else(|| b.code.clone());
    app.live.go(Presentation {
        title: reference.clone(),
        subtitle: Some(app.bible.abbreviation(&version)),
        slides,
        source: Source::Bible { version, reference },
        ..Presentation::default()
    });
}

/// The packed slides the preview pane shows: the selection, or the whole
/// chapter when nothing is selected. Resolved under the live theme.
pub fn preview_slides(app: &App) -> Vec<SlideContent> {
    let Some(b) = &app.bible.bible else {
        return Vec::new();
    };
    let r = app.bible.reference();
    let verses: Vec<(u32, String)> = bible::verses(b, &r)
        .into_iter()
        .filter(|(n, _)| app.bible.selected.is_empty() || app.bible.selected.contains(n))
        .collect();
    let theme = &app.live.theme;
    let groups = pack_verses(&verses, fits_heuristic(theme, verse_box_pt(theme)));
    verse_slides(&r, &groups, app.core.language)
}

/// A static scene of one verses slide under the live theme, same shape the
/// Present preview uses.
pub fn preview_scene(
    app: &App,
    content: &SlideContent,
    size: (u32, u32),
) -> Option<Arc<sda_render::Scene>> {
    let theme = &app.live.theme;
    let footer = theme.footer.then(|| {
        app.bible
            .version
            .as_deref()
            .map(|v| app.bible.abbreviation(v))
            .unwrap_or_default()
    });
    let resolved = Arc::new(sda_core::resolve::resolve(
        theme,
        content,
        footer.as_deref(),
    )?);
    Some(crate::state::present::still(
        theme,
        &app.live.bitmaps,
        sda_render::Layer::Text(resolved),
        size,
    ))
}

pub fn update(app: &mut App, msg: BibleMsg) -> Task<Msg> {
    let dir = app.core.paths.bibles_dir.clone();
    match msg {
        BibleMsg::Opened => return opened(app),
        BibleMsg::Input(text) => {
            app.bible.input = text;
            if app.bible.text_mode {
                run_search(app);
            } else if let Some(r) = parse_reference(&app.bible.input, app.core.language) {
                app.bible.jump(r);
            }
        }
        BibleMsg::Submit => {
            if app.bible.text_mode
                && let Some(r) = app.bible.results.first().map(|(r, _)| *r)
            {
                app.bible.jump(r);
            }
        }
        BibleMsg::TextMode(on) => {
            app.bible.text_mode = on;
            app.bible.input.clear();
            run_search(app);
        }
        BibleMsg::PreviewSlide(i) => {
            let n = preview_slides(app).len();
            app.bible.preview_slide = i.min(n.saturating_sub(1));
        }
        BibleMsg::Jump(r) => app.bible.jump(r),
        BibleMsg::Version(code) => {
            if app.bible.busy.is_some() {
                return Task::none();
            }
            if app.bible.version.as_deref() != Some(&code) || app.bible.bible.is_none() {
                return switch_version(app, code);
            }
        }
        BibleMsg::Book(i) => {
            app.bible.book = i;
            app.bible.chapter = 1;
            app.bible.selected.clear();
            app.bible.anchor = None;
            app.bible.selection_changed();
        }
        BibleMsg::Chapter(c) => {
            app.bible.chapter = c.max(1);
            app.bible.selected.clear();
            app.bible.anchor = None;
            app.bible.selection_changed();
        }
        BibleMsg::Press(n) => {
            if app.modifiers.shift()
                && let Some(a) = app.bible.anchor
            {
                app.bible.selected.extend(a.min(n)..=a.max(n));
            } else {
                if !app.bible.selected.remove(&n) {
                    app.bible.selected.insert(n);
                }
                app.bible.anchor = Some(n);
                app.bible.drag_base = app.bible.selected.clone();
                app.bible.dragging = true;
            }
            app.bible.selection_changed();
        }
        BibleMsg::Enter(n) => {
            if app.bible.dragging {
                app.bible.drag_to(n);
            }
        }
        BibleMsg::Release => app.bible.dragging = false,
        BibleMsg::ClearSelection => {
            app.bible.selected.clear();
            app.bible.anchor = None;
            app.bible.selection_changed();
        }
        BibleMsg::Present => present(app),
        BibleMsg::ManageOpen => {
            app.bible.manage_open = true;
            app.bible.error = None;
            app.bible.installed = installed_versions(&dir);
            if app.bible.index.is_empty() && app.bible.busy.is_none() {
                app.bible.busy = Some("index".into());
                return bible_index(dir);
            }
        }
        BibleMsg::ManageClose => app.bible.manage_open = false,
        BibleMsg::Download(code) => {
            if app.bible.busy.is_none() {
                app.bible.error = None;
                app.bible.busy = Some(code.clone());
                return bible_download(dir, code);
            }
        }
        BibleMsg::Remove(code) => {
            let _ = std::fs::remove_file(version_path(&dir, &code));
            let _ = std::fs::remove_file(stamp_path(&dir, &code));
            app.bible.installed = installed_versions(&dir);
            if app.bible.version.as_deref() == Some(&code) {
                app.bible.version = None;
                app.bible.bible = None;
                app.bible.results.clear();
                app.core.cfg.bible_version = None;
                app.core.save_cfg();
            }
        }
        BibleMsg::IndexLoaded(r) => {
            app.bible.busy = None;
            match r {
                Ok(index) => app.bible.index = index,
                Err(e) => app.bible.error = Some(e),
            }
        }
        BibleMsg::Downloaded(r) => {
            app.bible.busy = None;
            match r {
                Ok(code) => {
                    app.bible.installed = installed_versions(&dir);
                    if app.bible.bible.is_none() {
                        return switch_version(app, code);
                    }
                }
                Err(e) => app.bible.error = Some(e),
            }
        }
        BibleMsg::Loaded(code, r) => {
            if app.bible.version.as_deref() != Some(&code) {
                log::debug!("dropping stale bible load for {code}");
                return Task::none();
            }
            app.bible.busy = None;
            match r {
                Ok(b) => {
                    if app.bible.book >= b.books.len() {
                        app.bible.book = 0;
                        app.bible.chapter = 1;
                        app.bible.selected.clear();
                        app.bible.anchor = None;
                    }
                    app.bible.bible = Some(b);
                    run_search(app);
                }
                Err(e) => {
                    app.bible.error = Some(e);
                    app.bible.version = None;
                }
            }
        }
    }
    Task::none()
}
