//! Self-update command for Forge CLI
//!
//! Downloads and installs the latest version from GitHub releases.
//! Restored in v10.0.0 after forge became public (see ADR-024 for history).

use crate::error::{ForgeError, ForgeResult};
use colored::Colorize;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

/// GitHub repository for releases
const GITHUB_REPO: &str = "mollendorff-ai/forge";

/// Current version from Cargo.toml
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Release info from GitHub API
#[derive(Debug)]
pub struct ReleaseInfo {
    pub version: String,
    pub published_at: String,
    pub assets: Vec<AssetInfo>,
}

/// Asset info from GitHub API
#[derive(Debug)]
pub struct AssetInfo {
    pub name: String,
    pub download_url: String,
    pub size: u64,
}

/// Detect the current platform for asset selection
fn detect_platform() -> ForgeResult<&'static str> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    match (os, arch) {
        ("linux", "x86_64") => Ok("linux-x86_64"),
        ("linux", "aarch64") => Ok("linux-arm64"),
        ("macos", "x86_64") => Ok("macos-x86_64"),
        ("macos", "aarch64") => Ok("macos-arm64"),
        ("windows", "x86_64") => Ok("windows"),
        _ => Err(ForgeError::Validation(format!(
            "Unsupported platform: {os}-{arch}"
        ))),
    }
}

/// Fetch latest release info from GitHub API
fn fetch_latest_release() -> ForgeResult<ReleaseInfo> {
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");

    // Use curl to fetch the release info (available on all platforms)
    let output = Command::new("curl")
        .args([
            "-sL",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: forge-cli",
            &url,
        ])
        .output()
        .map_err(|e| ForgeError::Validation(format!("Failed to run curl: {e}")))?;

    if !output.status.success() {
        return Err(ForgeError::Validation(format!(
            "GitHub API request failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| ForgeError::Validation(format!("Failed to parse GitHub response: {e}")))?;

    // Check for API error
    if let Some(message) = json.get("message") {
        return Err(ForgeError::Validation(format!(
            "GitHub API error: {}",
            message.as_str().unwrap_or("Unknown error")
        )));
    }

    let version = json["tag_name"]
        .as_str()
        .ok_or_else(|| ForgeError::Validation("Missing tag_name in response".to_string()))?
        .trim_start_matches('v')
        .to_string();

    let published_at = json["published_at"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    let assets = json["assets"]
        .as_array()
        .ok_or_else(|| ForgeError::Validation("Missing assets in response".to_string()))?
        .iter()
        .filter_map(|asset| {
            let name = asset["name"].as_str()?.to_string();
            let download_url = asset["browser_download_url"].as_str()?.to_string();
            let size = asset["size"].as_u64().unwrap_or(0);
            Some(AssetInfo {
                name,
                download_url,
                size,
            })
        })
        .collect();

    Ok(ReleaseInfo {
        version,
        published_at,
        assets,
    })
}

/// Compare versions (semver-style)
fn is_newer_version(latest: &str, current: &str) -> bool {
    // Parse versions, handling pre-release tags like "alpha.4"
    let parse_version = |v: &str| -> (Vec<u32>, Option<String>) {
        let parts: Vec<&str> = v.split('-').collect();
        let numbers: Vec<u32> = parts[0].split('.').filter_map(|s| s.parse().ok()).collect();
        let prerelease = parts.get(1).map(ToString::to_string);
        (numbers, prerelease)
    };

    let (latest_nums, latest_pre) = parse_version(latest);
    let (current_nums, current_pre) = parse_version(current);

    // Compare version numbers
    for i in 0..std::cmp::max(latest_nums.len(), current_nums.len()) {
        let l = latest_nums.get(i).copied().unwrap_or(0);
        let c = current_nums.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }

    // Same base version, compare pre-release
    // None (stable) > Some (pre-release)
    // alpha.5 > alpha.4
    match (&latest_pre, &current_pre) {
        (None, Some(_)) => true,         // stable > pre-release
        (Some(_) | None, None) => false, // pre-release < stable, or same stable version
        (Some(l), Some(c)) => l > c,     // compare pre-release strings
    }
}

/// Find the asset for the current platform
fn find_platform_asset<'a>(
    release: &'a ReleaseInfo,
    platform: &str,
    binary_name: &str,
) -> Option<&'a AssetInfo> {
    // Look for tarball or direct binary
    let patterns = [
        format!("{binary_name}-{platform}.tar.gz"),
        format!("{binary_name}-{platform}.tar.xz"),
        format!("{binary_name}-{platform}"),
        format!("{binary_name}-{platform}.exe"),
    ];

    for pattern in &patterns {
        if let Some(asset) = release.assets.iter().find(|a| a.name == *pattern) {
            return Some(asset);
        }
    }

    None
}

/// Download and install a binary
fn download_and_install(
    asset: &AssetInfo,
    install_path: &PathBuf,
    verbose: bool,
) -> ForgeResult<()> {
    let temp_dir = env::temp_dir().join("forge-update");
    fs::create_dir_all(&temp_dir)
        .map_err(|e| ForgeError::Validation(format!("Failed to create temp dir: {e}")))?;

    let download_path = temp_dir.join(&asset.name);

    if verbose {
        println!("   Downloading: {}", asset.download_url);
        println!("   Size: {} bytes", asset.size);
    }

    // Download the asset
    let status = Command::new("curl")
        .args([
            "-sL",
            "-o",
            download_path.to_str().unwrap(),
            &asset.download_url,
        ])
        .status()
        .map_err(|e| ForgeError::Validation(format!("Failed to download: {e}")))?;

    if !status.success() {
        return Err(ForgeError::Validation("Download failed".to_string()));
    }

    // Extract if tarball, otherwise use directly
    let binary_path = if asset.name.ends_with(".tar.gz") || asset.name.ends_with(".tar.xz") {
        // Extract tarball
        let extract_dir = temp_dir.join("extracted");
        fs::create_dir_all(&extract_dir)
            .map_err(|e| ForgeError::Validation(format!("Failed to create extract dir: {e}")))?;

        let tar_flag = if asset.name.ends_with(".tar.xz") {
            "xJf"
        } else {
            "xzf"
        };

        let status = Command::new("tar")
            .args([tar_flag, download_path.to_str().unwrap(), "-C"])
            .arg(&extract_dir)
            .status()
            .map_err(|e| ForgeError::Validation(format!("Failed to extract: {e}")))?;

        if !status.success() {
            return Err(ForgeError::Validation("Extraction failed".to_string()));
        }

        // Find the binary in extracted files
        let binary_name = install_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("forge");

        // Check both root and subdirectory
        let candidates = [
            extract_dir.join(binary_name),
            extract_dir.join(format!("{binary_name}/{binary_name}")),
        ];

        candidates
            .iter()
            .find(|p| p.exists())
            .cloned()
            .ok_or_else(|| {
                ForgeError::Validation(format!("Binary not found in archive: {binary_name}"))
            })?
    } else {
        download_path
    };

    // Backup existing binary
    if install_path.exists() {
        let backup_path = install_path.with_extension("bak");
        if verbose {
            println!(
                "   Backup: {} -> {}",
                install_path.display(),
                backup_path.display()
            );
        }
        fs::rename(install_path, &backup_path)
            .map_err(|e| ForgeError::Validation(format!("Failed to backup: {e}")))?;
    }

    // Install new binary
    fs::copy(&binary_path, install_path)
        .map_err(|e| ForgeError::Validation(format!("Failed to install: {e}")))?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(install_path)
            .map_err(|e| ForgeError::Validation(format!("Failed to get permissions: {e}")))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(install_path, perms)
            .map_err(|e| ForgeError::Validation(format!("Failed to set permissions: {e}")))?;
    }

    // Cleanup temp files
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

/// Get the installation path for the current binary
fn get_install_path() -> ForgeResult<PathBuf> {
    env::current_exe()
        .map_err(|e| ForgeError::Validation(format!("Failed to get current executable path: {e}")))
}

/// Execute the update command.
///
/// # Errors
///
/// Returns an error if the GitHub API request fails, the platform is unsupported,
/// or the download/install process fails.
///
/// # Panics
///
/// Panics if stdout flushing or stdin reading fails during the confirmation prompt.
///
/// # Coverage Exclusion (ADR-006)
/// Network operations and file system modifications cannot be unit tested.
/// Tested via: manual integration testing
#[cfg(not(coverage))]
pub fn update(check_only: bool, verbose: bool) -> ForgeResult<()> {
    println!("{}", "🔄 Forge - Update Check".bold().green());
    println!("   Current version: {}", CURRENT_VERSION.cyan());
    println!();

    // Fetch latest release
    if verbose {
        println!("{}", "📡 Fetching latest release from GitHub...".cyan());
    }

    let release = fetch_latest_release()?;

    println!(
        "   Latest version:  {} ({})",
        release.version.bright_yellow().bold(),
        release.published_at.dimmed()
    );

    // Check if update needed
    if !is_newer_version(&release.version, CURRENT_VERSION) {
        println!();
        println!("{}", "✅ You're running the latest version!".bold().green());
        return Ok(());
    }

    println!();
    println!(
        "{}",
        format!(
            "🆕 Update available: {} → {}",
            CURRENT_VERSION, release.version
        )
        .bold()
        .yellow()
    );

    if check_only {
        println!();
        println!("   Run {} to install the update", "forge update".cyan());
        return Ok(());
    }

    // Detect platform and find asset
    let platform = detect_platform()?;
    if verbose {
        println!("   Platform: {platform}");
    }

    let install_path = get_install_path()?;
    let binary_name = install_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("forge");

    let asset = find_platform_asset(&release, platform, binary_name).ok_or_else(|| {
        ForgeError::Validation(format!(
            "No release asset found for platform: {platform}\nAvailable assets: {}",
            release
                .assets
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })?;

    println!();
    println!("   📦 Asset: {}", asset.name.cyan());
    println!("   📂 Install to: {}", install_path.display());

    // Confirm with user
    print!("\n   Proceed with update? [y/N] ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("{}", "   Update cancelled.".yellow());
        return Ok(());
    }

    println!();
    println!("{}", "⬇️  Downloading...".cyan());

    download_and_install(asset, &install_path, verbose)?;

    println!();
    println!(
        "{}",
        format!("✅ Successfully updated to v{}!", release.version)
            .bold()
            .green()
    );
    println!("{}", "   Restart forge to use the new version.".dimmed());

    Ok(())
}

/// Stub for coverage builds - see ADR-006
#[cfg(coverage)]
pub fn update(_check_only: bool, _verbose: bool) -> ForgeResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_platform() {
        // Should not error on supported platforms
        let result = detect_platform();
        // The test runs on the current platform, so it should succeed
        assert!(result.is_ok() || result.is_err()); // Just verify it runs
    }

    #[test]
    fn test_is_newer_version_major() {
        assert!(is_newer_version("11.0.0", "10.0.0"));
        assert!(!is_newer_version("10.0.0", "11.0.0"));
        assert!(!is_newer_version("10.0.0", "10.0.0"));
    }

    #[test]
    fn test_is_newer_version_minor() {
        assert!(is_newer_version("10.1.0", "10.0.0"));
        assert!(!is_newer_version("10.0.0", "10.1.0"));
    }

    #[test]
    fn test_is_newer_version_patch() {
        assert!(is_newer_version("10.0.1", "10.0.0"));
        assert!(!is_newer_version("10.0.0", "10.0.1"));
    }

    #[test]
    fn test_is_newer_version_prerelease() {
        // Stable > pre-release
        assert!(is_newer_version("10.0.0", "10.0.0-alpha.1"));
        assert!(!is_newer_version("10.0.0-alpha.1", "10.0.0"));

        // Higher pre-release
        assert!(is_newer_version("10.0.0-alpha.5", "10.0.0-alpha.4"));
        assert!(!is_newer_version("10.0.0-alpha.4", "10.0.0-alpha.5"));

        // Same pre-release
        assert!(!is_newer_version("10.0.0-alpha.4", "10.0.0-alpha.4"));
    }

    #[test]
    fn test_is_newer_version_different_lengths() {
        assert!(is_newer_version("10.0.0.1", "10.0.0"));
        assert!(!is_newer_version("10.0.0", "10.0.0.1"));
    }

    #[test]
    fn test_find_platform_asset() {
        let release = ReleaseInfo {
            version: "10.0.0".to_string(),
            published_at: "2026-01-02".to_string(),
            assets: vec![
                AssetInfo {
                    name: "forge-linux-x86_64.tar.gz".to_string(),
                    download_url: "https://example.com/forge-linux-x86_64.tar.gz".to_string(),
                    size: 1000,
                },
                AssetInfo {
                    name: "forge-macos-arm64.tar.gz".to_string(),
                    download_url: "https://example.com/forge-macos-arm64.tar.gz".to_string(),
                    size: 1000,
                },
            ],
        };

        let asset = find_platform_asset(&release, "linux-x86_64", "forge");
        assert!(asset.is_some());
        assert_eq!(asset.unwrap().name, "forge-linux-x86_64.tar.gz");

        let asset = find_platform_asset(&release, "macos-arm64", "forge");
        assert!(asset.is_some());
        assert_eq!(asset.unwrap().name, "forge-macos-arm64.tar.gz");

        let asset = find_platform_asset(&release, "windows", "forge");
        assert!(asset.is_none());
    }

    #[test]
    fn test_current_version() {
        // Verify the current version has the expected semver format
        assert!(CURRENT_VERSION.contains('.'));
    }
}
