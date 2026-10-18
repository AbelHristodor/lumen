//! Library-loading pipeline shared by app boot, force sync and folder edits:
//! keep the built-in library current, index every enabled library, maintain
//! the on-disk cache.

use crate::index::{load_cache, refresh_index, save_cache};
use crate::library::{
    default_library, default_library_dir, hymns_stamp_path, index_cache_path, legacy_clone_dir,
    Config,
};
use crate::model::Hymn;
use crate::sync::{sync_hymns, SyncOutcome};
use log::{debug, info, warn};
use std::path::Path;

/// Delete the built-in library folder, its ETag stamp and the index cache.
/// Missing paths are not an error.
pub fn force_clean_paths(
    hymns_dir: Option<&Path>,
    stamp: Option<&Path>,
    cache_file: Option<&Path>,
) -> anyhow::Result<()> {
    if let Some(dir) = hymns_dir.filter(|d| d.exists()) {
        info!("force_clean: removing {}", dir.display());
        std::fs::remove_dir_all(dir)?;
    }
    for file in [stamp, cache_file]
        .into_iter()
        .flatten()
        .filter(|f| f.exists())
    {
        info!("force_clean: removing {}", file.display());
        std::fs::remove_file(file)?;
    }
    Ok(())
}

/// Force-clean using the standard app directories.
pub fn force_clean() -> anyhow::Result<()> {
    force_clean_paths(
        default_library_dir().as_deref(),
        hymns_stamp_path().as_deref(),
        index_cache_path().as_deref(),
    )
}

/// Make sure `cfg` carries the built-in library at its current location. Adds
/// the entry when absent and repairs the path of one left by an older version
/// that pointed into the git clone. Returns false if no data dir is available.
pub fn register_default_library(cfg: &mut Config) -> bool {
    let Some(default) = default_library() else {
        warn!("could not determine default library dir");
        return false;
    };
    match cfg.libraries.iter_mut().find(|l| l.managed_by_git) {
        Some(managed) => {
            if managed.path != default.path {
                debug!(
                    "repairing default library path {} -> {}",
                    managed.path, default.path
                );
                managed.path = default.path;
                managed.name = default.name;
            }
        }
        None => {
            debug!("registering default library at {}", default.path);
            cfg.libraries.push(default);
        }
    }
    true
}

/// Download the hymns zip if its ETag changed. Network step: keep it off the
/// UI thread. Returns `None` on error or when no data dir exists.
pub fn sync_default(cfg: &Config) -> Option<SyncOutcome> {
    let dir = default_library_dir()?;
    let stamp = hymns_stamp_path()?;
    info!("syncing hymns from {} -> {}", cfg.hymns_url, dir.display());
    match sync_hymns(&cfg.hymns_url, &dir, &stamp) {
        Ok(outcome) => {
            info!("sync ok: {outcome:?}");
            if outcome == SyncOutcome::Downloaded {
                if let Some(old) = legacy_clone_dir().filter(|p| p.exists()) {
                    info!("removing legacy clone {}", old.display());
                    let _ = std::fs::remove_dir_all(old);
                }
            }
            Some(outcome)
        }
        Err(e) => {
            warn!("hymns sync failed: {e}");
            None
        }
    }
}

/// Index from disk only. Use on boot for instant search; follow with
/// [`sync_default`] and re-index when it reports a download.
pub fn load_local(mut cfg: Config) -> Vec<Hymn> {
    register_default_library(&mut cfg);
    index_enabled(&cfg, false)
}

/// True once the built-in library has been downloaded at least once.
pub fn default_library_present() -> bool {
    default_library_dir().is_some_and(|d| d.is_dir())
}

/// Sync, then index every enabled library. `force` ignores the cache. Use for
/// first run and force sync, not the fast boot path.
pub fn load_library(mut cfg: Config, force: bool) -> Vec<Hymn> {
    let _ = sync_default(&cfg);
    register_default_library(&mut cfg);
    index_enabled(&cfg, force)
}

fn index_enabled(cfg: &Config, force: bool) -> Vec<Hymn> {
    let cache = index_cache_path();
    let cached = if force {
        Vec::new()
    } else {
        cache
            .as_ref()
            .and_then(|p| load_cache(p))
            .unwrap_or_default()
    };
    debug!("loaded {} cached entries (force={force})", cached.len());

    let mut entries = Vec::new();
    for lib in cfg.libraries.iter().filter(|l| l.enabled) {
        let before = entries.len();
        entries.extend(refresh_index(Path::new(&lib.path), &lib.name, &cached));
        info!(
            "indexed '{}' at {} -> {} hymns",
            lib.name,
            lib.path,
            entries.len() - before
        );
    }
    if entries.is_empty() && !force {
        warn!(
            "no entries indexed; falling back to {} cached",
            cached.len()
        );
        entries = cached;
    }
    info!("total {} hymns indexed", entries.len());

    if let Some(p) = cache {
        if let Err(e) = save_cache(&p, &entries) {
            warn!("failed to write index cache: {e}");
        }
    }
    entries
}
