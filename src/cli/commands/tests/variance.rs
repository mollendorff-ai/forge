//! Variance tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;
use super::common::create_test_yaml;
use tempfile::TempDir;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_variance_result_struct() {
    let vr = VarianceResult {
        name: "revenue".to_string(),
        budget: 1000.0,
        actual: 1200.0,
        variance: 200.0,
        variance_pct: 20.0,
        is_favorable: true,
        exceeds_threshold: true,
    };

    assert_eq!(vr.name, "revenue");
    assert_eq!(vr.budget, 1000.0);
    assert_eq!(vr.actual, 1200.0);
    assert_eq!(vr.variance, 200.0);
    assert_eq!(vr.variance_pct, 20.0);
    assert!(vr.is_favorable);
    assert!(vr.exceeds_threshold);
}

#[test]
fn test_variance_result_clone() {
    let vr = VarianceResult {
        name: "cost".to_string(),
        budget: 500.0,
        actual: 600.0,
        variance: 100.0,
        variance_pct: 20.0,
        is_favorable: false,
        exceeds_threshold: false,
    };
    let cloned = vr.clone();
    assert_eq!(cloned.name, vr.name);
    assert_eq!(cloned.budget, vr.budget);
}

#[test]
fn test_print_variance_table_empty() {
    let variances: Vec<VarianceResult> = vec![];
    // Just verify it doesn't panic
    print_variance_table(&variances, 10.0);
}

#[test]
fn test_print_variance_table_with_data() {
    let variances = vec![
        VarianceResult {
            name: "revenue".to_string(),
            budget: 1000.0,
            actual: 1100.0,
            variance: 100.0,
            variance_pct: 10.0,
            is_favorable: true,
            exceeds_threshold: false,
        },
        VarianceResult {
            name: "expense".to_string(),
            budget: 500.0,
            actual: 600.0,
            variance: 100.0,
            variance_pct: 20.0,
            is_favorable: false,
            exceeds_threshold: true,
        },
    ];
    // Just verify it doesn't panic
    print_variance_table(&variances, 15.0);
}

#[test]
fn test_export_variance_to_yaml() {
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("variance.yaml");

    let variances = vec![VarianceResult {
        name: "revenue".to_string(),
        budget: 1000.0,
        actual: 1100.0,
        variance: 100.0,
        variance_pct: 10.0,
        is_favorable: true,
        exceeds_threshold: false,
    }];

    let result = export_variance_to_yaml(&output_path, &variances, 5.0);
    assert!(result.is_ok());
    assert!(output_path.exists());

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("revenue"));
    assert!(content.contains("1000"));
    assert!(content.contains("1100"));
}

#[test]
fn test_export_variance_to_excel() {
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("variance.xlsx");

    let variances = vec![VarianceResult {
        name: "costs".to_string(),
        budget: 500.0,
        actual: 600.0,
        variance: 100.0,
        variance_pct: 20.0,
        is_favorable: false,
        exceeds_threshold: true,
    }];

    let result = export_variance_to_excel(&output_path, &variances, 10.0);
    assert!(result.is_ok());
    assert!(output_path.exists());
}

#[test]
fn test_variance_basic() {
    let dir = TempDir::new().unwrap();
    let budget = create_test_yaml(
        &dir,
        "budget.yaml",
        r#"_forge_version: "1.0.0"
summary:
  revenue:
    value: 1000
    formula: null
  expense:
    value: 500
    formula: null
"#,
    );
    let actual = create_test_yaml(
        &dir,
        "actual.yaml",
        r#"_forge_version: "1.0.0"
summary:
  revenue:
    value: 1100
    formula: null
  expense:
    value: 550
    formula: null
"#,
    );

    let result = variance(budget, actual, 10.0, None, false);
    assert!(result.is_ok());
}

#[test]
fn test_variance_verbose() {
    let dir = TempDir::new().unwrap();
    let budget = create_test_yaml(
        &dir,
        "budget_v.yaml",
        r#"_forge_version: "1.0.0"
summary:
  sales:
    value: 500
    formula: null
"#,
    );
    let actual = create_test_yaml(
        &dir,
        "actual_v.yaml",
        r#"_forge_version: "1.0.0"
summary:
  sales:
    value: 600
    formula: null
"#,
    );

    let result = variance(budget, actual, 5.0, None, true);
    assert!(result.is_ok());
}

#[test]
fn test_variance_output_xlsx() {
    let dir = TempDir::new().unwrap();
    let budget = create_test_yaml(
        &dir,
        "budget_xlsx.yaml",
        r#"_forge_version: "1.0.0"
summary:
  profit:
    value: 200
    formula: null
"#,
    );
    let actual = create_test_yaml(
        &dir,
        "actual_xlsx.yaml",
        r#"_forge_version: "1.0.0"
summary:
  profit:
    value: 250
    formula: null
"#,
    );
    let output = dir.path().join("variance_report.xlsx");

    let result = variance(budget, actual, 10.0, Some(output.clone()), false);
    assert!(result.is_ok());
    assert!(output.exists());
}

#[test]
fn test_variance_output_yaml() {
    let dir = TempDir::new().unwrap();
    let budget = create_test_yaml(
        &dir,
        "budget_yaml.yaml",
        r#"_forge_version: "1.0.0"
summary:
  cost:
    value: 100
    formula: null
"#,
    );
    let actual = create_test_yaml(
        &dir,
        "actual_yaml.yaml",
        r#"_forge_version: "1.0.0"
summary:
  cost:
    value: 120
    formula: null
"#,
    );
    let output = dir.path().join("variance_report.yaml");

    let result = variance(budget, actual, 5.0, Some(output.clone()), false);
    assert!(result.is_ok());
    assert!(output.exists());
}

#[test]
fn test_variance_unsupported_format() {
    let dir = TempDir::new().unwrap();
    let budget = create_test_yaml(
        &dir,
        "budget_bad.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
"#,
    );
    let actual = create_test_yaml(
        &dir,
        "actual_bad.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 15
    formula: null
"#,
    );
    let output = dir.path().join("variance.txt"); // Unsupported

    let result = variance(budget, actual, 10.0, Some(output), false);
    assert!(result.is_err());
}

#[test]
fn test_variance_file_not_found() {
    let dir = TempDir::new().unwrap();
    let actual = create_test_yaml(
        &dir,
        "actual_only.yaml",
        r#"_forge_version: "1.0.0"
summary:
  x:
    value: 10
    formula: null
"#,
    );

    let result = variance(
        PathBuf::from("/nonexistent.yaml"),
        actual,
        10.0,
        None,
        false,
    );
    assert!(result.is_err());
}

#[test]
fn test_variance_unfavorable() {
    let dir = TempDir::new().unwrap();
    let budget = create_test_yaml(
        &dir,
        "budget_unfav.yaml",
        r#"_forge_version: "1.0.0"
revenue:
  value: 1000
  formula: null
"#,
    );
    let actual = create_test_yaml(
        &dir,
        "actual_unfav.yaml",
        r#"_forge_version: "1.0.0"
revenue:
  value: 800
  formula: null
"#,
    );

    // Revenue below budget = unfavorable
    let result = variance(budget, actual, 5.0, None, false);
    assert!(result.is_ok());
}

#[test]
fn test_variance_zero_budget() {
    let dir = TempDir::new().unwrap();
    let budget = create_test_yaml(
        &dir,
        "budget_zero.yaml",
        r#"_forge_version: "1.0.0"
amount:
  value: 0
  formula: null
"#,
    );
    let actual = create_test_yaml(
        &dir,
        "actual_zero.yaml",
        r#"_forge_version: "1.0.0"
amount:
  value: 100
  formula: null
"#,
    );

    // Zero budget should not cause division by zero
    let result = variance(budget, actual, 10.0, None, false);
    assert!(result.is_ok());
}

#[test]
fn test_variance_xlsx_unfavorable() {
    let dir = TempDir::new().unwrap();
    let budget = create_test_yaml(
        &dir,
        "budget_xlsx_unfav.yaml",
        r#"_forge_version: "1.0.0"
revenue:
  value: 1000
  formula: null
cost:
  value: 500
  formula: null
"#,
    );
    let actual = create_test_yaml(
        &dir,
        "actual_xlsx_unfav.yaml",
        r#"_forge_version: "1.0.0"
revenue:
  value: 900
  formula: null
cost:
  value: 600
  formula: null
"#,
    );
    let output = dir.path().join("variance_unfav.xlsx");

    // Revenue below budget (unfavorable), cost above budget (unfavorable)
    let result = variance(budget, actual, 5.0, Some(output.clone()), false);
    assert!(result.is_ok());
    assert!(output.exists());
}
