//! Self-update functionality for Forge CLI
//!
//! Checks GitHub Releases for new versions and updates the binary in-place.

// During coverage builds, stubbed functions don't use all imports/constants
#![cfg_attr(coverage, allow(unused_imports, dead_code))]

use std::env;
use std::fs;

/// GitHub API URL for latest release
const GITHUB_RELEASES_URL: &str = "https://api.github.com/repos/royalbit/forge/releases/latest";

/// Current version from Cargo.toml
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Result of version check
#[derive(Debug)]
pub struct VersionCheck {
    pub current: String,
    pub latest: String,
    pub update_available: bool,
    pub download_url: Option<String>,
    pub checksums_url: Option<String>,
}

/// Get the appropriate asset name for the current platform
fn get_platform_asset() -> Option<&'static str> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Some("forge-x86_64-unknown-linux-gnu.tar.gz");

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return Some("forge-aarch64-unknown-linux-gnu.tar.gz");

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Some("forge-aarch64-apple-darwin.tar.gz");

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Some("forge-x86_64-apple-darwin.tar.gz");

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Some("forge-x86_64-pc-windows-msvc.zip");

    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    return None;
}

/// Check for updates by querying GitHub Releases API
///
/// # Coverage Exclusion (ADR-006)
/// Makes HTTP request to GitHub API - cannot unit test network calls
#[cfg(not(coverage))]
pub fn check_for_update() -> Result<VersionCheck, String> {
    // Use curl to fetch the release info (available on all platforms)
    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-H",
            "Accept: application/vnd.github.v3+json",
            "-H",
            "User-Agent: forge-cli",
            GITHUB_RELEASES_URL,
        ])
        .output()
        .map_err(|e| format!("Failed to fetch release info: {}", e))?;

    if !output.status.success() {
        return Err("Failed to fetch release info from GitHub".to_string());
    }

    let body = String::from_utf8_lossy(&output.stdout);

    // Parse version from JSON (simple extraction without serde_json dependency)
    let latest_version = extract_json_string(&body, "tag_name")
        .ok_or("Could not parse version from GitHub response")?
        .trim_start_matches('v')
        .to_string();

    let update_available = is_newer_version(&latest_version, CURRENT_VERSION);

    // Find download URL for current platform
    let download_url = if update_available {
        get_platform_asset().and_then(|asset_name| {
            // Find the browser_download_url for our asset
            // Try both with and without space after colon (GitHub uses space)
            let search_with_space = format!("\"name\": \"{}\"", asset_name);
            let search_no_space = format!("\"name\":\"{}\"", asset_name);
            let pos = body
                .find(&search_with_space)
                .or_else(|| body.find(&search_no_space));
            if let Some(pos) = pos {
                // Look for browser_download_url near this position
                let chunk = &body[pos.saturating_sub(500)..body.len().min(pos + 500)];
                extract_json_string(chunk, "browser_download_url")
                    .filter(|url| url.contains(asset_name))
            } else {
                None
            }
        })
    } else {
        None
    };

    // Find checksums.txt URL
    let checksums_url = if update_available {
        // Try both with and without space after colon
        let pos = body
            .find("\"name\": \"checksums.txt\"")
            .or_else(|| body.find("\"name\":\"checksums.txt\""));
        if let Some(pos) = pos {
            let chunk = &body[pos.saturating_sub(500)..body.len().min(pos + 500)];
            extract_json_string(chunk, "browser_download_url")
                .filter(|url| url.contains("checksums.txt"))
        } else {
            None
        }
    } else {
        None
    };

    Ok(VersionCheck {
        current: CURRENT_VERSION.to_string(),
        latest: latest_version,
        update_available,
        download_url,
        checksums_url,
    })
}

/// Stub for coverage builds - see ADR-006
#[cfg(coverage)]
pub fn check_for_update() -> Result<VersionCheck, String> {
    Ok(VersionCheck {
        current: CURRENT_VERSION.to_string(),
        latest: CURRENT_VERSION.to_string(),
        update_available: false,
        download_url: None,
        checksums_url: None,
    })
}

/// Simple JSON string extraction (avoids adding serde_json dependency)
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    // Try with space after colon first (GitHub style), then without
    let search_with_space = format!("\"{}\": \"", key);
    let search_no_space = format!("\"{}\":\"", key);

    let (start, search_len) = json
        .find(&search_with_space)
        .map(|pos| (pos, search_with_space.len()))
        .or_else(|| {
            json.find(&search_no_space)
                .map(|pos| (pos, search_no_space.len()))
        })?;

    let value_start = start + search_len;
    let end = json[value_start..].find('"')?;
    Some(json[value_start..value_start + end].to_string())
}

/// Compare semantic versions (returns true if latest > current)
fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse_version =
        |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse().ok()).collect() };

    let latest_parts = parse_version(latest);
    let current_parts = parse_version(current);

    for i in 0..3 {
        let l = latest_parts.get(i).copied().unwrap_or(0);
        let c = current_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }
    false
}

/// Download and install the update with optional checksum verification
///
/// # Coverage Exclusion (ADR-006)
/// Downloads files from internet and replaces binary - cannot unit test
#[cfg(not(coverage))]
pub fn perform_update(download_url: &str, checksums_url: Option<&str>) -> Result<(), String> {
    let current_exe = env::current_exe()
        .map_err(|e| format!("Could not determine current executable path: {}", e))?;

    println!("  Downloading update...");

    // Download to temp file
    let temp_dir = env::temp_dir();

    #[cfg(not(target_os = "windows"))]
    let temp_archive = temp_dir.join("forge_update.tar.gz");

    #[cfg(target_os = "windows")]
    let temp_archive = temp_dir.join("forge_update.zip");

    let download_status = std::process::Command::new("curl")
        .args(["-L", "-o", temp_archive.to_str().unwrap(), download_url])
        .status()
        .map_err(|e| format!("Failed to download update: {}", e))?;

    if !download_status.success() {
        return Err("Download failed".to_string());
    }

    // Verify checksum if available
    if let Some(checksums_url) = checksums_url {
        println!("  Verifying checksum...");
        if let Some(asset_name) = get_platform_asset() {
            verify_checksum(&temp_archive, checksums_url, asset_name)?;
        }
    }

    println!("  Extracting...");

    // Extract the binary
    #[cfg(not(target_os = "windows"))]
    let temp_binary = temp_dir.join("forge");

    #[cfg(target_os = "windows")]
    let temp_binary = temp_dir.join("forge.exe");

    #[cfg(not(target_os = "windows"))]
    {
        let extract_status = std::process::Command::new("tar")
            .args([
                "-xzf",
                temp_archive.to_str().unwrap(),
                "-C",
                temp_dir.to_str().unwrap(),
            ])
            .status()
            .map_err(|e| format!("Failed to extract update: {}", e))?;

        if !extract_status.success() {
            return Err("Extraction failed".to_string());
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Windows uses zip files
        // Use PowerShell to extract
        let extract_status = std::process::Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    temp_archive.display(),
                    temp_dir.display()
                ),
            ])
            .status()
            .map_err(|e| format!("Failed to extract update: {}", e))?;

        if !extract_status.success() {
            return Err("Extraction failed".to_string());
        }
    }

    // Verify extracted binary exists
    if !temp_binary.exists() {
        return Err(format!(
            "Extracted binary not found at {}",
            temp_binary.display()
        ));
    }

    println!("  Installing...");

    // Replace current executable
    // On Unix, we can't replace a running executable directly, so we rename first
    let backup_path = current_exe.with_extension("old");

    // Remove old backup if exists
    let _ = fs::remove_file(&backup_path);

    // Rename current to backup
    fs::rename(&current_exe, &backup_path)
        .map_err(|e| format!("Failed to backup current binary: {}", e))?;

    // Move new binary to current location
    fs::copy(&temp_binary, &current_exe)
        .map_err(|e| format!("Failed to install new binary: {}", e))?;

    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&current_exe)
            .map_err(|e| format!("Failed to get permissions: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&current_exe, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    // Cleanup
    let _ = fs::remove_file(&temp_archive);
    let _ = fs::remove_file(&temp_binary);
    let _ = fs::remove_file(&backup_path);

    Ok(())
}

/// Stub for coverage builds - see ADR-006
#[cfg(coverage)]
pub fn perform_update(_download_url: &str, _checksums_url: Option<&str>) -> Result<(), String> {
    Ok(())
}

/// Verify SHA256 checksum of downloaded file
///
/// # Coverage Exclusion (ADR-006)
/// Downloads checksums.txt from internet - cannot unit test network calls
#[cfg(not(coverage))]
fn verify_checksum(
    file_path: &std::path::Path,
    checksums_url: &str,
    asset_name: &str,
) -> Result<(), String> {
    // Download checksums.txt
    let output = std::process::Command::new("curl")
        .args(["-sL", checksums_url])
        .output()
        .map_err(|e| format!("Failed to download checksums: {}", e))?;

    if !output.status.success() {
        return Err("Failed to download checksums.txt".to_string());
    }

    let checksums = String::from_utf8_lossy(&output.stdout);

    // Find the expected checksum for our asset
    let expected_checksum = checksums
        .lines()
        .find(|line| line.contains(asset_name))
        .and_then(|line| line.split_whitespace().next())
        .ok_or_else(|| format!("Checksum not found for {}", asset_name))?;

    // Calculate actual checksum using sha256sum (Unix) or certutil (Windows)
    #[cfg(not(target_os = "windows"))]
    let actual_checksum = {
        let output = std::process::Command::new("sha256sum")
            .arg(file_path)
            .output()
            .map_err(|e| format!("Failed to calculate checksum: {}", e))?;

        if !output.status.success() {
            return Err("Failed to calculate SHA256 checksum".to_string());
        }

        String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string()
    };

    #[cfg(target_os = "windows")]
    let actual_checksum = {
        let output = std::process::Command::new("certutil")
            .args(["-hashfile", file_path.to_str().unwrap(), "SHA256"])
            .output()
            .map_err(|e| format!("Failed to calculate checksum: {}", e))?;

        if !output.status.success() {
            return Err("Failed to calculate SHA256 checksum".to_string());
        }

        // certutil output has checksum on second line
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .nth(1)
            .unwrap_or("")
            .trim()
            .replace(' ', "")
            .to_lowercase()
    };

    if actual_checksum != expected_checksum {
        return Err(format!(
            "Checksum mismatch!\n  Expected: {}\n  Actual:   {}",
            expected_checksum, actual_checksum
        ));
    }

    Ok(())
}

/// Stub for coverage builds - see ADR-006
#[cfg(coverage)]
fn verify_checksum(
    _file_path: &std::path::Path,
    _checksums_url: &str,
    _asset_name: &str,
) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════
    // VERSION COMPARISON TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("4.3.0", "4.2.1"));
        assert!(is_newer_version("5.0.0", "4.9.9"));
        assert!(is_newer_version("4.2.2", "4.2.1"));
        assert!(!is_newer_version("4.2.1", "4.2.1"));
        assert!(!is_newer_version("4.2.0", "4.2.1"));
        assert!(!is_newer_version("4.2.1", "4.3.0"));
    }

    #[test]
    fn test_version_comparison_major() {
        assert!(is_newer_version("2.0.0", "1.9.9"));
        assert!(is_newer_version("10.0.0", "9.99.99"));
        assert!(!is_newer_version("1.0.0", "2.0.0"));
    }

    #[test]
    fn test_version_comparison_minor() {
        assert!(is_newer_version("1.2.0", "1.1.0"));
        assert!(is_newer_version("1.10.0", "1.9.0"));
        assert!(!is_newer_version("1.1.0", "1.2.0"));
    }

    #[test]
    fn test_version_comparison_patch() {
        assert!(is_newer_version("1.0.2", "1.0.1"));
        assert!(is_newer_version("1.0.10", "1.0.9"));
        assert!(!is_newer_version("1.0.1", "1.0.2"));
    }

    #[test]
    fn test_version_comparison_incomplete() {
        // Missing parts should be treated as 0
        assert!(is_newer_version("1.1", "1.0.0"));
        assert!(is_newer_version("2", "1.0.0"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // JSON STRING EXTRACTION TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_extract_json_string() {
        // Without spaces (compact JSON)
        let json = r#"{"tag_name":"v4.3.0","name":"Release 4.3.0"}"#;
        assert_eq!(
            extract_json_string(json, "tag_name"),
            Some("v4.3.0".to_string())
        );
        assert_eq!(
            extract_json_string(json, "name"),
            Some("Release 4.3.0".to_string())
        );
        assert_eq!(extract_json_string(json, "missing"), None);

        // With spaces (GitHub API style)
        let json_spaced = r#"{"tag_name": "v4.3.0", "name": "Release 4.3.0"}"#;
        assert_eq!(
            extract_json_string(json_spaced, "tag_name"),
            Some("v4.3.0".to_string())
        );
        assert_eq!(
            extract_json_string(json_spaced, "name"),
            Some("Release 4.3.0".to_string())
        );
    }

    #[test]
    fn test_extract_json_string_empty() {
        assert_eq!(extract_json_string("", "key"), None);
        assert_eq!(extract_json_string("{}", "key"), None);
    }

    #[test]
    fn test_extract_json_string_nested() {
        // Find key in nested structure
        let json = r#"{"outer": {"inner": "value"}}"#;
        assert_eq!(
            extract_json_string(json, "inner"),
            Some("value".to_string())
        );
    }

    #[test]
    fn test_extract_json_string_url() {
        let json = r#"{"browser_download_url": "https://github.com/royalbit/forge/releases/download/v4.3.0/forge.tar.gz"}"#;
        assert_eq!(
            extract_json_string(json, "browser_download_url"),
            Some(
                "https://github.com/royalbit/forge/releases/download/v4.3.0/forge.tar.gz"
                    .to_string()
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // CURRENT VERSION TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_current_version_set() {
        // CURRENT_VERSION comes from CARGO_PKG_VERSION, always valid semver
        assert!(CURRENT_VERSION.contains('.'));
    }

    #[test]
    fn test_current_version_valid_semver() {
        let parts: Vec<&str> = CURRENT_VERSION.split('.').collect();
        assert!(parts.len() >= 2, "Should have at least major.minor");
        for part in parts {
            assert!(
                part.parse::<u32>().is_ok(),
                "Version part '{}' should be numeric",
                part
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // PLATFORM ASSET TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_get_platform_asset_returns_option() {
        // This will return Some on supported platforms, None on unsupported
        let asset = get_platform_asset();
        // On a standard CI/dev machine, should return Some
        #[cfg(any(
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "x86_64")
        ))]
        assert!(asset.is_some());

        // If Some, should contain tar.gz or zip
        if let Some(name) = asset {
            assert!(name.contains("forge-"));
            assert!(name.ends_with(".tar.gz") || name.ends_with(".zip"));
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // VERSION CHECK STRUCT TESTS
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_version_check_struct() {
        let check = VersionCheck {
            current: "4.2.0".to_string(),
            latest: "4.3.0".to_string(),
            update_available: true,
            download_url: Some("https://example.com/forge.tar.gz".to_string()),
            checksums_url: Some("https://example.com/checksums.txt".to_string()),
        };

        assert_eq!(check.current, "4.2.0");
        assert_eq!(check.latest, "4.3.0");
        assert!(check.update_available);
        assert!(check.download_url.is_some());
        assert!(check.checksums_url.is_some());
    }

    #[test]
    fn test_version_check_debug() {
        let check = VersionCheck {
            current: "1.0.0".to_string(),
            latest: "2.0.0".to_string(),
            update_available: true,
            download_url: None,
            checksums_url: None,
        };

        let debug = format!("{:?}", check);
        assert!(debug.contains("VersionCheck"));
        assert!(debug.contains("1.0.0"));
        assert!(debug.contains("2.0.0"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ADDITIONAL VERSION TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_version_comparison_equal() {
        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("5.0.0", "5.0.0"));
        assert!(!is_newer_version("0.0.0", "0.0.0"));
    }

    #[test]
    fn test_version_comparison_older() {
        assert!(!is_newer_version("1.0.0", "2.0.0"));
        assert!(!is_newer_version("1.1.0", "1.2.0"));
        assert!(!is_newer_version("1.0.1", "1.0.2"));
    }

    #[test]
    fn test_version_comparison_invalid_chars() {
        // Non-numeric parts get filtered out, resulting in empty vectors
        // Empty vs any version means equal (all 0s), so not newer
        assert!(!is_newer_version("abc", "1.0.0"));
        // 1.0.0 vs empty is newer since 1 > 0
        assert!(is_newer_version("1.0.0", "abc"));
    }

    #[test]
    fn test_extract_json_string_special_chars() {
        let json = r#"{"key": "value with spaces and: colons"}"#;
        assert_eq!(
            extract_json_string(json, "key"),
            Some("value with spaces and: colons".to_string())
        );
    }

    #[test]
    fn test_extract_json_string_first_occurrence() {
        // Should find first occurrence
        let json = r#"{"key": "first", "other": {"key": "second"}}"#;
        assert_eq!(extract_json_string(json, "key"), Some("first".to_string()));
    }

    #[test]
    fn test_version_check_no_update() {
        let check = VersionCheck {
            current: "5.0.0".to_string(),
            latest: "4.0.0".to_string(),
            update_available: false,
            download_url: None,
            checksums_url: None,
        };
        assert!(!check.update_available);
        assert!(check.download_url.is_none());
    }

    #[test]
    fn test_github_releases_url_constant() {
        assert!(GITHUB_RELEASES_URL.contains("github.com"));
        assert!(GITHUB_RELEASES_URL.contains("releases"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // PLATFORM ASSET TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_get_platform_asset() {
        // This test verifies get_platform_asset returns something for supported platforms
        let asset = get_platform_asset();

        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        assert_eq!(asset, Some("forge-x86_64-unknown-linux-gnu.tar.gz"));

        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        assert_eq!(asset, Some("forge-aarch64-unknown-linux-gnu.tar.gz"));

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        assert_eq!(asset, Some("forge-aarch64-apple-darwin.tar.gz"));

        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        assert_eq!(asset, Some("forge-x86_64-apple-darwin.tar.gz"));

        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        assert_eq!(asset, Some("forge-x86_64-pc-windows-msvc.zip"));

        // Asset name contains "forge"
        if let Some(name) = asset {
            assert!(name.contains("forge"));
        }
    }

    #[test]
    fn test_current_version_constant() {
        // CURRENT_VERSION should be a valid semver-ish string with major.minor format
        assert!(CURRENT_VERSION.contains('.'));
        assert!(CURRENT_VERSION.len() >= 3); // At least "x.y"
    }

    // ═══════════════════════════════════════════════════════════════════════
    // EXTRACT JSON STRING EDGE CASES
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_extract_json_string_empty_value() {
        let json = r#"{"key": ""}"#;
        assert_eq!(extract_json_string(json, "key"), Some("".to_string()));
    }

    #[test]
    fn test_extract_json_string_escaped_quotes() {
        let json = r#"{"key": "value with \"escaped\" quotes"}"#;
        // The function doesn't handle escaped quotes perfectly, but shouldn't crash
        let result = extract_json_string(json, "key");
        assert!(result.is_some());
    }

    #[test]
    fn test_extract_json_string_url_value() {
        let json = r#"{"browser_download_url": "https://github.com/releases/forge.tar.gz"}"#;
        assert_eq!(
            extract_json_string(json, "browser_download_url"),
            Some("https://github.com/releases/forge.tar.gz".to_string())
        );
    }

    #[test]
    fn test_extract_json_string_nested_json() {
        let json = r#"{
            "release": {
                "tag_name": "v5.0.0",
                "assets": []
            }
        }"#;
        assert_eq!(
            extract_json_string(json, "tag_name"),
            Some("v5.0.0".to_string())
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // VERSION CHECK STRUCT TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_version_check_with_urls() {
        let check = VersionCheck {
            current: "4.0.0".to_string(),
            latest: "5.0.0".to_string(),
            update_available: true,
            download_url: Some("https://example.com/forge.tar.gz".to_string()),
            checksums_url: Some("https://example.com/checksums.txt".to_string()),
        };
        assert!(check.update_available);
        assert!(check.download_url.is_some());
        assert!(check.checksums_url.is_some());
        assert_eq!(
            check.download_url.as_ref().unwrap(),
            "https://example.com/forge.tar.gz"
        );
    }

    #[test]
    fn test_version_comparison_with_v_prefix() {
        // Sometimes versions come with 'v' prefix
        let v1 = "5.0.0";
        let v2 = "4.0.0";
        assert!(is_newer_version(v1, v2));

        // Test stripping v prefix (like we do in check_for_update)
        let tagged = "v5.0.0";
        let stripped = tagged.trim_start_matches('v');
        assert_eq!(stripped, "5.0.0");
    }

    #[test]
    fn test_version_comparison_large_numbers() {
        assert!(is_newer_version("100.0.0", "99.999.999"));
        assert!(is_newer_version("1.100.0", "1.99.0"));
        assert!(is_newer_version("1.0.100", "1.0.99"));
    }
}
