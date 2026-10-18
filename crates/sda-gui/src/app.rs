use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use sda_core::downloader::DownloadEvent;
use sda_core::library;
use sda_core::model::HymnEntry;
use sda_core::search::Searcher;
use sda_core::theme::Theme;

use crate::core::Core;
use crate::projector::{self, Display};
use crate::{live, preview, search_panel, settings, shortcuts, slide, style, videos};

/// Results sent from worker threads to the UI thread.
pub enum Msg {
    Index(Searcher),
    Synced(Result<Searcher, String>),
    Reindexed(Result<Searcher, String>),
    Update(String),
    Download(DownloadEvent),
}

/// Transient operator screen state.
#[derive(Default)]
pub struct UiState {
    pub query: String,
    pub last_query: Option<String>,
    pub hits: Vec<usize>,
    pub selected: Option<usize>,
    pub scroll_to_selected: bool,
    pub preview_slide: usize,
    pub focus_search: bool,
    pub settings_open: bool,
    pub videos_open: bool,
    pub draft: Theme,
    pub draft_index: usize,
    pub fit: slide::FitCache,
    pub video: VideoState,
    pub library_status: String,
}

/// Status bar texts.
#[derive(Default)]
pub struct Status {
    pub library: String,
    pub sync: String,
    pub update: String,
    pub syncing: bool,
}

/// The open projector and the display it covers.
pub struct ProjectorState {
    pub display: usize,
}

/// Progress of the video downloader.
#[derive(Default)]
pub struct VideoState {
    pub url: String,
    pub dir: PathBuf,
    pub title: String,
    pub percent: f32,
    pub speed: String,
    pub eta: String,
    pub phase: VideoPhase,
    pub message: String,
}

/// Where a video download stands.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum VideoPhase {
    #[default]
    Idle,
    Resolving,
    Downloading,
    Done,
    Failed,
}

/// The eframe application.
pub struct App {
    pub core: Core,
    pub ui: UiState,
    pub projector: Option<ProjectorState>,
    pub rx: Receiver<Msg>,
    pub tx: Sender<Msg>,
    pub displays: Vec<Display>,
    pub status: Status,
}

impl App {
    /// Install the style, load the core and start the boot worker.
    pub fn new(cc: &eframe::CreationContext) -> Self {
        style::install(&cc.egui_ctx);
        let core = Core::load();
        let (tx, rx) = mpsc::channel();
        let ui = UiState {
            focus_search: true,
            draft: core.active_theme.clone(),
            draft_index: core.themes.iter().position(|t| t.name == core.active_theme.name).unwrap_or(0),
            video: VideoState { dir: library::downloads_dir(&core.cfg), ..VideoState::default() },
            ..UiState::default()
        };
        let status = Status {
            library: core.strings.status_loading.clone(),
            update: core.strings.update_checking.clone(),
            ..Status::default()
        };
        core.spawn_boot(tx.clone(), cc.egui_ctx.clone());
        App { core, ui, projector: None, rx, tx, displays: projector::list_displays(), status }
    }

    /// An app over an already built searcher, with no workers and no disk writes.
    #[cfg(test)]
    pub fn for_test(searcher: Searcher) -> Self {
        let mut core = Core::load();
        core.cfg_path = None;
        core.searcher = Some(searcher);
        let (tx, rx) = mpsc::channel();
        let ui = UiState { focus_search: true, ..UiState::default() };
        let displays = vec![Display { index: 0, label: "Test".into(), is_primary: true }];
        App { core, ui, projector: None, rx, tx, displays, status: Status::default() }
    }

    /// The hymn highlighted in the result list.
    pub fn selected_entry(&self) -> Option<&HymnEntry> {
        let idx = *self.ui.hits.get(self.ui.selected?)?;
        self.core.searcher.as_ref()?.entry(idx)
    }

    /// Move the highlight by `delta` rows, clamped to the list.
    pub fn move_selection(&mut self, delta: i32) {
        let Some(last) = self.ui.hits.len().checked_sub(1) else { return };
        let cur = self.ui.selected.unwrap_or(0);
        let step = delta.unsigned_abs() as usize;
        let next = if delta < 0 { cur.saturating_sub(step) } else { cur.saturating_add(step).min(last) };
        self.ui.selected = Some(next);
        self.ui.preview_slide = 0;
        self.ui.scroll_to_selected = true;
    }

    /// Rerun the search for the current query and select the best hit.
    pub fn run_search(&mut self) {
        let Some(s) = self.core.searcher.as_mut() else { return };
        self.ui.hits = s.search(&self.ui.query).iter().map(|h| h.index).collect();
        self.ui.last_query = Some(self.ui.query.clone());
        self.ui.selected = if self.ui.hits.is_empty() { None } else { Some(0) };
        self.ui.preview_slide = 0;
        self.ui.scroll_to_selected = true;
    }

    /// Put the selected hymn on the wall at `slide`.
    pub fn go_live(&mut self, slide: usize) {
        let Some(e) = self.selected_entry().cloned() else { return };
        self.core.present.load_hymn(e.number, e.title, e.slides);
        self.core.present.index = slide.min(self.core.present.slide_count().saturating_sub(1));
    }

    /// Advance the live slide.
    pub fn live_next(&mut self) {
        self.core.present.next();
    }

    /// Step the live slide back.
    pub fn live_prev(&mut self) {
        self.core.present.prev();
    }

    /// Toggle the blank screen.
    pub fn live_blank(&mut self) {
        self.core.present.toggle_blank();
    }

    /// Jump the live hymn to slide `n`, counted from one.
    pub fn live_jump(&mut self, n: usize) {
        if (1..=self.core.present.slide_count()).contains(&n) {
            self.core.present.index = n - 1;
        }
    }

    /// Open the projector on `display` and remember the choice.
    pub fn open_projector(&mut self, display: usize) {
        self.projector = Some(ProjectorState { display });
        self.core.cfg.output_display = i32::try_from(display).ok();
        self.core.save_cfg();
    }

    /// Close the projector.
    pub fn stop_projector(&mut self) {
        self.projector = None;
    }

    fn drain(&mut self) {
        while let Ok(m) = self.rx.try_recv() {
            let s = &self.core.strings;
            match m {
                Msg::Index(searcher) => {
                    log::info!("searcher ready with {} hymns", searcher.len());
                    self.status.library = s.status_library_ready.clone();
                    self.set_searcher(searcher);
                }
                Msg::Synced(Ok(searcher)) => {
                    self.status.syncing = false;
                    self.status.sync = s.status_synced_fmt.replace("{}", &searcher.len().to_string());
                    self.set_searcher(searcher);
                }
                Msg::Synced(Err(e)) => {
                    self.status.syncing = false;
                    self.status.sync = s.status_sync_failed_fmt.replace("{}", &e);
                }
                Msg::Reindexed(Ok(searcher)) => {
                    self.ui.library_status = s.status_indexed_fmt.replacen("{}", &searcher.len().to_string(), 1);
                    self.set_searcher(searcher);
                }
                Msg::Reindexed(Err(e)) => self.ui.library_status = s.status_indexing_failed_fmt.replacen("{}", &e, 1),
                Msg::Update(u) => self.status.update = u,
                Msg::Download(ev) => self.apply_download(ev),
            }
        }
    }

    fn set_searcher(&mut self, searcher: Searcher) {
        self.core.searcher = Some(searcher);
        self.ui.last_query = None;
    }

    fn apply_download(&mut self, ev: DownloadEvent) {
        let v = &mut self.ui.video;
        match ev {
            DownloadEvent::Resolving => v.phase = VideoPhase::Resolving,
            DownloadEvent::Title(t) => v.title = t,
            DownloadEvent::Progress(p) => {
                v.phase = VideoPhase::Downloading;
                v.percent = p.percent;
                v.speed = p.speed;
                v.eta = p.eta;
            }
            DownloadEvent::Done { .. } => {
                v.phase = VideoPhase::Done;
                v.percent = 100.0;
            }
            DownloadEvent::Failed { message } => {
                v.phase = VideoPhase::Failed;
                v.message = message;
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain();
        if self.ui.last_query.as_deref() != Some(self.ui.query.as_str()) {
            self.run_search();
        }
        shortcuts::handle(ui, self);
        let s = self.core.strings.clone();
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(&s.app_title);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(&s.settings).clicked() {
                        self.ui.settings_open = true;
                    }
                    if ui.button(&s.videos).clicked() {
                        self.ui.videos_open = true;
                    }
                });
            });
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status.library);
                ui.separator();
                ui.label(&self.status.update);
                if self.projector.is_some() {
                    ui.separator();
                    ui.colored_label(style::ACCENT, &s.live);
                }
            });
        });
        egui::Panel::left("search").resizable(true).default_size(340.0).show(ui, |ui| search_panel::show(ui, self));
        egui::CentralPanel::default().show(ui, |ui| {
            let card = egui::Frame::group(ui.style()).inner_margin(12.0).corner_radius(8.0);
            let gap = ui.spacing().item_spacing.y;
            let size = egui::vec2(ui.available_width(), (ui.available_height() - gap) / 2.0);
            let layout = egui::Layout::top_down(egui::Align::Min);
            for (i, body) in [preview::show as fn(&mut egui::Ui, &mut App), live::show].into_iter().enumerate() {
                ui.push_id(i, |ui| {
                    ui.allocate_ui_with_layout(size, layout, |ui| {
                        ui.set_clip_rect(ui.max_rect());
                        card.show(ui, |ui| {
                            ui.set_min_size(ui.available_size());
                            body(ui, self);
                        });
                    });
                });
            }
        });
        settings::show(ui.ctx(), self);
        videos::show(ui.ctx(), self);
        projector::show(ui.ctx(), self);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use sda_core::index::refresh_index;
    use sda_core::search::Searcher;

    use super::App;

    fn app() -> App {
        let entries = refresh_index(Path::new("../sda-core/tests/fixtures"), "test", &[]);
        App::for_test(Searcher::new(entries))
    }

    #[test]
    fn search_select_and_go_live_drive_presentation_state() {
        let mut app = app();
        app.ui.query = "664".into();
        app.run_search();
        assert_eq!(app.selected_entry().unwrap().number.as_deref(), Some("664b"));

        app.ui.query.clear();
        app.run_search();
        assert_eq!(app.ui.hits.len(), 4);
        app.move_selection(10);
        assert_eq!(app.ui.selected, Some(3));
        app.move_selection(-1);
        assert_eq!(app.ui.selected, Some(2));

        app.go_live(99);
        let count = app.core.present.slide_count();
        assert!(count > 1);
        assert_eq!(app.core.present.index, count - 1);
        app.live_jump(1);
        assert_eq!(app.core.present.index, 0);
        app.live_jump(count + 1);
        assert_eq!(app.core.present.index, 0);
        app.live_next();
        assert_eq!(app.core.present.index, 1);
        app.live_blank();
        assert!(app.core.present.blank);
    }
}
