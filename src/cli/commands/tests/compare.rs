//! Compare tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;
use super::common::create_test_yaml;
use tempfile::TempDir;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_compare_two_scenarios() {
    let dir = TempDir::new().unwrap();
    // Use top-level scalars for scenario overrides to work
    let yaml = create_test_yaml(
        &dir,
        "compare.yaml",
        r#"_forge_version: "1.0.0"
rate:
  value: 0.05
  formula: null
revenue:
  value: 1000
  formula: null
profit:
  value: null
  formula: "=revenue * rate"
scenarios:
  low:
    rate: 0.03
  high:
    rate: 0.10
"#,
    );

    let result = compare(yaml, vec!["low".to_string(), "high".to_string()], false);
    assert!(result.is_ok());
}

#[test]
fn test_compare_verbose() {
    let dir = TempDir::new().unwrap();
    // Use top-level scalars
    let yaml = create_test_yaml(
        &dir,
        "compare_verbose.yaml",
        r#"_forge_version: "1.0.0"
x:
  value: 10
  formula: null
scenarios:
  a:
    x: 5
  b:
    x: 15
"#,
    );

    let result = compare(yaml, vec!["a".to_string(), "b".to_string()], true);
    assert!(result.is_ok());
}

#[test]
fn test_compare_with_formula_only_vars() {
    let dir = TempDir::new().unwrap();
    // Variable with formula but no value (covers value=None path)
    let yaml = create_test_yaml(
        &dir,
        "compare_formula.yaml",
        r#"_forge_version: "1.0.0"
base:
  value: 100
  formula: null
derived:
  value: null
  formula: "=base * 2"
scenarios:
  s1:
    base: 50
  s2:
    base: 200
"#,
    );

    let result = compare(yaml, vec!["s1".to_string(), "s2".to_string()], false);
    assert!(result.is_ok());
}

#[test]
fn test_compare_scenario_not_found() {
    let dir = TempDir::new().unwrap();
    // Use top-level scalars and correct scenario format
    let yaml = create_test_yaml(
        &dir,
        "compare_notfound.yaml",
        r#"_forge_version: "1.0.0"
x:
  value: 10
  formula: null
scenarios:
  exists:
    x: 5
"#,
    );

    let result = compare(
        yaml,
        vec!["exists".to_string(), "missing".to_string()],
        false,
    );
    assert!(result.is_err());
}

#[test]
fn test_compare_file_not_found() {
    let result = compare(
        PathBuf::from("/nonexistent.yaml"),
        vec!["a".to_string()],
        false,
    );
    assert!(result.is_err());
}
