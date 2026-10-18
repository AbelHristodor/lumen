use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Release asset holding the hymn library, downloadable without the GitHub API.
pub const DEFAULT_HYMNS_URL: &str =
    "https://github.com/AbelHristodor/sda_manager/releases/latest/download/hymns.zip";

fn default_hymns_url() -> String {
    DEFAULT_HYMNS_URL.to_string()
}

/// Display name of the built-in library.
pub const DEFAULT_LIBRARY_NAME: &str = "Imnuri Creștine";

/// One library = a folder of .pptx files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Library {
    pub name: String,
    pub path: String,
    pub enabled: bool,
    /// True for the built-in library the app downloads and refreshes itself.
    pub managed_by_git: bool,
}

/// Persisted application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// Where the built-in library zip is fetched from. Old configs carried a
    /// `default_repo_url` git URL instead; that key is ignored and this default applies.
    #[serde(default = "default_hymns_url")]
    pub hymns_url: String,
    pub libraries: Vec<Library>,
    /// User's chosen download folder. `None` => OS Downloads directory.
    #[serde(default)]
    pub download_dir: Option<String>,
    /// Selected UI language code ("en"/"it"/"ro"). `None` => not yet chosen
    /// (detect from OS locale on first run).
    #[serde(default)]
    pub language: Option<String>,
    /// Name of the active projection theme. `None` => built-in Default.
    #[serde(default)]
    pub active_theme: Option<String>,
    /// Index of the last-used output display. `None` => auto-pick.
    #[serde(default)]
    pub output_display: Option<i32>,
    /// Id of the chosen Bible translation. `None` => none picked yet.
    #[serde(default)]
    pub bible_version: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            hymns_url: default_hymns_url(),
            libraries: Vec::new(),
            download_dir: None,
            language: None,
            active_theme: None,
            output_display: None,
            bible_version: None,
        }
    }
}

impl Config {
    pub fn to_toml(&self) -> anyhow::Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn from_toml(text: &str) -> anyhow::Result<Config> {
        Ok(toml::from_str(text)?)
    }

    /// Load config from `path`, or return the default if it doesn't exist.
    pub fn load(path: &Path) -> anyhow::Result<Config> {
        match std::fs::read_to_string(path) {
            Ok(text) => Config::from_toml(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(Config::default())
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_toml()?)?;
        Ok(())
    }
}

fn dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("org", "hymnal", "HymnFinder")
}

pub fn config_path() -> Option<PathBuf> {
    dirs().map(|d| d.config_dir().join("config.toml"))
}

/// Folder the built-in library zip is extracted into.
pub fn default_library_dir() -> Option<PathBuf> {
    dirs().map(|d| d.data_dir().join("hymns"))
}

/// ETag of the last downloaded hymns zip, next to the library folder.
pub fn hymns_stamp_path() -> Option<PathBuf> {
    dirs().map(|d| d.data_dir().join("hymns.etag"))
}

/// Where versions before the zip sync cloned the whole app repo. Deleted once
/// the zip library is in place.
pub fn legacy_clone_dir() -> Option<PathBuf> {
    dirs().map(|d| d.data_dir().join("default-library"))
}

/// The built-in library entry, or `None` if the data directory can't be
/// determined. Single source of truth shared by the indexer and Settings UI.
pub fn default_library() -> Option<Library> {
    default_library_dir().map(|dir| Library {
        name: DEFAULT_LIBRARY_NAME.to_string(),
        path: dir.to_string_lossy().to_string(),
        enabled: true,
        managed_by_git: true,
    })
}

pub fn index_cache_path() -> Option<PathBuf> {
    dirs().map(|d| d.cache_dir().join("index.bin"))
}

pub fn tools_dir() -> Option<PathBuf> {
    dirs().map(|d| d.data_dir().join("tools"))
}

pub fn themes_dir() -> Option<PathBuf> {
    dirs().map(|d| d.config_dir().join("themes"))
}

/// Folder holding downloaded Bible translations.
pub fn bibles_dir() -> Option<PathBuf> {
    dirs().map(|d| d.data_dir().join("bibles"))
}

/// Recently opened hymns.
pub fn history_path() -> Option<PathBuf> {
    dirs().map(|d| d.data_dir().join("history.json"))
}

/// Record of finished video downloads.
pub fn downloads_path() -> Option<PathBuf> {
    dirs().map(|d| d.data_dir().join("downloads.json"))
}

/// Per hymn user overrides.
pub fn overrides_path() -> Option<PathBuf> {
    dirs().map(|d| d.data_dir().join("overrides.json"))
}

/// Saved main window geometry.
pub fn window_path() -> Option<PathBuf> {
    dirs().map(|d| d.data_dir().join("window.json"))
}

/// Rendered slide images for hymns shown as is.
pub fn asis_cache_dir() -> Option<PathBuf> {
    dirs().map(|d| d.cache_dir().join("asis"))
}

/// The configured download directory, else the OS Downloads folder, else home.
pub fn downloads_dir(cfg: &Config) -> PathBuf {
    if let Some(d) = &cfg.download_dir {
        return PathBuf::from(d);
    }
    directories::UserDirs::new()
        .and_then(|u| u.download_dir().map(Path::to_path_buf).or_else(|| Some(u.home_dir().to_path_buf())))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Whether a library folder is currently reachable on disk. Used for the
/// Settings "unavailable" marker; an unreachable folder is simply skipped at
/// index time (the crawl yields nothing) rather than being an error.
pub fn library_available(path: &str) -> bool {
    Path::new(path).is_dir()
}

/// Set the `enabled` flag of the library whose `path` matches. Works on any
/// library, including the default — the default may be disabled (just not
/// removed). No-op if no library has that path.
pub fn set_library_enabled(cfg: &mut Config, path: &str, enabled: bool) {
    for lib in cfg.libraries.iter_mut() {
        if lib.path == path {
            lib.enabled = enabled;
        }
    }
}

/// Canonicalize `path` to an absolute, symlink-resolved form for stable
/// comparison; falls back to the input as-is if canonicalization fails (e.g.
/// the folder is on an unmounted drive).
fn canonical_string(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

/// Add a user folder as a `Library`. The display name is the folder's last
/// path component (falling back to the full path). `managed_by_git = false`,
/// `enabled = true`. The stored path is the canonicalized form. Returns `Err`
/// if the folder is already present (compared canonically) or does not exist.
pub fn add_user_library(cfg: &mut Config, path: &Path) -> anyhow::Result<()> {
    if !path.is_dir() {
        anyhow::bail!("not a folder: {}", path.display());
    }
    let canon = canonical_string(path);
    let exists = cfg
        .libraries
        .iter()
        .any(|l| canonical_string(Path::new(&l.path)) == canon);
    if exists {
        anyhow::bail!("folder already added");
    }
    // Derive the display name from the canonical path so a messy input (e.g.
    // ".../Dup/./..") or a Windows `\\?\`-prefixed path doesn't yield a weird
    // name like "." or the verbatim prefix.
    let name = Path::new(&canon)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| canon.clone());
    cfg.libraries.push(Library {
        name,
        path: canon,
        enabled: true,
        managed_by_git: false,
    });
    Ok(())
}

/// Remove the library whose `path` matches. Refuses to remove a
/// `managed_by_git` entry (the default library is locked against removal).
/// No-op if no matching removable library is found.
pub fn remove_user_library(cfg: &mut Config, path: &str) {
    cfg.libraries
        .retain(|l| l.managed_by_git || l.path != path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_toml_round_trips() {
        let cfg = Config {
            hymns_url: "https://example.com/hymns.zip".into(),
            libraries: vec![
                Library {
                    name: "Imnuri".into(),
                    path: "/data/imnuri".into(),
                    enabled: true,
                    managed_by_git: true,
                },
                Library {
                    name: "MyHymns".into(),
                    path: "/tmp/myhymns".into(),
                    enabled: true,
                    managed_by_git: false,
                },
            ],
            download_dir: None,
            language: None,
            active_theme: None,
            output_display: None,
            bible_version: None,
        };
        let text = cfg.to_toml().unwrap();
        let back = Config::from_toml(&text).unwrap();
        assert_eq!(back.libraries.len(), 2);
        assert_eq!(back.libraries[0].name, "Imnuri");
        assert!(back.libraries[0].managed_by_git);
        assert!(back
            .libraries
            .iter()
            .any(|l| !l.managed_by_git && l.name == "MyHymns"));
    }

    #[test]
    fn default_config_points_at_release_zip() {
        assert_eq!(Config::default().hymns_url, DEFAULT_HYMNS_URL);
    }

    #[test]
    fn old_config_with_git_url_still_parses() {
        let cfg = Config::from_toml("default_repo_url = \"https://example.com/x.git\"\nlibraries = []\n").unwrap();
        assert_eq!(cfg.hymns_url, DEFAULT_HYMNS_URL);
    }

    #[test]
    fn config_persists_download_dir() {
        let cfg = Config {
            hymns_url: "https://example.com/hymns.zip".into(),
            libraries: vec![],
            download_dir: Some("/home/user/Videos".into()),
            language: None,
            active_theme: None,
            output_display: None,
            bible_version: None,
        };
        let back = Config::from_toml(&cfg.to_toml().unwrap()).unwrap();
        assert_eq!(back.download_dir, Some("/home/user/Videos".into()));
    }

    #[test]
    fn config_persists_language() {
        let cfg = Config {
            hymns_url: "https://example.com/hymns.zip".into(),
            libraries: vec![],
            download_dir: None,
            language: Some("ro".into()),
            active_theme: Some("Sunset".into()),
            output_display: Some(2),
            bible_version: None,
        };
        let back = Config::from_toml(&cfg.to_toml().unwrap()).unwrap();
        assert_eq!(back.language, Some("ro".into()));
        assert_eq!(back.active_theme, Some("Sunset".into()));
        assert_eq!(back.output_display, Some(2));
    }

    #[test]
    fn config_download_dir_defaults_to_none() {
        let cfg = Config::default();
        assert_eq!(cfg.download_dir, None);
    }

    #[test]
    fn library_available_true_for_existing_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(library_available(&dir.path().to_string_lossy()));
    }

    #[test]
    fn library_available_false_for_missing_dir() {
        assert!(!library_available("/no/such/path/hopefully/12345"));
    }

    #[test]
    fn set_library_enabled_flips_flag() {
        let mut cfg = Config {
            hymns_url: "x".into(),
            libraries: vec![Library {
                name: "U".into(), path: "/tmp/u".into(), enabled: true, managed_by_git: false,
            }],
            download_dir: None,
            language: None,
            active_theme: None,
            output_display: None,
            bible_version: None,
        };
        set_library_enabled(&mut cfg, "/tmp/u", false);
        assert!(!cfg.libraries[0].enabled);
        set_library_enabled(&mut cfg, "/tmp/u", true);
        assert!(cfg.libraries[0].enabled);
    }

    #[test]
    fn set_library_enabled_can_disable_default() {
        let mut cfg = Config {
            hymns_url: "x".into(),
            libraries: vec![Library {
                name: "Default".into(), path: "/data/default".into(), enabled: true, managed_by_git: true,
            }],
            download_dir: None,
            language: None,
            active_theme: None,
            output_display: None,
            bible_version: None,
        };
        set_library_enabled(&mut cfg, "/data/default", false);
        assert!(!cfg.libraries[0].enabled);
    }

    #[test]
    fn add_user_library_uses_folder_name_and_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("MyHymns");
        std::fs::create_dir(&sub).unwrap();
        let mut cfg = Config::default();
        add_user_library(&mut cfg, &sub).unwrap();
        assert_eq!(cfg.libraries.len(), 1);
        let lib = &cfg.libraries[0];
        assert_eq!(lib.name, "MyHymns");
        assert!(lib.enabled);
        assert!(!lib.managed_by_git);
    }

    #[test]
    fn add_user_library_rejects_duplicate_path() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("Dup");
        std::fs::create_dir(&sub).unwrap();
        let mut cfg = Config::default();
        add_user_library(&mut cfg, &sub).unwrap();
        let messy = sub.join(".").join("..").join("Dup");
        assert!(add_user_library(&mut cfg, &messy).is_err());
        assert_eq!(cfg.libraries.len(), 1);
    }

    #[test]
    fn add_user_library_rejects_non_directory() {
        let mut cfg = Config::default();
        assert!(add_user_library(&mut cfg, Path::new("/no/such/dir/xyz")).is_err());
        assert!(cfg.libraries.is_empty());
    }

    #[test]
    fn config_without_bible_version_parses() {
        let cfg = Config::from_toml("libraries = []\n").unwrap();
        assert_eq!(cfg.bible_version, None);
    }

    #[test]
    fn bible_version_round_trips() {
        let cfg = Config { bible_version: Some("kjv".into()), ..Config::default() };
        let back = Config::from_toml(&cfg.to_toml().unwrap()).unwrap();
        assert_eq!(back.bible_version, Some("kjv".into()));
    }

    #[test]
    fn remove_user_library_removes_user_but_not_default() {
        let mut cfg = Config {
            hymns_url: "x".into(),
            libraries: vec![
                Library { name: "Default".into(), path: "/data/default".into(), enabled: true, managed_by_git: true },
                Library { name: "Mine".into(), path: "/tmp/mine".into(), enabled: true, managed_by_git: false },
            ],
            download_dir: None,
            language: None,
            active_theme: None,
            output_display: None,
            bible_version: None,
        };
        remove_user_library(&mut cfg, "/tmp/mine");
        assert_eq!(cfg.libraries.len(), 1);
        remove_user_library(&mut cfg, "/data/default");
        assert_eq!(cfg.libraries.len(), 1);
        assert!(cfg.libraries[0].managed_by_git);
    }
}
