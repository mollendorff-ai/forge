//! End-to-end tests for forge CLI
//!
//! # Coverage Exclusion (ADR-006)
//! These tests are skipped during coverage runs because the binaries are
//! stubbed to empty main() functions. Run without coverage for full testing.

// Skip all e2e tests during coverage builds (ADR-006)
// The binaries have stubbed main() functions that exit immediately
#![cfg(not(coverage))]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn forge_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("release");
    path.push("forge");

    if !path.exists() {
        path.pop();
        path.pop();
        path.push("debug");
        path.push("forge");
    }

    path
}

fn test_data_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("test-data");
    path.push(filename);
    path
}

// ========== Basic Validation Tests ==========

#[test]
fn e2e_invalid_formula_variable_not_found() {
    let file = test_data_path("test_invalid_formula.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    assert!(!output.status.success(), "Invalid formula should fail");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("Eval")
            || combined.contains("unknown variable")
            || combined.contains("UNDEFINED_VARIABLE")
            || combined.contains("Error"),
        "Should report variable not found error, got: {combined}"
    );
}

#[test]
fn e2e_valid_updated_yaml_passes() {
    let file = test_data_path("test_valid_updated.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Valid YAML should pass, stdout: {stdout}, stderr: {stderr}"
    );

    assert!(
        stdout.contains("valid") || stdout.contains("match"),
        "Should indicate validation passed, got: {stdout}"
    );
}

#[test]
fn e2e_platform_test_file_validates() {
    let file = test_data_path("test_platform.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "test_platform.yaml should be valid, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_financial_test_file_validates() {
    let file = test_data_path("test_financial.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "test_financial.yaml should be valid, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_underscore_test_file_validates() {
    let file = test_data_path("test_underscore.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "test_underscore.yaml should be valid, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_basic_test_file_validates() {
    let file = test_data_path("test.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "test.yaml should be valid, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_v1_quarterly_pl_validates() {
    let file = test_data_path("v1.0/quarterly_pl.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "quarterly_pl.yaml should be valid, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_v1_saas_unit_economics_validates() {
    let file = test_data_path("v1.0/saas_unit_economics.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "saas_unit_economics.yaml should be valid, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_v1_budget_vs_actual_validates() {
    let file = test_data_path("v1.0/budget_vs_actual.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "budget_vs_actual.yaml should be valid, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_v4_validate_rejected_needs_manual_upgrade() {
    // v4.0 is no longer supported - must use `forge upgrade` manually
    let file = test_data_path("v4_enterprise_model.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // v4.0.0 should be rejected
    assert!(
        !output.status.success(),
        "v4 validate should fail (use forge upgrade), stderr: {stderr}"
    );

    assert!(
        stderr.contains("Unsupported _forge_version"),
        "Should reject v4.0.0, got: {stderr}"
    );
}

#[test]
fn e2e_v5_unit_validation_detects_mismatch() {
    // Test that unit validation catches incompatible units
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("unit_mismatch.yaml");

    let content = r#"
_forge_version: "5.0.0"
financials:
  revenue:
    value: [100000, 120000]
    unit: "CAD"
  margin:
    value: [0.30, 0.35]
    unit: "%"
  # This should trigger a unit warning: CAD + %
  bad_sum: "=revenue + margin"
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should still succeed (warnings don't block execution)
    assert!(
        output.status.success(),
        "Calculate should succeed even with unit warnings"
    );

    // Should contain unit warning
    assert!(
        stdout.contains("Unit Consistency Warnings") || stdout.contains("incompatible units"),
        "Should show unit mismatch warning, got: {stdout}"
    );
}

#[test]
fn e2e_v5_unit_validation_no_warning_for_compatible() {
    // Test that compatible units don't trigger warnings
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("unit_compatible.yaml");

    let content = r#"
_forge_version: "5.0.0"
financials:
  revenue:
    value: [100000, 120000]
    unit: "CAD"
  expenses:
    value: [80000, 90000]
    unit: "CAD"
  # CAD - CAD is fine
  profit: "=revenue - expenses"
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "Calculate should succeed");

    // Should NOT contain unit warning
    assert!(
        !stdout.contains("Unit Consistency Warnings"),
        "Should not show warnings for compatible units, got: {stdout}"
    );
}

#[test]
fn e2e_multi_document_yaml_validates() {
    let file = test_data_path("test_multi_document.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Multi-document YAML should validate, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_model_with_includes_validates() {
    let file = test_data_path("v4_with_includes.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Model with includes should validate, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_v5_model_validates() {
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("v5_model.yaml");

    let content = r#"
_forge_version: "5.0.0"

inputs:
  tax_rate:
    value: 0.25
    formula: null
    unit: "%"
  discount_rate:
    value: 0.10
    formula: null

outputs:
  net_profit:
    value: null
    formula: "=SUM(data.revenue) * (1 - tax_rate)"

data:
  quarter: ["Q1", "Q2", "Q3", "Q4"]
  revenue: [100000, 120000, 150000, 180000]
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&yaml_file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "v5.0.0 model should validate, stdout: {stdout}, stderr: {stderr}"
    );
}
