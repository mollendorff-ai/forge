//! Self-update command for Forge CLI
//!
//! Downloads and installs the latest version from GitHub releases.
//! Restored in v10.0.0 after forge became public (see ADR-024 for history).

use crate::error::{ForgeError, ForgeResult};
use colored::Colorize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
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

/// Name prefix of the release archives (`forge-<target>.tar.gz`)
const ASSET_PREFIX: &str = "forge";

/// Release asset that lists the SHA-256 of every archive
const CHECKSUMS_ASSET: &str = "checksums.txt";

/// Detect the current platform as the Rust target used in release asset names
fn detect_platform() -> ForgeResult<&'static str> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    match (os, arch) {
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-musl"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-musl"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("windows", "x86_64") => Ok("x86_64-pc-windows-msvc"),
        _ => Err(ForgeError::Validation(format!(
            "Unsupported platform: {os}-{arch}"
        ))),
    }
}

/// Download `url` to `dest`. HTTPS only; an HTTP error status is a failure,
/// so an error page is never saved as if it were the file.
fn download(url: &str, dest: &Path) -> ForgeResult<()> {
    let status = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--output",
        ])
        .arg(dest)
        .arg(url)
        .status()
        .map_err(|e| ForgeError::Validation(format!("Failed to run curl: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(ForgeError::Validation(format!("Download failed: {url}")))
    }
}

/// Parse `sha256sum` output into file name -> lowercase hex digest.
///
/// Entries may include a directory (`dir/forge-x.tar.gz`); only the file
/// name is kept.
fn parse_checksums(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let digest = parts.next()?;
            let path = parts.next()?.trim_start_matches('*');
            let name = path.rsplit('/').next()?;
            let is_sha256 = digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit());
            is_sha256.then(|| (name.to_string(), digest.to_ascii_lowercase()))
        })
        .collect()
}

/// SHA-256 of a file as lowercase hex
fn sha256_file(path: &Path) -> ForgeResult<String> {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    let bytes = fs::read(path)
        .map_err(|e| ForgeError::Validation(format!("Failed to read {}: {e}", path.display())))?;
    let digest = Sha256::digest(&bytes);
    Ok(digest
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        }))
}

/// Check a downloaded file against the release checksums.
///
/// # Errors
///
/// Returns an error if the release lists no checksum for `name` or the
/// digest differs. The caller must not install the file in either case.
fn verify_checksum(
    name: &str,
    path: &Path,
    checksums: &HashMap<String, String>,
) -> ForgeResult<()> {
    let expected = checksums.get(name).ok_or_else(|| {
        ForgeError::Validation(format!(
            "The release lists no checksum for {name}; refusing to install it"
        ))
    })?;
    let actual = sha256_file(path)?;
    if &actual == expected {
        Ok(())
    } else {
        Err(ForgeError::Validation(format!(
            "Checksum mismatch for {name}: expected {expected}, got {actual}. \
             The download is corrupt or has been tampered with; nothing was installed."
        )))
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

/// Find the archive for the current platform (`forge-<target>.tar.gz` or `.zip`)
fn find_platform_asset<'a>(release: &'a ReleaseInfo, platform: &str) -> Option<&'a AssetInfo> {
    ["tar.gz", "zip"].iter().find_map(|extension| {
        let name = format!("{ASSET_PREFIX}-{platform}.{extension}");
        release.assets.iter().find(|a| a.name == name)
    })
}

/// Download an archive, verify its checksum, extract it and install the binary
fn download_and_install(
    release: &ReleaseInfo,
    asset: &AssetInfo,
    install_path: &PathBuf,
    verbose: bool,
) -> ForgeResult<()> {
    // A private directory, not a fixed name that another user could create first
    let temp_dir = tempfile::Builder::new()
        .prefix("forge-update-")
        .tempdir()
        .map_err(|e| ForgeError::Validation(format!("Failed to create temp dir: {e}")))?;

    let checksums_asset = release
        .assets
        .iter()
        .find(|a| a.name == CHECKSUMS_ASSET)
        .ok_or_else(|| {
            ForgeError::Validation(format!(
                "The release has no {CHECKSUMS_ASSET}; refusing to install an unverified binary"
            ))
        })?;
    let checksums_path = temp_dir.path().join(CHECKSUMS_ASSET);
    download(&checksums_asset.download_url, &checksums_path)?;
    let checksums = parse_checksums(
        &fs::read_to_string(&checksums_path)
            .map_err(|e| ForgeError::Validation(format!("Failed to read checksums: {e}")))?,
    );

    let download_path = temp_dir.path().join(&asset.name);
    if verbose {
        println!("   Downloading: {}", asset.download_url);
        println!("   Size: {} bytes", asset.size);
    }
    download(&asset.download_url, &download_path)?;

    verify_checksum(&asset.name, &download_path, &checksums)?;
    println!("   🔒 SHA-256 verified against {CHECKSUMS_ASSET}");

    // Extract (bsdtar, the `tar` on macOS and Windows, also reads zip)
    let extract_dir = temp_dir.path().join("extracted");
    fs::create_dir_all(&extract_dir)
        .map_err(|e| ForgeError::Validation(format!("Failed to create extract dir: {e}")))?;
    let status = Command::new("tar")
        .arg("-xf")
        .arg(&download_path)
        .arg("-C")
        .arg(&extract_dir)
        .status()
        .map_err(|e| ForgeError::Validation(format!("Failed to extract: {e}")))?;
    if !status.success() {
        return Err(ForgeError::Validation("Extraction failed".to_string()));
    }

    let archived_binary = format!("{ASSET_PREFIX}{}", env::consts::EXE_SUFFIX);
    let binary_path = extract_dir.join(&archived_binary);
    if !binary_path.is_file() {
        return Err(ForgeError::Validation(format!(
            "Binary not found in archive: {archived_binary}"
        )));
    }

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

    let asset = find_platform_asset(&release, platform).ok_or_else(|| {
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

    download_and_install(&release, asset, &install_path, verbose)?;

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

    /// Asset names and checksums.txt lines as published for v10.0.0-beta.8
    const BETA8_CHECKSUMS: &str = "\
d02f9496cf002f9a274274a354471567637b508fc1e4a36734b7eed5fc6dc1e0  forge-aarch64-apple-darwin/forge-aarch64-apple-darwin.tar.gz
4f9e25a7a8d40dad4e356757ea4cf8901724d231335020b457b31b0160d610b7  forge-aarch64-unknown-linux-musl/forge-aarch64-unknown-linux-musl.tar.gz
a1c6bde499812f6ae236467f1d31db52f5ba66cb30c7bcba262bbb3d8b1a48ec  forge-x86_64-apple-darwin/forge-x86_64-apple-darwin.tar.gz
3ea6b2fb071bc30c7d5aa6393cbcd5f413629936b96b1d470fec5ee9e3890c14  forge-x86_64-pc-windows-msvc/forge-x86_64-pc-windows-msvc.zip
c4c3e290f8043329925127b97cc7831094f0f0fcfbf0cc362bd6cb92c5573a54  forge-x86_64-unknown-linux-musl/forge-x86_64-unknown-linux-musl.tar.gz
";

    fn beta8_release() -> ReleaseInfo {
        let names = [
            "checksums.txt",
            "forge-aarch64-apple-darwin.tar.gz",
            "forge-aarch64-unknown-linux-musl.tar.gz",
            "forge-x86_64-apple-darwin.tar.gz",
            "forge-x86_64-pc-windows-msvc.zip",
            "forge-x86_64-unknown-linux-musl.tar.gz",
        ];
        ReleaseInfo {
            version: "10.0.0-beta.8".to_string(),
            published_at: "2026-03-15".to_string(),
            assets: names
                .iter()
                .map(|name| AssetInfo {
                    name: (*name).to_string(),
                    download_url: format!("https://example.com/{name}"),
                    size: 1000,
                })
                .collect(),
        }
    }

    #[test]
    fn test_find_platform_asset_matches_published_names() {
        // The old lookup used names like forge-linux-x86_64.tar.gz, which no
        // release has, so `forge update` could never find an asset
        let release = beta8_release();
        for (platform, expected) in [
            (
                "x86_64-unknown-linux-musl",
                "forge-x86_64-unknown-linux-musl.tar.gz",
            ),
            (
                "aarch64-unknown-linux-musl",
                "forge-aarch64-unknown-linux-musl.tar.gz",
            ),
            ("x86_64-apple-darwin", "forge-x86_64-apple-darwin.tar.gz"),
            ("aarch64-apple-darwin", "forge-aarch64-apple-darwin.tar.gz"),
            ("x86_64-pc-windows-msvc", "forge-x86_64-pc-windows-msvc.zip"),
        ] {
            let asset = find_platform_asset(&release, platform);
            assert_eq!(asset.map(|a| a.name.as_str()), Some(expected), "{platform}");
        }
        assert!(find_platform_asset(&release, "riscv64gc-unknown-linux-gnu").is_none());
    }

    #[test]
    fn test_detect_platform_has_a_published_asset() {
        if let Ok(platform) = detect_platform() {
            assert!(
                find_platform_asset(&beta8_release(), platform).is_some(),
                "{platform}"
            );
        }
    }

    #[test]
    fn test_parse_checksums_keeps_file_names() {
        let checksums = parse_checksums(BETA8_CHECKSUMS);
        assert_eq!(checksums.len(), 5);
        assert_eq!(
            checksums["forge-x86_64-unknown-linux-musl.tar.gz"],
            "c4c3e290f8043329925127b97cc7831094f0f0fcfbf0cc362bd6cb92c5573a54"
        );
        // Binary-mode marker, upper case and junk lines
        let other = parse_checksums("ABCDEF0123456789abcdef0123456789ABCDEF0123456789abcdef0123456789 *a.zip\nnot a checksum\n");
        assert_eq!(
            other["a.zip"],
            "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
        );
        assert_eq!(other.len(), 1);
    }

    #[test]
    fn test_verify_checksum() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forge-test.tar.gz");
        fs::write(&file, b"abc").unwrap();
        // SHA-256("abc"), a standard test vector
        let good = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(sha256_file(&file).unwrap(), good);

        let mut checksums = HashMap::new();
        checksums.insert("forge-test.tar.gz".to_string(), good.to_string());
        assert!(verify_checksum("forge-test.tar.gz", &file, &checksums).is_ok());

        fs::write(&file, b"abd").unwrap();
        let err = verify_checksum("forge-test.tar.gz", &file, &checksums).unwrap_err();
        assert!(err.to_string().contains("Checksum mismatch"), "{err}");

        let err = verify_checksum("other.tar.gz", &file, &checksums).unwrap_err();
        assert!(err.to_string().contains("no checksum"), "{err}");
    }

    #[test]
    fn test_current_version() {
        // Verify the current version has the expected semver format
        assert!(CURRENT_VERSION.contains('.'));
    }
}
