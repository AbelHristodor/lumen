use crate::model::HymnEntry;
use crate::pptx::extract;
use log::warn;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

/// Recursively find all .pptx files under `root`, skipping `~$` lock files.
pub fn crawl_pptx_paths(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| {
            let name = match p.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => return false,
            };
            name.to_ascii_lowercase().ends_with(".pptx") && !name.starts_with("~$")
        })
        .collect()
}

fn mtime_secs(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// On-disk cache format version. Bump this whenever the parser or entry layout
/// changes in a way that makes previously-cached entries stale (e.g. improved
/// title extraction). A mismatch makes `load_cache` return `None`, forcing a
/// full re-parse even though the underlying .pptx mtimes are unchanged.
pub const CACHE_VERSION: u32 = 7;

#[derive(serde::Serialize, serde::Deserialize)]
struct CacheFile {
    version: u32,
    entries: Vec<HymnEntry>,
}

/// Load a cached index from `cache_path`. Returns `None` if missing, corrupt,
/// or written by a different `CACHE_VERSION`.
pub fn load_cache(cache_path: &Path) -> Option<Vec<HymnEntry>> {
    let bytes = std::fs::read(cache_path).ok()?;
    let cache: CacheFile = bincode::deserialize(&bytes).ok()?;
    if cache.version != CACHE_VERSION {
        return None;
    }
    Some(cache.entries)
}

/// Persist the index to `cache_path` (best-effort; errors are returned).
pub fn save_cache(cache_path: &Path, entries: &[HymnEntry]) -> anyhow::Result<()> {
    let cache = CacheFile {
        version: CACHE_VERSION,
        entries: entries.to_vec(),
    };
    let bytes = bincode::serialize(&cache)?;
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(cache_path, bytes)?;
    Ok(())
}

/// Build the index for every .pptx under `root`. A cached entry is reused when
/// its path and mtime are unchanged; everything else is re-parsed. Unparseable
/// files are logged and skipped so one bad file never aborts the crawl. Output
/// follows crawl order.
pub fn refresh_index(root: &Path, library: &str, cached: &[HymnEntry]) -> Vec<HymnEntry> {
    let by_path: HashMap<&Path, &HymnEntry> =
        cached.iter().map(|e| (e.path.as_path(), e)).collect();
    let mut out = Vec::new();
    for path in crawl_pptx_paths(root) {
        let mtime = mtime_secs(&path);
        if let Some(hit) = by_path.get(path.as_path()).filter(|e| e.mtime == mtime) {
            out.push((*hit).clone());
            continue;
        }
        match extract(&path) {
            Ok(parsed) => out.push(HymnEntry {
                number: parsed.number,
                title: parsed.title,
                slides: parsed.slides,
                path,
                library: library.to_string(),
                mtime,
            }),
            Err(err) => warn!("skip {}: {err}", path.display()),
        }
    }
    out
}
