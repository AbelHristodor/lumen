//! Small JSON files for app state. Loading never fails. A missing or broken
//! file yields the default value. Saving writes a temp file first and renames
//! it over the target so a crash never leaves a half written file.

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::Path;

/// Read and parse `path`. A missing file is silent. Any other error logs a
/// warning. Both return `T::default()`.
pub fn json_load<T: DeserializeOwned + Default>(path: &Path) -> T {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::warn!("read {}: {e}", path.display());
            }
            return T::default();
        }
    };
    match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("parse {}: {e}", path.display());
            T::default()
        }
    }
}

/// Write `value` as pretty JSON to `path`. Creates the parent folder.
pub fn json_save<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(value)?;
    std::fs::write(&tmp, text).with_context(|| format!("write {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("rename to {}", path.display()))?;
    Ok(())
}
