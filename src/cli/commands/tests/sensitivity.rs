//! Sensitivity tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;
use super::common::create_test_yaml;
use tempfile::TempDir;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_sensitivity_one_variable() {
    let dir = TempDir::new().unwrap();
    // Use top-level scalars for sensitivity analysis
    let yaml = create_test_yaml(
        &dir,
        "sensitivity.yaml",
        r#"_forge_version: "1.0.0"
rate:
  value: 0.05
  formula: null
principal:
  value: 1000
  formula: null
interest:
  value: null
  formula: "=principal * rate"
"#,
    );

    let result = sensitivity(
        yaml,
        "rate".to_string(),
        "0.01,0.10,0.02".to_string(),
        None,
        None,
        "interest".to_string(),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn test_sensitivity_two_variables() {
    let dir = TempDir::new().unwrap();
    // Use top-level scalars
    let yaml = create_test_yaml(
        &dir,
        "sensitivity_2var.yaml",
        r#"_forge_version: "1.0.0"
rate:
  value: 0.05
  formula: null
years:
  value: 5
  formula: null
result:
  value: null
  formula: "=rate * years"
"#,
    );

    let result = sensitivity(
        yaml,
        "rate".to_string(),
        "0.01,0.05,0.02".to_string(),
        Some("years".to_string()),
        Some("1,5,2".to_string()),
        "result".to_string(),
        true,
    );
    assert!(result.is_ok());
}

#[test]
fn test_sensitivity_variable_not_found() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "sensitivity_notfound.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
"#,
    );

    let result = sensitivity(
        yaml,
        "nonexistent".to_string(),
        "1,10,1".to_string(),
        None,
        None,
        "x".to_string(),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn test_sensitivity_invalid_range() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "sensitivity_badrange.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
  y:
    value: null
    formula: "=x * 2"
"#,
    );

    let result = sensitivity(
        yaml,
        "x".to_string(),
        "invalid".to_string(),
        None,
        None,
        "y".to_string(),
        false,
    );
    assert!(result.is_err());
}

#[test]
fn test_sensitivity_second_var_not_found() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "sensitivity_2nd_notfound.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
  result:
    value: null
    formula: "=x * 2"
"#,
    );

    let result = sensitivity(
        yaml,
        "x".to_string(),
        "1,10,1".to_string(),
        Some("missing".to_string()),
        Some("1,5,1".to_string()),
        "result".to_string(),
        false,
    );
    assert!(result.is_err());
}
