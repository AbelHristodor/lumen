//! Keeps downloaded assets current. One HEAD request compares the asset's
//! ETag with the last one we stored. Only a changed ETag triggers a download.
//! No GitHub API, so no rate limits.

use anyhow::{Context, Result};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOutcome {
    /// A new copy was downloaded.
    Downloaded,
    /// The local copy already matches the server.
    Unchanged,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(300))
        .build()
}

/// Ensure `dest` holds the file published at `url`. `stamp` stores the ETag,
/// or Last-Modified when the server sends no ETag, of the last download. The
/// body goes to a `.part` sibling and is renamed over `dest` so a failed
/// download never leaves a half file. A missing `dest` always downloads.
pub fn sync_file(url: &str, dest: &Path, stamp: &Path) -> Result<SyncOutcome> {
    sync_file_if(url, dest, stamp, dest.exists())
}

/// `present` tells whether the local result of a previous download still
/// exists. For a plain file that is `dest` itself. For the hymn library it is
/// the extracted folder, since the zip is deleted after extraction.
fn sync_file_if(url: &str, dest: &Path, stamp: &Path, present: bool) -> Result<SyncOutcome> {
    let agent = agent();
    let head = agent
        .head(url)
        .call()
        .with_context(|| format!("HEAD {url}"))?;
    let etag = head
        .header("etag")
        .or_else(|| head.header("last-modified"))
        .unwrap_or_default()
        .to_string();
    let last = std::fs::read_to_string(stamp).unwrap_or_default();
    if !etag.is_empty() && etag == last.trim() && present {
        return Ok(SyncOutcome::Unchanged);
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Some(parent) = stamp.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let part = dest.with_extension("part");
    let result = (|| -> Result<()> {
        let resp = agent
            .get(url)
            .call()
            .with_context(|| format!("GET {url}"))?;
        let mut file = std::fs::File::create(&part)?;
        std::io::copy(&mut resp.into_reader(), &mut file)?;
        std::fs::rename(&part, dest)?;
        std::fs::write(stamp, &etag)?;
        Ok(())
    })();
    let _ = std::fs::remove_file(&part);
    result.map(|_| SyncOutcome::Downloaded)
}

/// Ensure `dest_dir` holds the hymns published at `url` as a zip. Extraction
/// goes to a sibling temp folder and is swapped in whole so a failed download
/// never leaves a half library. The zip is removed once extracted.
pub fn sync_hymns(url: &str, dest_dir: &Path, stamp: &Path) -> Result<SyncOutcome> {
    let zip_path = dest_dir.with_extension("zip");
    if sync_file_if(url, &zip_path, stamp, dest_dir.is_dir())? == SyncOutcome::Unchanged {
        return Ok(SyncOutcome::Unchanged);
    }

    let tmp_dir = dest_dir.with_extension("tmp");
    let result = (|| -> Result<()> {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        extract_zip(&zip_path, &tmp_dir)?;
        if dest_dir.exists() {
            std::fs::remove_dir_all(dest_dir)?;
        }
        std::fs::rename(&tmp_dir, dest_dir)?;
        Ok(())
    })();
    let _ = std::fs::remove_file(&zip_path);
    let _ = std::fs::remove_dir_all(&tmp_dir);
    if result.is_err() {
        // The stamp already holds the new ETag. Drop it so the next launch
        // downloads again instead of keeping the old library.
        let _ = std::fs::remove_file(stamp);
    }
    result.map(|_| SyncOutcome::Downloaded)
}

/// A few words for a status chip out of a sync error chain. A status code
/// becomes `HTTP 404`, a transport failure becomes `network`, anything else
/// keeps its first 40 chars. Never a URL.
pub fn short_reason(err: &str) -> String {
    if let Some(code) = err
        .split("status code ")
        .nth(1)
        .and_then(|rest| rest.split(|c: char| !c.is_ascii_digit()).next())
        .filter(|c| !c.is_empty())
    {
        return format!("HTTP {code}");
    }
    let network = [
        "Dns Failed",
        "Connection Failed",
        "Network Error",
        "timed out",
        "Too Many Redirects",
    ];
    if network.iter().any(|n| err.contains(n)) {
        return "network".into();
    }
    let first_line = err.lines().next().unwrap_or_default().trim();
    first_line.chars().take(40).collect()
}

/// Unpack `zip` into `dest`. Entry paths are sanitized by the `zip` crate, so
/// an archive cannot write outside `dest`.
pub fn extract_zip(zip: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(zip).with_context(|| format!("open {}", zip.display()))?;
    let mut archive = zip::ZipArchive::new(file).context("read zip")?;
    archive.extract(dest).context("extract zip")?;
    Ok(())
}
