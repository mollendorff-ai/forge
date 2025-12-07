//! Export tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;
use super::common::create_test_yaml;
use tempfile::TempDir;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_export_basic() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "export.yaml",
        r#"_forge_version: "1.0.0"
summary:
  price:
    value: 100
    formula: null
"#,
    );
    let output = dir.path().join("output.xlsx");

    let result = export(yaml, output.clone(), false);
    assert!(result.is_ok());
    assert!(output.exists());
}

#[test]
fn test_export_verbose() {
    let dir = TempDir::new().unwrap();
    // Use proper table array format
    let yaml = create_test_yaml(
        &dir,
        "export_verbose.yaml",
        r#"_forge_version: "1.0.0"
sales:
  month: [1, 2, 3]
  revenue: [100, 200, 300]
"#,
    );
    let output = dir.path().join("output_verbose.xlsx");

    let result = export(yaml, output.clone(), true);
    assert!(result.is_ok());
    assert!(output.exists());
}

#[test]
fn test_export_file_not_found() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("output.xlsx");

    let result = export(PathBuf::from("/nonexistent.yaml"), output, false);
    assert!(result.is_err());
}
