use sda_core::update::{is_transient_failure, portable_asset};

#[test]
fn rate_limit_403_is_transient() {
    // The exact shape self_update/ureq produces on GitHub's unauthenticated
    // rate limit, plus common phrasings.
    assert!(is_transient_failure("UreqError: http status: 403"));
    assert!(is_transient_failure("http status: 429"));
    assert!(is_transient_failure("API rate limit exceeded for 1.2.3.4"));
}

#[test]
fn network_errors_are_transient() {
    assert!(is_transient_failure("dns error: failed to lookup address"));
    assert!(is_transient_failure("Network is unreachable (os error 51)"));
    assert!(is_transient_failure("connection timed out"));
}

#[test]
fn real_failures_are_not_transient() {
    // A genuine problem we'd want surfaced, not silently softened.
    assert!(!is_transient_failure(
        "Could not find the required path in the archive: lumen"
    ));
    assert!(!is_transient_failure("http status: 404"));
    assert!(!is_transient_failure("permission denied (os error 13)"));
}

#[test]
fn portable_asset_picks_the_flat_archive_and_skips_installers() {
    // Must match the release asset names in .github/workflows/release.yml.
    let assets = [
        "lumen-2.0.1-linux-x86_64.AppImage",
        "lumen-2.0.1-linux-x86_64.deb",
        "lumen-2.0.1-linux-x86_64.rpm",
        "lumen-2.0.1-linux-x86_64.tar.gz",
        "lumen-2.0.1-linux-aarch64.tar.gz",
        "lumen-2.0.1-windows-x64.msi",
        "lumen-2.0.1-windows-x64-portable.zip",
        "lumen-2.0.1-macos-universal.dmg",
    ];
    let pick = |os, arch| {
        let (target, marker) = portable_asset(os, arch, false)?;
        assets
            .iter()
            .find(|a| a.contains(target) && a.contains(marker))
            .copied()
    };
    assert_eq!(
        pick("linux", "x86_64"),
        Some("lumen-2.0.1-linux-x86_64.tar.gz")
    );
    assert_eq!(
        pick("linux", "aarch64"),
        Some("lumen-2.0.1-linux-aarch64.tar.gz")
    );
    assert_eq!(
        pick("windows", "x86_64"),
        Some("lumen-2.0.1-windows-x64-portable.zip")
    );
    assert_eq!(pick("macos", "aarch64"), None);
    assert_eq!(portable_asset("linux", "x86_64", true), None);
}
