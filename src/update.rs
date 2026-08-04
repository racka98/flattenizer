use serde::Deserialize;
use std::io::{Read, Write};
use std::path::PathBuf;

// >>> SET THESE to match your GitHub repo before building a release <<<
pub const GITHUB_OWNER: &str = "racka98";
pub const GITHUB_REPO: &str = "flattenizer";

/// Name of the installer asset as published in GitHub Releases.
/// Must match the filename produced by the Inno Setup build
/// (see installer/flattenizer.iss -> OutputBaseFilename).
const INSTALLER_ASSET_NAME: &str = "FlattenizerSetup.exe";

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
    html_url: String,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub download_url: String,
    pub release_url: String,
}

/// Strips a leading 'v' from a git tag, e.g. "v1.2.3" -> "1.2.3".
fn normalize_version(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

/// Very small semver-ish comparison: splits on '.', compares numeric parts
/// left to right. Good enough for straightforward "major.minor.patch" tags;
/// doesn't handle pre-release suffixes like "-beta".
fn is_newer(current: &str, candidate: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let cur = parse(current);
    let cand = parse(candidate);
    for i in 0..cur.len().max(cand.len()) {
        let c = cur.get(i).copied().unwrap_or(0);
        let n = cand.get(i).copied().unwrap_or(0);
        if n > c {
            return true;
        }
        if n < c {
            return false;
        }
    }
    false
}

/// Checks GitHub Releases for a newer version than the currently running
/// build. Returns `Ok(None)` if already up to date. Network/parse errors
/// are returned as `Err` so the caller can decide whether to surface them
/// (this should never block normal app usage).
pub fn check_for_update() -> Result<Option<UpdateInfo>, String> {
    let url = format!(
        "https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases/latest"
    );

    let response = ureq::get(&url)
        .set("User-Agent", "flattenizer-update-check")
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("Failed to reach GitHub: {e}"))?;

    let release: GithubRelease = response
        .into_json()
        .map_err(|e| format!("Failed to parse GitHub response: {e}"))?;

    let latest_version = normalize_version(&release.tag_name).to_string();
    let current_version = env!("CARGO_PKG_VERSION");

    if !is_newer(current_version, &latest_version) {
        return Ok(None);
    }

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == INSTALLER_ASSET_NAME)
        .ok_or_else(|| {
            format!("Release {latest_version} has no asset named {INSTALLER_ASSET_NAME}")
        })?;

    Ok(Some(UpdateInfo {
        latest_version,
        download_url: asset.browser_download_url.clone(),
        release_url: release.html_url,
    }))
}

/// Downloads the installer to a temp file and returns its path.
pub fn download_installer(info: &UpdateInfo) -> Result<PathBuf, String> {
    let response = ureq::get(&info.download_url)
        .set("User-Agent", "flattenizer-update-check")
        .call()
        .map_err(|e| format!("Failed to download installer: {e}"))?;

    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read installer download: {e}"))?;

    let mut path = std::env::temp_dir();
    path.push(INSTALLER_ASSET_NAME);

    let mut file =
        std::fs::File::create(&path).map_err(|e| format!("Failed to save installer: {e}"))?;
    file.write_all(&bytes)
        .map_err(|e| format!("Failed to write installer: {e}"))?;

    Ok(path)
}

/// Launches the downloaded installer. The installer replaces the currently
/// running exe, so the caller should exit shortly after calling this.
/// `/SILENT` shows a minimal progress UI but no prompts; the installer is
/// built with `PrivilegesRequired=lowest`, so no UAC elevation dialog is
/// expected for a per-user install.
pub fn launch_installer(path: &PathBuf) -> Result<(), String> {
    std::process::Command::new(path)
        .arg("/SILENT")
        .arg("/CLOSEAPPLICATIONS")
        .spawn()
        .map_err(|e| format!("Failed to launch installer: {e}"))?;
    Ok(())
}
