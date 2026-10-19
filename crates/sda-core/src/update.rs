//! Self update from GitHub Releases. Compares the running version
//! (`CARGO_PKG_VERSION`) against the latest release tag. A portable install
//! on Linux or Windows downloads its flat archive and replaces the executable
//! in place. The running process keeps old code until the next restart.
//! Everything else, the macOS app, an AppImage, a deb or rpm, or a folder we
//! cannot write, only reports that a new version is available.

use anyhow::Result;
use std::path::Path;

const REPO_OWNER: &str = "AbelHristodor";
const REPO_NAME: &str = "lumen";
const BIN_NAME: &str = "lumen";

/// Web page of the newest release, for the update chip.
pub const RELEASES_URL: &str = "https://github.com/AbelHristodor/lumen/releases/latest";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
    UpToDate,
    Updated {
        version: String,
    },
    /// Newer release exists but this install cannot replace itself.
    Available {
        version: String,
    },
    /// The check could not be performed for a transient reason (offline,
    /// GitHub API rate limit / 403, timeout). Not a real error — try later.
    CheckUnavailable,
}

/// Classify an update-check error message as transient (network/rate-limit)
/// vs. a genuine failure. Transient ones are expected and should be reported
/// calmly ("try later"), not as alarming failures. Matched on the message
/// text because `self_update` erases the underlying error type.
pub fn is_transient_failure(msg: &str) -> bool {
    let m = msg.to_lowercase();
    // GitHub rate limit (403) / too-many-requests (429), and its prose form.
    m.contains("403")
        || m.contains("429")
        || m.contains("rate limit")
        // Common offline/network phrasings from ureq/std.
        || m.contains("dns")
        || m.contains("timed out")
        || m.contains("timeout")
        || m.contains("unreachable")
        || m.contains("connection")
}

/// Release asset markers for an in place update on this platform:
/// a substring every asset for it has, plus one that picks the flat archive
/// over the installers. `None` where we only notify.
pub fn portable_asset(
    os: &str,
    arch: &str,
    appimage: bool,
) -> Option<(&'static str, &'static str)> {
    match (os, arch) {
        _ if appimage => None,
        ("linux", "x86_64") => Some(("linux-x86_64", ".tar.gz")),
        ("linux", "aarch64") => Some(("linux-aarch64", ".tar.gz")),
        ("windows", "x86_64") => Some(("windows-x64", "-portable.zip")),
        _ => None,
    }
}

/// A deb or rpm puts us in /usr/bin, which a normal user cannot write.
fn exe_dir_writable() -> bool {
    let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
    else {
        return false;
    };
    let probe = dir.join(".lumen-update-probe");
    let ok = std::fs::File::create(&probe).is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

/// Check releases and, if a newer version exists, stage it or report it.
/// Errors are returned. Callers on boot should log and ignore them.
pub fn check_and_stage_update() -> Result<UpdateOutcome> {
    let current = env!("CARGO_PKG_VERSION");
    let portable = portable_asset(
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::var_os("APPIMAGE").is_some(),
    );
    let result: Result<UpdateOutcome> = (|| {
        let mut builder = self_update::backends::github::Update::configure();
        builder
            .repo_owner(REPO_OWNER)
            .repo_name(REPO_NAME)
            .bin_name(BIN_NAME)
            .current_version(current)
            .no_confirm(true)
            .show_download_progress(false)
            .show_output(false);
        if let Some((target, marker)) = portable {
            builder.target(target).identifier(marker);
        }
        let updater = builder.build()?;
        let latest = updater.get_latest_release()?;
        if !self_update::version::bump_is_greater(current, &latest.version)? {
            return Ok(UpdateOutcome::UpToDate);
        }
        // `update` skips major bumps, so those fall through to Available.
        if portable.is_some() && exe_dir_writable() {
            let status = updater.update()?;
            if status.updated() {
                return Ok(UpdateOutcome::Updated {
                    version: status.version().to_string(),
                });
            }
        }
        Ok(UpdateOutcome::Available {
            version: latest.version,
        })
    })();

    match result {
        Ok(outcome) => Ok(outcome),
        // Transient (offline / rate-limit / 403) → soft outcome, not an error.
        Err(e) if is_transient_failure(&e.to_string()) => Ok(UpdateOutcome::CheckUnavailable),
        Err(e) => Err(e),
    }
}
