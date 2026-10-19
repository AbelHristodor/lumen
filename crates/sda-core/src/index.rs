use crate::model::Hymn;
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
pub const CACHE_VERSION: u32 = 8;

#[derive(serde::Serialize, serde::Deserialize)]
struct CacheFile {
    version: u32,
    entries: Vec<Hymn>,
}

/// Load a cached index from `cache_path`. Returns `None` if missing, corrupt,
/// or written by a different `CACHE_VERSION`.
pub fn load_cache(cache_path: &Path) -> Option<Vec<Hymn>> {
    let bytes = std::fs::read(cache_path).ok()?;
    let cache: CacheFile = bincode::deserialize(&bytes).ok()?;
    if cache.version != CACHE_VERSION {
        return None;
    }
    Some(cache.entries)
}

/// Persist the index to `cache_path` (best-effort; errors are returned).
pub fn save_cache(cache_path: &Path, entries: &[Hymn]) -> anyhow::Result<()> {
    let cache = CacheFile {
        version: CACHE_VERSION,
        entries: entries.to_vec(),
    };
    let bytes = bincode::serialize(&cache)?;
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Write next to the target and rename so a crash never leaves a half file.
    let tmp = cache_path.with_extension("bin.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, cache_path)?;
    Ok(())
}

/// Build the index for every .pptx under `root`. A cached entry is reused when
/// its path and mtime are unchanged; everything else is re-parsed in parallel.
/// Unparseable files are logged and skipped so one bad file never aborts the
/// crawl. Output follows crawl order.
pub fn refresh_index(root: &Path, library: &str, cached: &[Hymn]) -> Vec<Hymn> {
    let by_path: HashMap<&Path, &Hymn> = cached.iter().map(|e| (e.path.as_path(), e)).collect();
    let paths = crawl_pptx_paths(root);
    let mut out: Vec<Option<Hymn>> = vec![None; paths.len()];
    let mut misses: Vec<(usize, PathBuf, i64)> = Vec::new();
    for (i, path) in paths.into_iter().enumerate() {
        let mtime = mtime_secs(&path);
        match by_path.get(path.as_path()).filter(|e| e.mtime == mtime) {
            Some(hit) => out[i] = Some((*hit).clone()),
            None => misses.push((i, path, mtime)),
        }
    }
    for (i, entry) in parse_all(&misses, library) {
        out[i] = entry;
    }
    out.into_iter().flatten().collect()
}

/// Parse every miss, spreading the work over all CPUs. Unzipping and XML
/// parsing hundreds of decks is the cold-start bottleneck.
fn parse_all(misses: &[(usize, PathBuf, i64)], library: &str) -> Vec<(usize, Option<Hymn>)> {
    if misses.is_empty() {
        return Vec::new();
    }
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(misses.len());
    let chunk = misses.len().div_ceil(threads);
    std::thread::scope(|scope| {
        let handles: Vec<_> = misses
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .map(|(i, p, m)| (*i, parse_one(p, *m, library)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("parse thread panicked"))
            .collect()
    })
}

fn parse_one(path: &Path, mtime: i64, library: &str) -> Option<Hymn> {
    match extract(path) {
        Ok(parsed) => Some(Hymn {
            number: parsed.number,
            title: parsed.title,
            slides: parsed.slides,
            path: path.to_path_buf(),
            library: library.to_string(),
            mtime,
        }),
        Err(err) => {
            warn!("skip {}: {err}", path.display());
            None
        }
    }
}
