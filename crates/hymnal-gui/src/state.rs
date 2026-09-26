//! Shared UI-thread state. Every tab module gets a clone of `App` and wires
//! its Slint callbacks against it, so config is loaded once and saved in one
//! place, and no handler re-reads `config.toml`.

use crate::projector::{self, Display};
use crate::{AppWindow, ProjectorWindow};
use hymnal_core::downloader::DownloadEvent;
use hymnal_core::i18n::{Language, Strings};
use hymnal_core::library::{self, Config};
use hymnal_core::model::HymnEntry;
use hymnal_core::present::PresentationState;
use hymnal_core::search::Searcher;
use hymnal_core::theme::{store, Theme};
use log::warn;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};

/// Results handed back from worker threads. Workers build the `Searcher` so
/// the UI thread only swaps a handle.
pub enum Msg {
    Index(Searcher),
    Synced(Result<Searcher, String>),
    Reindexed(Result<Searcher, String>),
    Download(DownloadEvent),
}

#[derive(Clone)]
pub struct App {
    pub cfg: Rc<RefCell<Config>>,
    pub cfg_path: Option<PathBuf>,
    pub strings: Rc<RefCell<Strings>>,
    pub searcher: Rc<RefCell<Option<Searcher>>>,
    /// Library tab result row -> entry index inside the searcher.
    pub rows: Rc<RefCell<Vec<usize>>>,
    /// Control tab result row -> entry index inside the searcher.
    pub ctl_rows: Rc<RefCell<Vec<usize>>>,
    pub present: Rc<RefCell<PresentationState>>,
    pub projector: Rc<RefCell<Option<ProjectorWindow>>>,
    pub active_theme: Rc<RefCell<Theme>>,
    pub draft: Rc<RefCell<Theme>>,
    pub themes: Rc<RefCell<Vec<Theme>>>,
    pub families: Rc<Vec<String>>,
    pub displays: Rc<Vec<Display>>,
    pub tx: Sender<Msg>,
    pub download_in_flight: Rc<Cell<bool>>,
}

impl App {
    /// Load config and persisted choices. Returns the receiving end of the
    /// worker channel for the caller to poll.
    pub fn load() -> (App, Receiver<Msg>) {
        let cfg_path = library::config_path();
        let mut cfg = cfg_path
            .as_deref()
            .map(|p| {
                Config::load(p).unwrap_or_else(|e| {
                    warn!("config load failed ({e}); using defaults");
                    Config::default()
                })
            })
            .unwrap_or_default();
        hymnal_core::refresh::register_default_library(&mut cfg);

        let lang = cfg.language.as_deref().map(Language::from_code).unwrap_or_else(|| {
            sys_locale::get_locale()
                .map(|l| Language::from_locale(&l))
                .unwrap_or(Language::En)
        });

        let active_theme = cfg
            .active_theme
            .as_deref()
            .zip(library::themes_dir())
            .and_then(|(name, dir)| store::load_theme(&dir, name).ok())
            .unwrap_or_default();

        let (tx, rx) = mpsc::channel();
        let app = App {
            cfg: Rc::new(RefCell::new(cfg)),
            cfg_path,
            strings: Rc::new(RefCell::new(Strings::for_language(lang))),
            searcher: Rc::default(),
            rows: Rc::default(),
            ctl_rows: Rc::default(),
            present: Rc::default(),
            projector: Rc::default(),
            active_theme: Rc::new(RefCell::new(active_theme)),
            draft: Rc::default(),
            themes: Rc::default(),
            families: Rc::new(crate::fonts::families()),
            displays: Rc::new(projector::list_displays()),
            tx,
            download_in_flight: Rc::default(),
        };
        (app, rx)
    }

    pub fn language(&self) -> Language {
        self.cfg
            .borrow()
            .language
            .as_deref()
            .map(Language::from_code)
            .unwrap_or(Language::En)
    }

    /// Write the in-memory config to disk. The only place config.toml is written.
    pub fn save_cfg(&self) {
        let Some(p) = &self.cfg_path else { return };
        if let Err(e) = self.cfg.borrow().save(p) {
            warn!("failed to save config: {e}");
        }
    }

    /// Mutate config, then persist.
    pub fn update_cfg(&self, f: impl FnOnce(&mut Config)) {
        f(&mut self.cfg.borrow_mut());
        self.save_cfg();
    }

    /// Run `f` on the entry behind visible `row` of the given row map.
    pub fn with_entry<R>(
        &self,
        rows: &RefCell<Vec<usize>>,
        row: i32,
        f: impl FnOnce(&HymnEntry) -> R,
    ) -> Option<R> {
        if row < 0 {
            return None;
        }
        let guard = self.searcher.borrow();
        let s = guard.as_ref()?;
        let idx = *rows.borrow().get(row as usize)?;
        s.entry(idx).map(f)
    }

    /// Swap in a freshly built searcher and re-run the Library query.
    pub fn install_searcher(&self, ui: &AppWindow, s: Searcher) {
        *self.searcher.borrow_mut() = Some(s);
        ui.invoke_query_changed("".into());
    }
}
