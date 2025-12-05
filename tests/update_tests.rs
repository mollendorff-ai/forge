//! Update module tests
//! ADR-004: 100% coverage required

use royalbit_forge::update::{VersionCheck, CURRENT_VERSION};

// ═══════════════════════════════════════════════════════════════════════════
// CURRENT VERSION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_current_version_format() {
    // CURRENT_VERSION should be a valid semver (x.y.z)
    let parts: Vec<&str> = CURRENT_VERSION.split('.').collect();
    assert_eq!(parts.len(), 3, "Version should have 3 parts");

    // Each part should be a number
    for part in &parts {
        assert!(
            part.parse::<u32>().is_ok(),
            "Version part '{}' should be numeric",
            part
        );
    }
}

#[test]
fn test_current_version_not_empty() {
    assert!(!CURRENT_VERSION.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// VERSION CHECK STRUCT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_version_check_no_update_available() {
    let check = VersionCheck {
        current: "4.3.0".to_string(),
        latest: "4.3.0".to_string(),
        update_available: false,
        download_url: None,
        checksums_url: None,
    };

    assert!(!check.update_available);
    assert!(check.download_url.is_none());
    assert!(check.checksums_url.is_none());
}

#[test]
fn test_version_check_update_available() {
    let check = VersionCheck {
        current: "4.2.0".to_string(),
        latest: "4.3.0".to_string(),
        update_available: true,
        download_url: Some("https://example.com/forge.tar.gz".to_string()),
        checksums_url: Some("https://example.com/checksums.txt".to_string()),
    };

    assert!(check.update_available);
    assert_eq!(check.current, "4.2.0");
    assert_eq!(check.latest, "4.3.0");
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

    let debug_str = format!("{:?}", check);
    assert!(debug_str.contains("VersionCheck"));
    assert!(debug_str.contains("1.0.0"));
    assert!(debug_str.contains("2.0.0"));
}

#[test]
fn test_version_check_partial_urls() {
    // Only download URL, no checksums
    let check = VersionCheck {
        current: "4.0.0".to_string(),
        latest: "4.1.0".to_string(),
        update_available: true,
        download_url: Some("https://example.com/binary".to_string()),
        checksums_url: None,
    };

    assert!(check.download_url.is_some());
    assert!(check.checksums_url.is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// These tests for internal functions are covered by the unit tests in update.rs
// ═══════════════════════════════════════════════════════════════════════════

// The is_newer_version and extract_json_string functions are tested
// in the #[cfg(test)] module inside update.rs itself
