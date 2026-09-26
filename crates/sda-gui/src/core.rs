use std::path::PathBuf;
use std::sync::mpsc::Sender;

use sda_core::downloader;
use sda_core::i18n::{Language, Strings};
use sda_core::library::{self, Config};
use sda_core::present::PresentationState;
use sda_core::refresh;
use sda_core::search::Searcher;
use sda_core::sync::SyncOutcome;
use sda_core::theme::{store, Theme};
use sda_core::update::{self, UpdateOutcome};

use crate::app::Msg;

/// Persistent app data and the background workers that feed the UI.
pub struct Core {
    pub cfg: Config,
    pub cfg_path: Option<PathBuf>,
    pub language: Language,
    pub strings: Strings,
    pub searcher: Option<Searcher>,
    pub present: PresentationState,
    pub themes: Vec<Theme>,
    pub active_theme: Theme,
}

impl Core {
    /// Read config, register the default library, pick the language and load themes.
    pub fn load() -> Core {
        let mut cfg_path = library::config_path();
        let mut cfg = match cfg_path.as_deref().map(Config::load) {
            Some(Ok(cfg)) => cfg,
            Some(Err(e)) => {
                log::warn!("config unreadable, using defaults and not saving over it: {e}");
                cfg_path = None;
                Config::default()
            }
            None => Config::default(),
        };
        refresh::register_default_library(&mut cfg);
        let language = match cfg.language.as_deref() {
            Some(code) => Language::from_code(code),
            None => sys_locale::get_locale().map_or(Language::En, |l| Language::from_locale(&l)),
        };
        let mut core = Core {
            strings: Strings::for_language(language),
            language,
            cfg,
            cfg_path,
            searcher: None,
            present: PresentationState::default(),
            themes: Vec::new(),
            active_theme: Theme::default(),
        };
        core.reload_themes();
        core.active_theme = match (library::themes_dir(), core.cfg.active_theme.as_deref()) {
            (Some(dir), Some(name)) => store::load_theme(&dir, name).unwrap_or_else(|e| {
                log::warn!("active theme {name} failed to load: {e}");
                Theme::default()
            }),
            _ => Theme::default(),
        };
        core
    }

    /// Write config.toml. The only place that writes it.
    pub fn save_cfg(&self) {
        let Some(path) = &self.cfg_path else { return };
        if let Err(e) = self.cfg.save(path) {
            log::warn!("saving config failed: {e}");
        }
    }

    /// Switch the UI language and persist it.
    pub fn set_language(&mut self, lang: Language) {
        self.language = lang;
        self.strings = Strings::for_language(lang);
        self.cfg.language = Some(lang.code().to_string());
        self.save_cfg();
    }

    /// Reread every saved theme from disk.
    pub fn reload_themes(&mut self) {
        self.themes = library::themes_dir()
            .map_or_else(|| vec![Theme::default()], |dir| store::list_themes(&dir));
    }

    /// Make a theme the one used on the projector and persist its name.
    pub fn set_active_theme(&mut self, t: Theme) {
        self.cfg.active_theme = Some(t.name.clone());
        self.active_theme = t;
        self.save_cfg();
    }

    /// Index from disk, sync the default library, then check for updates.
    pub fn spawn_boot(&self, tx: Sender<Msg>, ctx: egui::Context) {
        let cfg = self.cfg.clone();
        let s = self.strings.clone();
        std::thread::spawn(move || {
            let send = |m: Msg| {
                let _ = tx.send(m);
                ctx.request_repaint();
            };
            if refresh::default_library_present() {
                send(Msg::Index(Searcher::new(refresh::load_local(cfg.clone()))));
                if refresh::sync_default(&cfg) == Some(SyncOutcome::Downloaded) {
                    send(Msg::Index(Searcher::new(refresh::load_local(cfg))));
                }
            } else {
                send(Msg::Index(Searcher::new(refresh::load_library(cfg, false))));
            }
            let msg = match update::check_and_stage_update() {
                Ok(UpdateOutcome::UpToDate) => s.update_up_to_date,
                Ok(UpdateOutcome::Updated { version }) => {
                    s.update_staged_fmt.replace("{}", &version)
                }
                Ok(UpdateOutcome::CheckUnavailable) => s.update_unavailable,
                Err(e) => {
                    log::warn!("update check failed: {e}");
                    s.update_failed
                }
            };
            send(Msg::Update(msg));
        });
    }

    /// Wipe the default library and cache, then download and index from scratch.
    pub fn spawn_force_sync(&self, tx: Sender<Msg>, ctx: egui::Context) {
        let cfg = self.cfg.clone();
        std::thread::spawn(move || {
            let msg = match refresh::force_clean() {
                Ok(()) => Msg::Synced(Ok(Searcher::new(refresh::load_library(cfg, true)))),
                Err(e) => Msg::Synced(Err(format!("force clean failed: {e}"))),
            };
            let _ = tx.send(msg);
            ctx.request_repaint();
        });
    }

    /// Reindex local libraries after the folder list changed.
    pub fn spawn_reindex(&self, tx: Sender<Msg>, ctx: egui::Context) {
        let cfg = self.cfg.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Reindexed(Ok(Searcher::new(refresh::load_local(cfg)))));
            ctx.request_repaint();
        });
    }

    /// Download a video into `dir`, streaming progress events.
    pub fn spawn_download(&self, url: String, dir: PathBuf, tx: Sender<Msg>, ctx: egui::Context) {
        std::thread::spawn(move || {
            downloader::download(&url, &dir, |ev| {
                let _ = tx.send(Msg::Download(ev));
                ctx.request_repaint();
            });
        });
    }
}
