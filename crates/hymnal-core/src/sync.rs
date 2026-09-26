//! Keeps the built-in hymn library current from a `hymns.zip` release asset.
//! One HEAD request compares the asset's ETag with the last one we extracted;
//! only a changed ETag triggers a download. No GitHub API, so no rate limits.

use anyhow::{Context, Result};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOutcome {
    /// A new zip was downloaded and extracted into the library folder.
    Downloaded,
    /// The library folder already holds the current asset.
    Unchanged,
}

/// Ensure `dest` holds the hymns published at `url`. `stamp` stores the ETag
/// of the last extracted zip. Extraction goes to a sibling temp folder and is
/// swapped in atomically so a failed download never leaves a half library.
pub fn sync_hymns(url: &str, dest: &Path, stamp: &Path) -> Result<SyncOutcome> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(300))
        .build();

    let head = agent.head(url).call().with_context(|| format!("HEAD {url}"))?;
    let etag = head.header("etag").unwrap_or_default().to_string();
    let last = std::fs::read_to_string(stamp).unwrap_or_default();
    if !etag.is_empty() && etag == last.trim() && dest.is_dir() {
        return Ok(SyncOutcome::Unchanged);
    }

    let parent = dest.parent().context("library folder has no parent")?;
    std::fs::create_dir_all(parent)?;
    let zip_path = parent.join("hymns.zip.part");
    let tmp_dir = parent.join("hymns.tmp");

    let result = (|| -> Result<()> {
        let resp = agent.get(url).call().with_context(|| format!("GET {url}"))?;
        let mut file = std::fs::File::create(&zip_path)?;
        std::io::copy(&mut resp.into_reader(), &mut file)?;
        let _ = std::fs::remove_dir_all(&tmp_dir);
        extract_zip(&zip_path, &tmp_dir)?;
        if dest.exists() {
            std::fs::remove_dir_all(dest)?;
        }
        std::fs::rename(&tmp_dir, dest)?;
        std::fs::write(stamp, &etag)?;
        Ok(())
    })();

    let _ = std::fs::remove_file(&zip_path);
    let _ = std::fs::remove_dir_all(&tmp_dir);
    result.map(|_| SyncOutcome::Downloaded)
}

/// Unpack `zip` into `dest`. Entry paths are sanitized by the `zip` crate, so
/// an archive cannot write outside `dest`.
pub fn extract_zip(zip: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(zip).with_context(|| format!("open {}", zip.display()))?;
    let mut archive = zip::ZipArchive::new(file).context("read zip")?;
    archive.extract(dest).context("extract zip")?;
    Ok(())
}
