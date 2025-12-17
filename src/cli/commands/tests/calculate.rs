//! Calculate tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;
use super::common::create_test_yaml;
use tempfile::TempDir;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_calculate_with_override_existing_scalar() {
    let mut model = crate::types::ParsedModel::new();
    model.scalars.insert(
        "rate".to_string(),
        crate::types::Variable::new("rate".to_string(), Some(0.05), None),
    );
    model.scalars.insert(
        "result".to_string(),
        crate::types::Variable::new("result".to_string(), None, Some("=rate * 100".to_string())),
    );

    let output = calculate_with_override(&model, "rate", 0.10, "result").unwrap();
    assert!((output - 10.0).abs() < 0.0001);
}

#[test]
fn test_calculate_with_override_new_scalar() {
    let mut model = crate::types::ParsedModel::new();
    model.scalars.insert(
        "result".to_string(),
        crate::types::Variable::new("result".to_string(), None, Some("=rate * 100".to_string())),
    );

    let output = calculate_with_override(&model, "rate", 0.15, "result").unwrap();
    assert!((output - 15.0).abs() < 0.0001);
}

#[test]
fn test_calculate_with_override_output_not_found() {
    let mut model = crate::types::ParsedModel::new();
    model.scalars.insert(
        "rate".to_string(),
        crate::types::Variable::new("rate".to_string(), Some(0.05), None),
    );

    let result = calculate_with_override(&model, "rate", 0.10, "nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_calculate_success() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "calc.yaml",
        r#"_forge_version: "5.0.0"
summary:
  price:
    value: 100
    formula: null
  total:
    value: null
    formula: "=price * 2"
"#,
    );

    let result = calculate(yaml, true, false, None);
    if let Err(e) = &result {
        eprintln!("calculate error: {e:?}");
    }
    assert!(result.is_ok());
}

#[test]
fn test_calculate_with_verbose() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "calc_verbose.yaml",
        r#"_forge_version: "5.0.0"
summary:
  x:
    value: 10
    formula: null
  y:
    value: null
    formula: "=x * 3"
"#,
    );

    let result = calculate(yaml, true, true, None);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_with_scenario() {
    let dir = TempDir::new().unwrap();
    // Use top-level scalars (not nested in summary) so scenario overrides work
    let yaml = create_test_yaml(
        &dir,
        "calc_scenario.yaml",
        r#"_forge_version: "5.0.0"
rate:
  value: 0.05
  formula: null
result:
  value: null
  formula: "=rate * 100"
scenarios:
  high_rate:
    rate: 0.15
"#,
    );

    let result = calculate(yaml, true, true, Some("high_rate".to_string()));
    assert!(result.is_ok());
}

#[test]
fn test_calculate_invalid_scenario() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "calc_bad_scenario.yaml",
        r#"_forge_version: "5.0.0"
summary:
  x:
    value: 10
    formula: null
"#,
    );

    let result = calculate(yaml, true, false, Some("nonexistent".to_string()));
    assert!(result.is_err());
}

#[test]
fn test_calculate_file_not_found() {
    let result = calculate(PathBuf::from("/nonexistent/file.yaml"), true, false, None);
    assert!(result.is_err());
}

#[test]
fn test_calculate_invalid_yaml() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(&dir, "invalid.yaml", "not: valid: yaml: content:");

    let result = calculate(yaml, true, false, None);
    assert!(result.is_err());
}

#[test]
fn test_calculate_with_tables() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "calc_tables.yaml",
        r#"_forge_version: "5.0.0"
sales:
  month: [1, 2, 3]
  revenue: [100, 200, 300]
"#,
    );

    let result = calculate(yaml, true, true, None);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_dry_run_no_write() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "dry_run.yaml",
        r#"_forge_version: "5.0.0"
summary:
  a:
    value: 5
    formula: null
"#,
    );

    // Store original content
    let original = std::fs::read_to_string(&yaml).unwrap();

    let result = calculate(yaml.clone(), true, false, None);
    assert!(result.is_ok());

    // Verify file unchanged in dry run
    let after = std::fs::read_to_string(&yaml).unwrap();
    assert_eq!(original, after);
}

#[test]
fn test_calculate_writes_results() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "write_test.yaml",
        r#"_forge_version: "5.0.0"
summary:
  x:
    value: 10
    formula: null
  y:
    value: null
    formula: "=x * 2"
"#,
    );

    // Not dry run - should write results
    let result = calculate(yaml.clone(), false, false, None);
    assert!(result.is_ok());

    // Backup should be created
    let backup = yaml.with_extension("yaml.bak");
    assert!(backup.exists());
}

#[test]
fn test_calculate_multi_doc() {
    let dir = TempDir::new().unwrap();
    let yaml_path = dir.path().join("multi_doc.yaml");
    std::fs::write(
        &yaml_path,
        r#"---
_forge_version: "5.0.0"
_name: "doc1"
x:
  value: 10
  formula: null
---
_forge_version: "5.0.0"
_name: "doc2"
y:
  value: 20
  formula: null
"#,
    )
    .unwrap();

    // Multi-doc with dry_run=false triggers write-back not supported message
    let result = calculate(yaml_path, false, true, None);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_with_unit_warnings() {
    let dir = TempDir::new().unwrap();
    let yaml = create_test_yaml(
        &dir,
        "unit_warn.yaml",
        r#"_forge_version: "5.0.0"
price:
  value: 100
  formula: null
  unit: "USD"
quantity:
  value: 5
  formula: null
  unit: "units"
result:
  value: null
  formula: "=price + quantity"
  unit: "USD"
"#,
    );

    // Adding USD + units should trigger unit warning
    let result = calculate(yaml, true, true, None);
    assert!(result.is_ok());
}
