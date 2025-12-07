//! Upgrade tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;
use super::common::create_test_yaml;
use tempfile::TempDir;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_upgrade_dry_run() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "upgrade_dry.yaml",
        r#"_forge_version: "4.0.0"
_name: "test"
my_input:
  value: 100
my_output:
  value: 200
  formula: "=my_input * 2"
"#,
    );

    let result = upgrade(yaml, true, "5.0.0".to_string(), false);
    assert!(result.is_ok());
}

#[test]
fn test_upgrade_already_current() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "upgrade_current.yaml",
        r#"_forge_version: "5.0.0"
_name: "test"
inputs:
  x:
    value: 10
"#,
    );

    let result = upgrade(yaml, true, "5.0.0".to_string(), true);
    assert!(result.is_ok());
}

#[test]
fn test_upgrade_with_write() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "upgrade_write.yaml",
        r#"_forge_version: "4.0.0"
_name: "test"
value_only:
  value: 50
"#,
    );

    let result = upgrade(yaml.clone(), false, "5.0.0".to_string(), true);
    assert!(result.is_ok());

    // Backup should exist
    let backup = yaml.with_extension("yaml.bak");
    assert!(backup.exists());
}

#[test]
fn test_upgrade_file_not_found() {
    let result = upgrade(
        PathBuf::from("/nonexistent.yaml"),
        true,
        "5.0.0".to_string(),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn test_upgrade_with_includes() {
    let dir = TempDir::new().unwrap();

    // Create included file first
    create_test_yaml(
        &dir,
        "included.yaml",
        r#"_forge_version: "4.0.0"
_name: "included"
inc_value:
  value: 25
"#,
    );

    // Create main file with include reference
    let yaml = create_test_yaml(
        &dir,
        "main.yaml",
        r#"_forge_version: "4.0.0"
_name: "main"
_includes:
  - file: "included.yaml"
main_value:
  value: 100
"#,
    );

    let result = upgrade(yaml, true, "5.0.0".to_string(), true);
    assert!(result.is_ok());
}
