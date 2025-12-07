//! Goal_Seek tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;
use super::common::create_test_yaml;
use tempfile::TempDir;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_goal_seek_basic() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "goal_seek.yaml",
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

    // Find rate where interest = 100
    let result = goal_seek(
        yaml,
        "interest".to_string(),
        100.0,
        "rate".to_string(),
        Some(0.01),
        Some(0.5),
        0.0001,
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn test_goal_seek_verbose() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "goal_seek_verbose.yaml",
        r#"_forge_version: "1.0.0"
x:
  value: 5
  formula: null
y:
  value: null
  formula: "=x * 10"
"#,
    );

    let result = goal_seek(
        yaml,
        "y".to_string(),
        100.0,
        "x".to_string(),
        Some(1.0),
        Some(20.0),
        0.001,
        true,
    );
    assert!(result.is_ok());
}

#[test]
fn test_goal_seek_variable_not_found() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "goal_seek_notfound.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
"#,
    );

    let result = goal_seek(
        yaml,
        "x".to_string(),
        50.0,
        "nonexistent".to_string(),
        None,
        None,
        0.001,
        false,
    );
    assert!(result.is_err());
}

#[test]
fn test_goal_seek_no_solution() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "goal_seek_nosol.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
  y:
    value: null
    formula: "=x * x"
"#,
    );

    // Try to find x where x^2 = -100 (impossible in reals)
    let result = goal_seek(
        yaml,
        "y".to_string(),
        -100.0,
        "x".to_string(),
        Some(0.1),
        Some(10.0),
        0.001,
        false,
    );
    assert!(result.is_err());
}

#[test]
fn test_goal_seek_default_bounds() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "goal_seek_defaults.yaml",
        r#"_forge_version: "1.0.0"
factor:
  value: 2.0
  formula: null
result:
  value: null
  formula: "=factor * 50"
"#,
    );

    // Don't specify min/max - use defaults
    let result = goal_seek(
        yaml,
        "result".to_string(),
        100.0,
        "factor".to_string(),
        None,
        None,
        0.0001,
        false,
    );
    assert!(result.is_ok());
}
