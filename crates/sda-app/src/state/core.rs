//! Persistent app data: config, language, themes, history, overrides and
//! download records. Loaded once at boot. No workers here, see `workers.rs`.

use crate::ui_theme::{self, AppTheme};
use sda_core::asis::Converter;
use sda_core::downloader::DownloadRecord;
use sda_core::history::{HistoryEntry, VerseHistoryEntry};
use sda_core::i18n::{Language, Strings};
use sda_core::library::{self, Config};
use sda_core::overrides::Overrides;
use sda_core::refresh;
use sda_core::search::Searcher;
use sda_core::store::{json_load, json_save};
use sda_core::theme::{self, Theme, store};
use std::path::{Path, PathBuf};

/// Where the app keeps its files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub themes_dir: PathBuf,
    pub history: PathBuf,
    pub verse_history: PathBuf,
    pub overrides: PathBuf,
    pub downloads: PathBuf,
    pub window: PathBuf,
    pub bibles_dir: PathBuf,
    pub asis_cache: PathBuf,
}

impl Paths {
    /// Standard app folders. Falls back to the OS temp dir when the platform
    /// gives no project dirs.
    pub fn standard() -> Paths {
        let tmp = std::env::temp_dir().join("lumen");
        let or = |p: Option<PathBuf>, name: &str| p.unwrap_or_else(|| tmp.join(name));
        Paths {
            themes_dir: or(library::themes_dir(), "themes"),
            history: or(library::history_path(), "history.json"),
            verse_history: or(library::verse_history_path(), "verse_history.json"),
            overrides: or(library::overrides_path(), "overrides.json"),
            downloads: or(library::downloads_path(), "downloads.json"),
            window: or(library::window_path(), "window.json"),
            bibles_dir: or(library::bibles_dir(), "bibles"),
            asis_cache: or(library::asis_cache_dir(), "asis"),
        }
    }

    /// Every file under `root`. Used by tests.
    pub fn under(root: &Path) -> Paths {
        Paths {
            themes_dir: root.join("themes"),
            history: root.join("history.json"),
            verse_history: root.join("verse_history.json"),
            overrides: root.join("overrides.json"),
            downloads: root.join("downloads.json"),
            window: root.join("window.json"),
            bibles_dir: root.join("bibles"),
            asis_cache: root.join("asis"),
        }
    }
}

pub struct Core {
    pub cfg: Config,
    /// `None` means the config is read only, for example when it failed to parse.
    pub cfg_path: Option<PathBuf>,
    pub paths: Paths,
    pub language: Language,
    pub strings: Strings,
    pub searcher: Option<Searcher>,
    pub themes: Vec<Theme>,
    pub active_theme: Theme,
    pub history: Vec<HistoryEntry>,
    pub verse_history: Vec<VerseHistoryEntry>,
    pub overrides: Overrides,
    pub downloads: Vec<DownloadRecord>,
    pub converter: Option<Converter>,
    /// Bumped every time an index worker starts. A result that carries an
    /// older value lost to a newer start and is dropped.
    pub index_gen: u64,
}

impl Core {
    /// Read config, register the default library, pick the language, load
    /// themes and the small JSON state files.
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
        Core::with(cfg, cfg_path, Paths::standard(), language)
    }

    /// Build a core over explicit paths. Themes and JSON state are read from
    /// `paths`. The active theme comes from `cfg.active_theme`.
    pub fn with(cfg: Config, cfg_path: Option<PathBuf>, paths: Paths, language: Language) -> Core {
        let mut core = Core {
            strings: Strings::for_language(language),
            language,
            history: json_load(&paths.history),
            verse_history: json_load(&paths.verse_history),
            overrides: json_load(&paths.overrides),
            downloads: json_load(&paths.downloads),
            cfg,
            cfg_path,
            paths,
            searcher: None,
            themes: Vec::new(),
            active_theme: Theme::default(),
            converter: None,
            index_gen: 0,
        };
        theme::bundled::seed(&core.paths.themes_dir);
        core.reload_themes();
        core.active_theme = match core.cfg.active_theme.as_deref() {
            Some(name) => store::load_theme(&core.paths.themes_dir, name).unwrap_or_else(|e| {
                log::warn!("active theme {name} failed to load: {e}");
                Theme::default()
            }),
            None => Theme::default(),
        };
        ui_theme::set_app_theme(AppTheme::from_code(core.cfg.app_theme.as_deref()));
        core
    }

    /// Write config.toml. The only place that writes it.
    pub fn save_cfg(&self) {
        let Some(path) = &self.cfg_path else { return };
        if let Err(e) = self.cfg.save(path) {
            log::warn!("saving config failed: {e}");
        }
    }

    pub fn save_history(&self) {
        if let Err(e) = json_save(&self.paths.history, &self.history) {
            log::warn!("saving history failed: {e}");
        }
    }

    pub fn save_verse_history(&self) {
        if let Err(e) = json_save(&self.paths.verse_history, &self.verse_history) {
            log::warn!("saving verse history failed: {e}");
        }
    }

    pub fn save_overrides(&self) {
        if let Err(e) = json_save(&self.paths.overrides, &self.overrides) {
            log::warn!("saving overrides failed: {e}");
        }
    }

    pub fn save_downloads(&self) {
        if let Err(e) = json_save(&self.paths.downloads, &self.downloads) {
            log::warn!("saving downloads failed: {e}");
        }
    }

    /// Switch the UI language and persist it.
    pub fn set_language(&mut self, lang: Language) {
        self.language = lang;
        self.strings = Strings::for_language(lang);
        self.cfg.language = Some(lang.code().to_string());
        self.save_cfg();
    }

    /// Switch the operator window palette and persist it.
    pub fn set_app_theme(&mut self, theme: AppTheme) {
        ui_theme::set_app_theme(theme);
        self.cfg.app_theme = Some(theme.code().to_string());
        self.save_cfg();
    }

    /// Reread every saved theme from disk.
    pub fn reload_themes(&mut self) {
        self.themes = store::list_themes(&self.paths.themes_dir);
    }

    /// Make a theme the one used on the projector and persist its name.
    pub fn set_active_theme(&mut self, t: Theme) {
        self.cfg.active_theme = Some(t.name.clone());
        self.active_theme = t;
        self.save_cfg();
    }

    /// Mark a new index worker start and return its generation.
    pub fn next_index_gen(&mut self) -> u64 {
        self.index_gen += 1;
        self.index_gen
    }

    /// True when a result from generation `g` is still the newest one.
    pub fn index_current(&self, g: u64) -> bool {
        g >= self.index_gen
    }

    /// The loaded theme with this name, if any.
    pub fn theme_named(&self, name: &str) -> Option<&Theme> {
        self.themes.iter().find(|t| t.name == name)
    }
}
