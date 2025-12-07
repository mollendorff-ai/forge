//! Validate tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;
use super::common::create_test_yaml;
use tempfile::TempDir;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_validate_single_invalid() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(&dir, "invalid.yaml", "not: [valid yaml");

    let result = validate(vec![yaml]);
    assert!(result.is_err());
}

#[test]
fn test_validate_batch_with_failures() {
    let dir = TempDir::new().unwrap();
    let valid = create_test_yaml(
        &dir,
        "valid.yaml",
        "_forge_version: \"5.0.0\"\n_name: \"valid\"\n",
    );
    let invalid = create_test_yaml(&dir, "invalid.yaml", "broken: [yaml");

    let result = validate(vec![valid, invalid]);
    assert!(result.is_err());
}

#[test]
fn test_validate_mismatch_values() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "mismatch.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
  y:
    value: 999
    formula: "=x * 2"
"#,
    );

    let result = validate(vec![yaml]);
    assert!(result.is_err());
}

#[test]
fn test_validate_table_length_mismatch() {
    let dir = TempDir::new().unwrap();
    let yaml_path = dir.path().join("bad_table.yaml");
    // Create a YAML that will fail table length validation
    // (inconsistent column lengths)
    std::fs::write(
        &yaml_path,
        r#"_forge_version: "1.0.0"
sales:
  month: [1, 2, 3]
  revenue: [100, 200]
"#,
    )
    .unwrap();

    let result = validate(vec![yaml_path]);
    assert!(result.is_err());
}

#[test]
fn test_validate_calculation_error() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "calc_error.yaml",
        r#"_forge_version: "1.0.0"
a:
  value: 10
  formula: null
b:
  value: null
  formula: "=nonexistent_var"
"#,
    );

    let result = validate(vec![yaml]);
    assert!(result.is_err());
}
