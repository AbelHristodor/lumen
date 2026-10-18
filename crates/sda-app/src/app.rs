//! The one app state Iced owns, its message handler, and window routing.

use crate::displays::{self, Display};
use crate::msg::{BgTarget, Handoff, Msg, Tab};
use crate::state::core::{Core, Paths};
use crate::state::live::LiveState;
use crate::state::{self, bible, present, projector, settings, themes, videos};
use crate::{shortcuts, ui_theme, view, workers};
use iced::widget::operation;
use iced::{Element, Point, Size, Subscription, Task, event, window};
use sda_core::i18n::Language;
use sda_core::library::Config;
use sda_core::search::Searcher;
use sda_core::store::{json_load, json_save};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How long the window must stay still before its geometry is written.
pub const WINDOW_SAVE_AFTER: Duration = Duration::from_millis(500);

/// Text shown in the top right chip.
#[derive(Debug, Clone, Default)]
pub struct Status {
    pub library: String,
    /// True when `library` reports a failure. The chip shows a danger dot.
    pub library_error: bool,
    pub sync_busy: bool,
    pub update: Option<String>,
    /// True when a new version was staged. The chip shows a short label.
    pub update_staged: bool,
    /// A warning that is not about the library, such as a background file
    /// that failed to load. Cleared when the next background loads.
    pub notice: Option<String>,
}

/// Saved operator window geometry in logical points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowGeom {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl WindowGeom {
    pub const DEFAULT: WindowGeom = WindowGeom {
        x: 0.0,
        y: 0.0,
        w: 1280.0,
        h: 800.0,
    };
}

/// Operator window moves and resizes. Other window events are dropped here
/// so `update` never sees a redraw stream.
fn window_geometry(event: iced::Event, _: event::Status, id: window::Id) -> Option<Msg> {
    match event {
        iced::Event::Window(window::Event::Moved(p)) => Some(Msg::WindowMoved(id, p)),
        iced::Event::Window(window::Event::Resized(s)) => Some(Msg::WindowResized(id, s)),
        _ => None,
    }
}

pub struct App {
    pub core: Core,
    pub tab: Tab,
    pub live: LiveState,
    pub present: present::PresentState,
    pub bible: bible::BibleState,
    pub videos: videos::VideosState,
    pub themes: themes::ThemesState,
    pub settings: settings::SettingsState,
    pub operator: window::Id,
    pub projector: Option<window::Id>,
    /// Index into `displays` the projector opens on.
    pub projector_display: Option<usize>,
    pub displays: Vec<Display>,
    pub status: Status,
    /// False when wgpu is unavailable. Scenes then show as plain text.
    pub gpu_ok: bool,
    pub window_geom: Option<WindowGeom>,
    /// Bumped on every move or resize. A `SaveWindow` carrying an older
    /// value is a debounce timer that lost the race and does nothing.
    pub window_geom_seq: u64,
    /// Windows opened by Identify.
    pub identify: Vec<window::Id>,
    /// The last frame time. Views draw the live scene at this instant.
    pub now: Instant,
}

impl App {
    fn new(core: Core, operator: window::Id, gpu_ok: bool) -> App {
        let live = LiveState::new(core.active_theme.clone());
        let status = Status {
            library: core.strings.status_loading.clone(),
            ..Status::default()
        };
        App {
            live,
            status,
            core,
            tab: Tab::Present,
            present: present::PresentState::default(),
            bible: bible::BibleState::default(),
            videos: videos::VideosState::default(),
            themes: themes::ThemesState::default(),
            settings: settings::SettingsState::default(),
            operator,
            projector: None,
            projector_display: None,
            displays: Vec::new(),
            gpu_ok,
            window_geom: None,
            window_geom_seq: 0,
            identify: Vec::new(),
            now: Instant::now(),
        }
    }

    /// Load state, open the operator window, start the boot workers.
    pub fn boot() -> (App, Task<Msg>) {
        let core = Core::load();
        let geom: Option<WindowGeom> = json_load(&core.paths.window);
        let (size, position) = match geom {
            Some(g) => (
                Size::new(g.w.max(960.0), g.h.max(640.0)),
                window::Position::Specific(Point::new(g.x, g.y)),
            ),
            None => (Size::new(1280.0, 800.0), window::Position::Centered),
        };
        let (operator, open) = window::open(window::Settings {
            size,
            position,
            min_size: Some(Size::new(960.0, 640.0)),
            ..window::Settings::default()
        });
        // Optimistic until the probe answers, so a working machine never
        // flashes the banner at boot.
        let mut app = App::new(core, operator, true);
        app.window_geom = geom;
        let task = Task::batch([
            open.discard(),
            workers::gpu_probe(),
            workers::background(BgTarget::Live, &app.live),
            workers::boot(&app.core),
            Task::perform(async { displays::list() }, Msg::DisplaysListed),
        ]);
        (app, task)
    }

    /// An app with no window, no GPU, and the four fixture hymns indexed.
    /// Files go to a fresh folder inside one shared temp dir.
    pub fn for_test() -> App {
        static DIR: OnceLock<tempfile::TempDir> = OnceLock::new();
        static SEQ: AtomicUsize = AtomicUsize::new(0);
        let dir = DIR.get_or_init(|| tempfile::tempdir().expect("temp dir"));
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let root = dir.path().join(format!("app{n}"));
        App::for_test_with_dirs(root.join("themes"))
    }

    /// Like `for_test` with the themes folder at `themes_dir`. The other
    /// files sit next to it.
    pub fn for_test_with_dirs(themes_dir: PathBuf) -> App {
        let root = themes_dir
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| themes_dir.clone());
        let paths = Paths {
            themes_dir,
            ..Paths::under(&root)
        };
        let core = Core::with(Config::default(), None, paths, Language::En);
        let mut app = App::new(core, window::Id::unique(), false);
        let fixtures = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../sda-core/tests/fixtures"
        ));
        let hymns = sda_core::index::refresh_index(fixtures, "test", &[]);
        app.core.searcher = Some(Searcher::new(hymns));
        app.displays = vec![Display {
            index: 0,
            label: "Display 1 1440x900".into(),
            rect: (0, 0, 1440, 900),
            scale: 2.0,
            is_primary: true,
        }];
        app.status.library = app.core.strings.status_library_ready.clone();
        present::run_search(&mut app);
        app
    }

    pub fn title(&self, id: window::Id) -> String {
        if Some(id) == self.projector {
            "SDA Manager Projector".into()
        } else {
            "SDA Manager".into()
        }
    }

    pub fn view(&self, id: window::Id) -> Element<'_, Msg> {
        if Some(id) == self.projector {
            view::projector::show(self)
        } else if let Some(n) = projector::identify_number(self, id) {
            view::projector::identify(n)
        } else {
            view::shell::show(self)
        }
    }

    /// The operator window content. Tests render this.
    pub fn view_operator(&self) -> Element<'_, Msg> {
        view::shell::show(self)
    }

    pub fn subscription(&self) -> Subscription<Msg> {
        // The projector has no decorations, so a close request only comes
        // from the OS. The default `exit_on_close_request` lets Iced close
        // that window itself and `close_events` then clears our state.
        let mut subs = vec![
            event::listen_with(shortcuts::filter),
            event::listen_with(window_geometry),
            window::close_events().map(Msg::ProjectorClosed),
        ];
        if self.live.animating() {
            subs.push(window::frames().map(Msg::Tick));
        }
        Subscription::batch(subs)
    }

    pub fn theme(&self, _id: window::Id) -> iced::Theme {
        ui_theme::theme()
    }

    /// Swap in a searcher a worker built and refresh the hit list. `None`
    /// when the handoff was already taken, so the status stays as it is.
    fn install_searcher(&mut self, handoff: &Handoff<Searcher>) -> Option<usize> {
        let searcher = handoff.take()?;
        let len = searcher.len();
        self.core.searcher = Some(searcher);
        present::run_search(self);
        Some(len)
    }

    /// Start the debounce timer for the window geometry.
    fn save_window_later(&mut self) -> Task<Msg> {
        self.window_geom_seq += 1;
        let seq = self.window_geom_seq;
        Task::perform(
            async { tokio::time::sleep(WINDOW_SAVE_AFTER).await },
            move |_| Msg::SaveWindow(seq),
        )
    }

    pub fn update(&mut self, msg: Msg) -> Task<Msg> {
        let s = self.core.strings.clone();
        match msg {
            Msg::Tab(t) => {
                self.tab = t;
                return match t {
                    Tab::Bible => bible::opened(self),
                    Tab::Themes => themes::opened(self),
                    _ => Task::none(),
                };
            }
            Msg::Search(q) => {
                self.present.query = q;
                present::run_search(self);
            }
            Msg::Select(i) => present::select(self, i),
            Msg::MoveSelection(d) => {
                if self.tab == Tab::Present {
                    present::move_selection(self, d);
                    return present::snap_to_selection(self);
                }
            }
            Msg::PreviewSlide(i) => self.present.preview_slide = i,
            Msg::GoLive => {
                present::go_live(self, self.present.preview_slide);
                return present::pending_load(self);
            }
            Msg::GoLiveAt(i) => {
                present::go_live(self, i);
                return present::pending_load(self);
            }
            Msg::FocusSearch => {
                self.tab = Tab::Present;
                return operation::focus(iced::widget::Id::new(view::present::SEARCH_ID));
            }
            Msg::LiveNext => self.live.next(),
            Msg::LivePrev => self.live.prev(),
            Msg::LiveBlank => self.live.blank(),
            Msg::LiveJump(n) => self.live.jump(n),
            Msg::LiveStop => {
                self.live.stop();
                return projector::close(self);
            }
            Msg::ProjectorToggle => return projector::toggle(self),
            Msg::ProjectorDisplay(i) => return projector::choose_display(self, i),
            Msg::ProjectorClosed(id) => {
                if id == self.operator {
                    return iced::exit();
                }
                projector::closed(self, id);
            }
            Msg::Tick(now) => projector::tick(self, now),
            Msg::Identify => return projector::identify(self),
            Msg::IdentifyDone => return projector::identify_done(self),
            Msg::Indexed(r) | Msg::Reindexed(r) => match r {
                Ok(h) => {
                    if let Some(n) = self.install_searcher(&h) {
                        self.status.library = s.status_indexed_fmt.replace("{}", &n.to_string());
                        self.status.library_error = false;
                    }
                }
                Err(e) => {
                    log::warn!("indexing worker failed: {e}");
                    self.status.library = s.worker_failed;
                    self.status.library_error = true;
                }
            },
            Msg::Synced(r) => {
                self.status.sync_busy = false;
                match r {
                    Ok(Some(h)) => {
                        if let Some(n) = self.install_searcher(&h) {
                            self.status.library = s.status_synced_fmt.replace("{}", &n.to_string());
                            self.status.library_error = false;
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        self.status.library = s.status_sync_failed_fmt.replace("{}", &e);
                        self.status.library_error = true;
                    }
                }
            }
            Msg::UpdateChecked(text, staged) => {
                self.status.update = Some(text);
                self.status.update_staged = staged;
            }
            Msg::GpuProbed(ok) => self.gpu_ok = ok,
            Msg::BackgroundReady(target, bg, result) => {
                let live = match target {
                    BgTarget::Live => &mut self.live,
                    BgTarget::ThemePreview => &mut self.themes.preview,
                };
                match result {
                    Ok(asset) => {
                        live.install_background(&bg, asset);
                        // The other state may wait on the same file.
                        let (a, b) = (&mut self.live, &mut self.themes.preview);
                        match target {
                            BgTarget::Live => b.adopt_video(a),
                            BgTarget::ThemePreview => a.adopt_video(b),
                        }
                        if target == BgTarget::Live {
                            self.status.notice = None;
                        }
                    }
                    Err(e) => {
                        log::warn!("background load failed: {e}");
                        if target == BgTarget::Live && self.live.theme.background == bg {
                            self.status.notice = Some(s.background_failed_fmt.replace("{}", &e));
                        }
                    }
                }
            }
            Msg::ForceSync => {
                self.status.sync_busy = true;
                self.status.library_error = false;
                self.status.library = s.status_re_cloning;
                return workers::force_sync(&self.core);
            }
            Msg::Present(m) => return state::present::update(self, m),
            Msg::Bible(m) => return state::bible::update(self, m),
            Msg::Videos(m) => return state::videos::update(self, m),
            Msg::Themes(m) => return state::themes::update(self, m),
            Msg::Settings(m) => return state::settings::update(self, m),
            Msg::AsIsToggle(_) | Msg::AsIsRendered(_) => {}
            Msg::DisplaysListed(list) => {
                let saved = self
                    .core
                    .cfg
                    .output_display
                    .and_then(|i| usize::try_from(i).ok())
                    .filter(|i| *i < list.len());
                self.projector_display = saved
                    .or_else(|| displays::default_display(&list))
                    .or_else(|| {
                        (displays::single_display_allowed() && !list.is_empty()).then_some(0)
                    });
                self.displays = list;
            }
            Msg::FontsListed(fonts) => self.themes.fonts = fonts,
            Msg::WindowMoved(id, p) => {
                if id == self.operator {
                    let g = self.window_geom.get_or_insert(WindowGeom::DEFAULT);
                    g.x = p.x;
                    g.y = p.y;
                    return self.save_window_later();
                }
            }
            Msg::WindowResized(id, size) => {
                if id == self.operator {
                    let g = self.window_geom.get_or_insert(WindowGeom::DEFAULT);
                    g.w = size.width;
                    g.h = size.height;
                    return self.save_window_later();
                }
            }
            Msg::SaveWindow(seq) => {
                if seq == self.window_geom_seq
                    && let Some(g) = self.window_geom
                    && let Err(e) = json_save(&self.core.paths.window, &g)
                {
                    log::warn!("saving window geometry failed: {e}");
                }
            }
            Msg::OpenReleases => {
                if let Err(e) = open::that_detached(sda_core::update::RELEASES_URL) {
                    log::warn!("opening the release page failed: {e}");
                }
            }
            Msg::Noop => {}
        }
        Task::none()
    }
}
