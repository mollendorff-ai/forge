//! Integration tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;
use super::common::create_test_yaml;
use tempfile::TempDir;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_validate_single_file_empty_model() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "empty.yaml",
        "_forge_version: \"5.0.0\"\n_name: \"empty\"\n",
    );

    // Empty model should pass validation with warning
    let result = validate_single_file(&yaml);
    assert!(result.is_ok());
}

#[test]
fn test_validate_single_file_valid_model() {
    let dir = TempDir::new().unwrap();
    // Use simple scalar format (like test.yaml) that works with v1.0.0
    let yaml = create_test_yaml(
        &dir,
        "valid.yaml",
        r#"_forge_version: "1.0.0"
summary:
  price:
    value: 100
    formula: null
  result:
    value: 200
    formula: "=price * 2"
"#,
    );

    let result = validate_single_file(&yaml);
    assert!(result.is_ok());
}

#[test]
fn test_validate_batch() {
    let dir = TempDir::new().unwrap();
    let yaml1 = create_test_yaml(
        &dir,
        "file1.yaml",
        "_forge_version: \"5.0.0\"\n_name: \"file1\"\n",
    );
    let yaml2 = create_test_yaml(
        &dir,
        "file2.yaml",
        "_forge_version: \"5.0.0\"\n_name: \"file2\"\n",
    );

    let result = validate(vec![yaml1, yaml2]);
    assert!(result.is_ok());
}

#[test]
fn test_validate_internal_success() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "test.yaml",
        r#"
_forge_version: "5.0.0"
_name: "test"
inputs:
  x:
    value: 10
"#,
    );

    let result = validate_internal(&yaml, false);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_internal_success() {
    let dir = TempDir::new().unwrap();
    // Use simple scalar format that works with v1.0.0
    let yaml = create_test_yaml(
        &dir,
        "calc.yaml",
        r#"_forge_version: "1.0.0"
summary:
  price:
    value: 50
    formula: null
  total:
    value: 100
    formula: "=price * 2"
"#,
    );

    let result = calculate_internal(&yaml, true);
    assert!(result.is_ok());
}

#[test]
fn test_functions_command_text() {
    // Just verify it doesn't panic
    let result = functions(false);
    assert!(result.is_ok());
}

#[test]
fn test_functions_command_json() {
    // Just verify it doesn't panic
    let result = functions(true);
    assert!(result.is_ok());
}

#[test]
fn test_run_watch_action_validate() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "watch.yaml",
        "_forge_version: \"5.0.0\"\n_name: \"watch\"\n",
    );

    // Just verify it doesn't panic
    run_watch_action(&yaml, true, false);
}

#[test]
fn test_run_watch_action_calculate() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "watch.yaml",
        r#"
_forge_version: "5.0.0"
_name: "watch"
inputs:
  x:
    value: 5
"#,
    );

    // Just verify it doesn't panic
    run_watch_action(&yaml, false, true);
}

#[test]
fn test_run_watch_action_validate_error() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "watch_invalid.yaml",
        r#"_forge_version: "1.0.0"
result:
  value: null
  formula: "=nonexistent_var"
"#,
    );

    // Should not panic even with validation error
    run_watch_action(&yaml, true, false);
}

#[test]
fn test_run_watch_action_calculate_error() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "watch_calc_err.yaml",
        r#"_forge_version: "1.0.0"
x:
  value: null
  formula: "=undefined_var"
"#,
    );

    // Should not panic even with calculation error
    run_watch_action(&yaml, false, false);
}

#[test]
fn test_run_watch_action_validate_verbose() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "watch_verbose.yaml",
        r#"_forge_version: "1.0.0"
sales:
  revenue: [100, 200, 300]
total:
  value: null
  formula: "=SUM(sales.revenue)"
"#,
    );

    // Test with verbose mode (covers verbose output paths)
    run_watch_action(&yaml, true, true);
}

#[test]
fn test_run_watch_action_calculate_with_tables() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "watch_tables.yaml",
        r#"_forge_version: "1.0.0"
data:
  qty: [1, 2, 3]
  price: [10, 20, 30]
  total:
    formula: "=data.qty * data.price"
"#,
    );

    // Test calculate with tables (covers table output path)
    run_watch_action(&yaml, false, true);
}

#[test]
fn test_run_watch_action_mismatch() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "watch_mismatch.yaml",
        r#"_forge_version: "1.0.0"
a:
  value: 10
  formula: null
b:
  value: 999
  formula: "=a * 2"
"#,
    );

    // Validation should fail due to mismatch, testing error path
    run_watch_action(&yaml, true, false);
}
