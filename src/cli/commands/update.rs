//! Self-update command for Forge CLI
//!
//! Downloads and installs the latest version from GitHub releases.
//! Restored in v10.0.0 after forge became public (see ADR-024 for history).

use crate::cli::console::{out, outln};
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use colored::Colorize;
use std::collections::HashMap;
use std::env;
use std::fs;
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

/// A program to run: its name, then the arguments that come before the ones
/// the command adds. The system's `curl` and `tar` in a real run; a test names
/// a shell script that stands in for them, so no test touches the network.
type Program<'a> = &'a [&'a str];

/// The programs the update runs
#[derive(Clone, Copy)]
struct Tools<'a> {
    curl: Program<'a>,
    tar: Program<'a>,
}

/// Where the update runs: the platform, the binary it replaces and the
/// directory its downloads go through
struct Target {
    platform: ForgeResult<&'static str>,
    install_path: PathBuf,
    work_dir: PathBuf,
}

/// What a real run uses
const SYSTEM_TOOLS: Tools<'static> = Tools {
    curl: &["curl"],
    tar: &["tar"],
};

/// A command for `program`, with its leading arguments already added
fn command(program: Program<'_>) -> Command {
    let mut command = Command::new(program[0]);
    command.args(&program[1..]);
    command
}

/// Detect the current platform as the Rust target used in release asset names
fn detect_platform() -> ForgeResult<&'static str> {
    platform_for(env::consts::OS, env::consts::ARCH)
}

/// The release target for an operating system and a processor architecture
fn platform_for(os: &str, arch: &str) -> ForgeResult<&'static str> {
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
fn download(tools: &Tools<'_>, url: &str, dest: &Path) -> ForgeResult<()> {
    let status = command(tools.curl)
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
            let name = path.rsplit('/').next().unwrap_or(path);
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
fn fetch_latest_release(tools: &Tools<'_>) -> ForgeResult<ReleaseInfo> {
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");

    // Use curl to fetch the release info (available on all platforms)
    let output = command(tools.curl)
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

    parse_release(&output.stdout)
}

/// Read the release the GitHub API answered with
fn parse_release(body: &[u8]) -> ForgeResult<ReleaseInfo> {
    let json: serde_json::Value = serde_json::from_slice(body)
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
    c: &mut Console<'_>,
    tools: &Tools<'_>,
    work_dir: &Path,
    release: &ReleaseInfo,
    asset: &AssetInfo,
    install_path: &Path,
    verbose: bool,
) -> ForgeResult<()> {
    // A private directory, not a fixed name that another user could create first
    let temp_dir = tempfile::Builder::new()
        .prefix("forge-update-")
        .tempdir_in(work_dir)
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
    download(tools, &checksums_asset.download_url, &checksums_path)?;
    let checksums = parse_checksums(
        &fs::read_to_string(&checksums_path)
            .map_err(|e| ForgeError::Validation(format!("Failed to read checksums: {e}")))?,
    );

    let download_path = temp_dir.path().join(&asset.name);
    if verbose {
        outln!(c, "   Downloading: {}", asset.download_url);
        outln!(c, "   Size: {} bytes", asset.size);
    }
    download(tools, &asset.download_url, &download_path)?;

    verify_checksum(&asset.name, &download_path, &checksums)?;
    outln!(c, "   🔒 SHA-256 verified against {CHECKSUMS_ASSET}");

    // Extract (bsdtar, the `tar` on macOS and Windows, also reads zip)
    let extract_dir = temp_dir.path();
    let status = command(tools.tar)
        .arg("-xf")
        .arg(&download_path)
        .arg("-C")
        .arg(extract_dir)
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
            outln!(
                c,
                "   Backup: {} -> {}",
                install_path.display(),
                backup_path.display()
            );
        }
        fs::rename(install_path, &backup_path)
            .map_err(|e| ForgeError::Validation(format!("Failed to backup: {e}")))?;
    }

    // Install new binary
    fs::read(&binary_path)
        .and_then(|bytes| write_executable(install_path, &bytes))
        .map_err(|e| ForgeError::Validation(format!("Failed to install: {e}")))
}

/// Write `bytes` to a new file at `path` that anyone may run (mode 755 on Unix)
fn write_executable(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;

    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o755);
    }
    options
        .open(path)
        .and_then(|mut file| file.write_all(bytes))
}

/// Get the installation path for the current binary
///
/// Empty when the system cannot say; the update refuses that.
fn get_install_path() -> PathBuf {
    env::current_exe().unwrap_or_default()
}

/// Execute the update command.
///
/// # Errors
///
/// Returns an error if the GitHub API request fails, the platform is unsupported,
/// the download/install process fails, or the answer to the confirmation
/// prompt cannot be read.
pub fn update(c: &mut Console<'_>, check_only: bool, verbose: bool) -> ForgeResult<()> {
    update_with(
        c,
        &SYSTEM_TOOLS,
        &Target {
            platform: detect_platform(),
            install_path: get_install_path(),
            work_dir: env::temp_dir(),
        },
        check_only,
        verbose,
    )
}

/// The update, with the programs it runs given
fn update_with(
    c: &mut Console<'_>,
    tools: &Tools<'_>,
    target: &Target,
    check_only: bool,
    verbose: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "🔄 Forge - Update Check".bold().green());
    outln!(c, "   Current version: {}", CURRENT_VERSION.cyan());
    outln!(c);

    // Fetch latest release
    if verbose {
        outln!(c, "{}", "📡 Fetching latest release from GitHub...".cyan());
    }

    let release = fetch_latest_release(tools)?;

    outln!(
        c,
        "   Latest version:  {} ({})",
        release.version.bright_yellow().bold(),
        release.published_at.dimmed()
    );

    // Check if update needed
    if !is_newer_version(&release.version, CURRENT_VERSION) {
        outln!(c);
        outln!(
            c,
            "{}",
            "✅ You're running the latest version!".bold().green()
        );
        return Ok(());
    }

    outln!(c);
    outln!(
        c,
        "{}",
        format!(
            "🆕 Update available: {} → {}",
            CURRENT_VERSION, release.version
        )
        .bold()
        .yellow()
    );

    if check_only {
        outln!(c);
        outln!(c, "   Run {} to install the update", "forge update".cyan());
        return Ok(());
    }

    // Detect platform and find asset
    let platform = target
        .platform
        .as_ref()
        .map_err(|e| ForgeError::Validation(e.to_string()))?;
    let install_path = target.install_path.as_path();
    if install_path.as_os_str().is_empty() {
        return Err(ForgeError::Validation(
            "Cannot tell which binary is running, so cannot replace it".to_string(),
        ));
    }
    if verbose {
        outln!(c, "   Platform: {platform}");
    }

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

    outln!(c);
    outln!(c, "   📦 Asset: {}", asset.name.cyan());
    outln!(c, "   📂 Install to: {}", install_path.display());

    // Confirm with user
    out!(c, "\n   Proceed with update? [y/N] ");
    let mut input = String::new();
    c.read_line(&mut input)?;

    if !input.trim().eq_ignore_ascii_case("y") {
        outln!(c, "{}", "   Update cancelled.".yellow());
        return Ok(());
    }

    outln!(c);
    outln!(c, "{}", "⬇️  Downloading...".cyan());

    download_and_install(
        c,
        tools,
        &target.work_dir,
        &release,
        asset,
        install_path,
        verbose,
    )?;

    outln!(c);
    outln!(
        c,
        "{}",
        format!("✅ Successfully updated to v{}!", release.version)
            .bold()
            .green()
    );
    outln!(c, "{}", "   Restart forge to use the new version.".dimmed());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_platform() {
        assert_eq!(
            platform_for("linux", "x86_64").unwrap(),
            "x86_64-unknown-linux-musl"
        );
        assert_eq!(
            platform_for("macos", "aarch64").unwrap(),
            "aarch64-apple-darwin"
        );
        for (os, arch, target) in [
            ("linux", "aarch64", "aarch64-unknown-linux-musl"),
            ("macos", "x86_64", "x86_64-apple-darwin"),
            ("windows", "x86_64", "x86_64-pc-windows-msvc"),
        ] {
            assert_eq!(platform_for(os, arch).unwrap(), target);
        }
        let err = platform_for("plan9", "mips").unwrap_err();
        assert!(err.to_string().contains("Unsupported platform: plan9-mips"));
        // The platforms the tests run on are ones a release is built for
        assert!(detect_platform().is_ok());
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
        let platform = detect_platform().unwrap();
        assert!(
            find_platform_asset(&beta8_release(), platform).is_some(),
            "{platform}"
        );
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
        // From the release after rc.24: the published names, no folder
        let bare = parse_checksums(
            "c4c3e290f8043329925127b97cc7831094f0f0fcfbf0cc362bd6cb92c5573a54  forge-x86_64-unknown-linux-musl.tar.gz\n",
        );
        assert_eq!(
            bare["forge-x86_64-unknown-linux-musl.tar.gz"],
            "c4c3e290f8043329925127b97cc7831094f0f0fcfbf0cc362bd6cb92c5573a54"
        );
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

    use std::io::{BufRead, Cursor};
    use std::process::Command as StdCommand;

    /// A release the fake `curl` serves, with the archive and checksums it
    /// downloads, all in one temporary directory
    struct Fixture {
        dir: tempfile::TempDir,
        /// The `curl` stand-in: `sh <dir>/curl.sh`
        script: String,
    }

    impl Fixture {
        /// `tag` is the release's tag; `archive_member` the file name inside
        /// the archive; `checksum` the digest the checksums file lists (the
        /// archive's own when `None`); `assets` the asset names listed
        fn new(
            tag: &str,
            archive_member: &str,
            checksum: Option<&str>,
            assets: &[&str],
            curl_body: &str,
        ) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path();
            let platform = detect_platform().unwrap();
            let archive_name = format!("forge-{platform}.tar.gz");

            let content = root.join("content");
            fs::create_dir(&content).unwrap();
            fs::write(content.join(archive_member), "NEW").unwrap();
            let archive = root.join("archive.tar.gz");
            let tarred = StdCommand::new("tar")
                .arg("-czf")
                .arg(&archive)
                .arg("-C")
                .arg(&content)
                .arg(archive_member)
                .status()
                .unwrap();
            assert!(tarred.success());

            let digest = checksum.map_or_else(|| sha256_file(&archive).unwrap(), str::to_string);
            fs::write(
                root.join("checksums.txt"),
                format!("{digest}  {archive_name}\n"),
            )
            .unwrap();

            let listed: Vec<String> = assets
                .iter()
                .map(|name| {
                    let name = name.replace("PLATFORM", platform);
                    format!(
                        r#"{{"name":"{name}","browser_download_url":"https://example.com/{name}","size":3}}"#
                    )
                })
                .collect();
            fs::write(
                root.join("release.json"),
                format!(
                    r#"{{"tag_name":"{tag}","published_at":"2026-01-01","assets":[{}]}}"#,
                    listed.join(",")
                ),
            )
            .unwrap();

            let dir_path = root.display();
            fs::write(
                root.join("curl.sh"),
                format!(
                    r#"if [ "$1" = "-sL" ]; then {curl_body}; fi
while [ $# -gt 1 ]; do
  if [ "$1" = "--output" ]; then dest="$2"; fi
  shift
done
case "$1" in
  */checksums.txt) cp "{dir_path}/checksums.txt" "$dest" ;;
  */forge-*) cp "{dir_path}/archive.tar.gz" "$dest" ;;
  *) exit 22 ;;
esac
"#
                ),
            )
            .unwrap();
            let script = format!("{dir_path}/curl.sh");
            Self { dir, script }
        }

        /// A release at `tag`, with the checksums and the archive for this platform
        fn at(tag: &str) -> Self {
            Self::new(
                tag,
                &format!("forge{}", env::consts::EXE_SUFFIX),
                None,
                &["checksums.txt", "forge-PLATFORM.tar.gz"],
                &format!("cat {}/release.json; exit 0", "$(dirname \"$0\")"),
            )
        }

        fn install_path(&self) -> PathBuf {
            self.dir.path().join("installed-forge")
        }
    }

    /// Run the update against `fixture`, answering the prompt with `answer`
    fn run_update(
        fixture: &Fixture,
        tar: Program<'_>,
        install_path: &Path,
        check_only: bool,
        verbose: bool,
        answer: &str,
    ) -> (ForgeResult<()>, String) {
        let target = Target {
            platform: detect_platform(),
            install_path: install_path.to_path_buf(),
            work_dir: env::temp_dir(),
        };
        let mut input = Cursor::new(answer.as_bytes().to_vec());
        run_target(fixture, tar, &target, check_only, verbose, &mut input)
    }

    /// Run the update against `fixture` with `target` and `input`
    fn run_target(
        fixture: &Fixture,
        tar: Program<'_>,
        target: &Target,
        check_only: bool,
        verbose: bool,
        input: &mut dyn BufRead,
    ) -> (ForgeResult<()>, String) {
        let script = ["sh", fixture.script.as_str()];
        let tools = Tools { curl: &script, tar };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let result = update_with(
            &mut Console::new(&mut out, &mut err, input),
            &tools,
            target,
            check_only,
            verbose,
        );
        (result, String::from_utf8(out).unwrap())
    }

    /// An input whose every read fails
    struct BrokenInput;

    impl std::io::Read for BrokenInput {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("no terminal"))
        }
    }

    #[test]
    fn update_installs_a_newer_release_and_keeps_the_old_binary() {
        let fixture = Fixture::at("v99.0.0");
        let target = fixture.install_path();
        fs::write(&target, "OLD").unwrap();
        let (result, out) = run_update(&fixture, &["tar"], &target, false, true, "y\n");
        result.unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "NEW");
        assert_eq!(
            fs::read_to_string(target.with_extension("bak")).unwrap(),
            "OLD"
        );
        for line in [
            "Fetching latest release",
            "Latest version:",
            "Update available",
            "Platform:",
            "SHA-256 verified against checksums.txt",
            "Backup:",
            "Size: 3 bytes",
            "Successfully updated to v99.0.0",
        ] {
            assert!(out.contains(line), "{line} in {out}");
        }
    }

    #[test]
    fn update_installs_where_there_was_no_binary_without_a_backup() {
        let fixture = Fixture::at("v99.0.0");
        let target = fixture.install_path();
        let (result, out) = run_update(&fixture, &["tar"], &target, false, false, "Y\n");
        result.unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "NEW");
        assert!(!target.with_extension("bak").exists());
        assert!(!out.contains("Backup:"), "{out}");
    }

    #[test]
    fn update_leaves_the_binary_alone_when_the_answer_is_no() {
        let fixture = Fixture::at("v99.0.0");
        let target = fixture.install_path();
        fs::write(&target, "OLD").unwrap();
        let (result, out) = run_update(&fixture, &["tar"], &target, false, false, "n\n");
        result.unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "OLD");
        assert!(out.contains("Update cancelled."), "{out}");
    }

    #[test]
    fn update_check_only_says_so_and_installs_nothing() {
        let fixture = Fixture::at("v99.0.0");
        let target = fixture.install_path();
        let (result, out) = run_update(&fixture, &["tar"], &target, true, false, "");
        result.unwrap();
        assert!(out.contains("forge update"), "{out}");
        assert!(!target.exists());
    }

    #[test]
    fn update_says_when_the_release_is_not_newer() {
        let fixture = Fixture::at("v0.0.1");
        let target = fixture.install_path();
        let (result, out) = run_update(&fixture, &["tar"], &target, false, false, "y\n");
        result.unwrap();
        assert!(out.contains("running the latest version"), "{out}");
        assert!(!target.exists());
    }

    #[test]
    fn update_refuses_a_release_without_an_archive_for_the_platform() {
        let fixture = Fixture::new(
            "v99.0.0",
            "forge",
            None,
            &["checksums.txt", "forge-other.tar.gz"],
            "cat \"$(dirname \"$0\")/release.json\"; exit 0",
        );
        let (result, _) = run_update(
            &fixture,
            &["tar"],
            &fixture.install_path(),
            false,
            false,
            "y\n",
        );
        let message = result.unwrap_err().to_string();
        assert!(message.contains("No release asset found"), "{message}");
        assert!(message.contains("forge-other.tar.gz"), "{message}");
    }

    #[test]
    fn update_refuses_a_release_without_checksums() {
        let fixture = Fixture::new(
            "v99.0.0",
            "forge",
            None,
            &["forge-PLATFORM.tar.gz"],
            "cat \"$(dirname \"$0\")/release.json\"; exit 0",
        );
        let target = fixture.install_path();
        let (result, _) = run_update(&fixture, &["tar"], &target, false, false, "y\n");
        let message = result.unwrap_err().to_string();
        assert!(message.contains("unverified binary"), "{message}");
        assert!(!target.exists());
    }

    #[test]
    fn update_refuses_an_archive_that_does_not_match_its_checksum() {
        let wrong = "0".repeat(64);
        let fixture = Fixture::new(
            "v99.0.0",
            "forge",
            Some(&wrong),
            &["checksums.txt", "forge-PLATFORM.tar.gz"],
            "cat \"$(dirname \"$0\")/release.json\"; exit 0",
        );
        let target = fixture.install_path();
        fs::write(&target, "OLD").unwrap();
        let (result, _) = run_update(&fixture, &["tar"], &target, false, false, "y\n");
        let message = result.unwrap_err().to_string();
        assert!(message.contains("Checksum mismatch"), "{message}");
        assert_eq!(fs::read_to_string(&target).unwrap(), "OLD");
    }

    #[test]
    fn update_reports_an_extraction_that_fails_or_cannot_run() {
        let fixture = Fixture::at("v99.0.0");
        let target = fixture.install_path();
        let (result, _) = run_update(&fixture, &["false"], &target, false, false, "y\n");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Extraction failed"));
        let (result, _) = run_update(&fixture, &["/no/such/tar"], &target, false, false, "y\n");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to extract"));
        assert!(!target.exists());
    }

    #[test]
    fn update_reports_an_archive_without_the_binary() {
        let fixture = Fixture::new(
            "v99.0.0",
            "something-else",
            None,
            &["checksums.txt", "forge-PLATFORM.tar.gz"],
            "cat \"$(dirname \"$0\")/release.json\"; exit 0",
        );
        let (result, _) = run_update(
            &fixture,
            &["tar"],
            &fixture.install_path(),
            false,
            false,
            "y\n",
        );
        let message = result.unwrap_err().to_string();
        assert!(message.contains("Binary not found in archive"), "{message}");
    }

    #[test]
    fn update_reports_a_backup_or_an_install_that_fails() {
        let fixture = Fixture::at("v99.0.0");
        let target = fixture.install_path();
        fs::write(&target, "OLD").unwrap();
        let bak = target.with_extension("bak");
        fs::create_dir(&bak).unwrap();
        fs::write(bak.join("keep"), "x").unwrap();
        let (result, _) = run_update(&fixture, &["tar"], &target, false, false, "y\n");
        assert!(result.unwrap_err().to_string().contains("Failed to backup"));
        assert_eq!(fs::read_to_string(&target).unwrap(), "OLD");

        let missing_dir = fixture.dir.path().join("no-such-dir").join("forge");
        let (result, _) = run_update(&fixture, &["tar"], &missing_dir, false, false, "y\n");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to install"));
    }

    #[test]
    fn update_reports_a_download_that_fails_or_leaves_no_file() {
        let fixture = Fixture::at("v99.0.0");
        let target = fixture.install_path();
        // The download is refused (curl exits 22 for an HTTP error)
        fs::write(
            fixture.dir.path().join("curl.sh"),
            "if [ \"$1\" = \"-sL\" ]; then cat \"$(dirname \"$0\")/release.json\"; exit 0; fi\nexit 22\n",
        )
        .unwrap();
        let (result, _) = run_update(&fixture, &["tar"], &target, false, false, "y\n");
        assert!(result.unwrap_err().to_string().contains("Download failed"));

        // curl says it worked but wrote nothing
        fs::write(
            fixture.dir.path().join("curl.sh"),
            "if [ \"$1\" = \"-sL\" ]; then cat \"$(dirname \"$0\")/release.json\"; exit 0; fi\nexit 0\n",
        )
        .unwrap();
        let (result, _) = run_update(&fixture, &["tar"], &target, false, false, "y\n");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to read checksums"));
    }

    #[test]
    fn the_programs_the_update_runs_may_be_missing() {
        let missing = Tools {
            curl: &["/no/such/curl"],
            tar: &["tar"],
        };
        let message = fetch_latest_release(&missing).unwrap_err().to_string();
        assert!(message.contains("Failed to run curl"), "{message}");
        let dir = tempfile::tempdir().unwrap();
        let message = download(&missing, "https://example.com/x", &dir.path().join("x"))
            .unwrap_err()
            .to_string();
        assert!(message.contains("Failed to run curl"), "{message}");
    }

    #[test]
    fn the_release_request_may_fail() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("curl.sh");
        fs::write(&script, "echo boom >&2; exit 7\n").unwrap();
        let script = script.display().to_string();
        let tools = Tools {
            curl: &["sh", &script],
            tar: &["tar"],
        };
        let message = fetch_latest_release(&tools).unwrap_err().to_string();
        assert!(
            message.contains("GitHub API request failed: boom"),
            "{message}"
        );
    }

    #[test]
    fn the_release_is_read_from_the_api_answer() {
        let release = parse_release(
            br#"{"tag_name":"v1.2.3","published_at":"2026-02-03","assets":[
                {"name":"a.tar.gz","browser_download_url":"https://x/a","size":9},
                {"name":"no-url"},
                {"browser_download_url":"https://x/nameless"},
                {"name":"b","browser_download_url":"https://x/b"}]}"#,
        )
        .unwrap();
        assert_eq!(release.version, "1.2.3");
        assert_eq!(release.published_at, "2026-02-03");
        let assets: Vec<(&str, u64)> = release
            .assets
            .iter()
            .map(|a| (a.name.as_str(), a.size))
            .collect();
        assert_eq!(assets, [("a.tar.gz", 9), ("b", 0)]);

        let release = parse_release(br#"{"tag_name":"2.0.0","assets":[]}"#).unwrap();
        assert_eq!(release.published_at, "unknown");
    }

    #[test]
    fn a_release_answer_that_is_wrong_is_refused() {
        let message = |body: &[u8]| parse_release(body).unwrap_err().to_string();
        assert!(message(b"not json").contains("Failed to parse GitHub response"));
        assert!(message(br#"{"message":"Not Found"}"#).contains("GitHub API error: Not Found"));
        assert!(message(br#"{"message":404}"#).contains("GitHub API error: Unknown error"));
        assert!(message(br#"{"assets":[]}"#).contains("Missing tag_name"));
        assert!(message(br#"{"tag_name":"v1"}"#).contains("Missing assets"));
    }

    #[test]
    fn a_file_that_cannot_be_read_has_no_checksum() {
        let dir = tempfile::tempdir().unwrap();
        let message = sha256_file(&dir.path().join("absent"))
            .unwrap_err()
            .to_string();
        assert!(message.contains("Failed to read"), "{message}");
    }

    #[test]
    fn the_public_update_starts_with_its_heading() {
        // check_only: whatever the network answers, nothing is installed
        let (_, captured) = crate::cli::console::capture(|c| update(c, true, false));
        assert!(
            captured.out.starts_with("🔄 Forge - Update Check"),
            "{}",
            captured.out
        );
        assert!(
            captured.out.contains("Current version:"),
            "{}",
            captured.out
        );
    }

    #[test]
    fn update_stops_when_it_cannot_find_what_it_needs() {
        let fixture = Fixture::at("v99.0.0");
        let dir = fixture.dir.path();
        let mut input = Cursor::new(b"y\n".to_vec());
        let good = || Target {
            platform: detect_platform(),
            install_path: fixture.install_path(),
            work_dir: dir.to_path_buf(),
        };

        let no_platform = Target {
            platform: platform_for("plan9", "mips"),
            ..good()
        };
        let (result, _) = run_target(&fixture, &["tar"], &no_platform, false, false, &mut input);
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported platform"));

        let no_binary = Target {
            install_path: PathBuf::new(),
            ..good()
        };
        let (result, _) = run_target(&fixture, &["tar"], &no_binary, false, false, &mut input);
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot tell which binary"));

        let no_work_dir = Target {
            work_dir: dir.join("absent"),
            ..good()
        };
        let (result, _) = run_target(&fixture, &["tar"], &no_work_dir, false, false, &mut input);
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to create temp dir"));
    }

    #[test]
    fn update_stops_when_the_release_cannot_be_fetched() {
        let fixture = Fixture::at("v99.0.0");
        let missing = Tools {
            curl: &["/no/such/curl"],
            tar: &["tar"],
        };
        let target = Target {
            platform: detect_platform(),
            install_path: fixture.install_path(),
            work_dir: env::temp_dir(),
        };
        let (mut out, mut err, mut input) = (Vec::new(), Vec::new(), Cursor::new(Vec::new()));
        let result = update_with(
            &mut Console::new(&mut out, &mut err, &mut input),
            &missing,
            &target,
            true,
            false,
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to run curl"));
    }

    #[test]
    fn update_stops_when_the_answer_cannot_be_read() {
        let fixture = Fixture::at("v99.0.0");
        let target = Target {
            platform: detect_platform(),
            install_path: fixture.install_path(),
            work_dir: env::temp_dir(),
        };
        let (result, _) = run_target(
            &fixture,
            &["tar"],
            &target,
            false,
            false,
            &mut std::io::BufReader::new(BrokenInput),
        );
        assert!(result.unwrap_err().to_string().contains("no terminal"));
        assert!(!target.install_path.exists());
    }

    #[test]
    fn update_reports_an_archive_that_cannot_be_downloaded() {
        let fixture = Fixture::at("v99.0.0");
        fs::write(
            fixture.dir.path().join("curl.sh"),
            "if [ \"$1\" = \"-sL\" ]; then cat \"$(dirname \"$0\")/release.json\"; exit 0; fi\n\
             while [ $# -gt 1 ]; do if [ \"$1\" = \"--output\" ]; then dest=\"$2\"; fi; shift; done\n\
             case \"$1\" in */checksums.txt) cp \"$(dirname \"$0\")/checksums.txt\" \"$dest\" ;; *) exit 22 ;; esac\n",
        )
        .unwrap();
        let (result, _) = run_update(
            &fixture,
            &["tar"],
            &fixture.install_path(),
            false,
            false,
            "y\n",
        );
        assert!(result.unwrap_err().to_string().contains("Download failed"));
    }

    #[test]
    fn checksum_lines_without_a_file_name_are_skipped() {
        let digest = "a".repeat(64);
        let parsed = parse_checksums(&format!("{digest}\n\n{digest}  dir/file.tar.gz\n"));
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed["file.tar.gz"], digest);
    }

    #[test]
    fn a_file_that_is_missing_fails_the_checksum() {
        let dir = tempfile::tempdir().unwrap();
        let mut checksums = HashMap::new();
        checksums.insert("x".to_string(), "0".repeat(64));
        let err = verify_checksum("x", &dir.path().join("absent"), &checksums).unwrap_err();
        assert!(err.to_string().contains("Failed to read"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn the_installed_binary_is_executable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("forge");
        write_executable(&path, b"bin").unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "{mode:o}");
        assert_eq!(fs::read(&path).unwrap(), b"bin");
    }
}
