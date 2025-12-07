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
fn e2e_v4_mixed_format_backward_compatible() {
    // Create a test file with mixed v1.0 and v4.0 formats
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("mixed_format.yaml");

    let mixed_content = r#"
_forge_version: "4.0.0"
# Mixed v1.0 and v4.0 formats in same file
sales:
  # v1.0 simple format
  month: ["Jan", "Feb", "Mar"]
  # v4.0 rich format
  revenue:
    value: [100, 200, 300]
    unit: "CAD"
    notes: "Monthly revenue"
  # v1.0 formula
  expenses: [50, 100, 150]
  profit: "=revenue - expenses"
"#;

    fs::write(&yaml_file, mixed_content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Mixed format should calculate, stdout: {stdout}, stderr: {stderr}"
    );

    // Verify profit was calculated
    assert!(
        stdout.contains("profit") && stdout.contains("3 rows"),
        "Should calculate profit column, got: {stdout}"
    );
}

#[test]
fn e2e_v4_scalar_with_full_metadata() {
    // Test scalar with all v4.0 metadata fields
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("v4_scalar_metadata.yaml");

    let content = r#"
_forge_version: "4.0.0"
metrics:
  total_revenue:
    value: 100000
    formula: null
    unit: "CAD"
    notes: "Annual revenue target"
    source: "budget_2025.yaml"
    validation_status: "VALIDATED"
    last_updated: "2025-11-26"
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
        "v4 scalar with full metadata should validate, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_v4_enterprise_model_500_formulas() {
    // Test that large enterprise model (500+ formula evaluations) calculates correctly
    let yaml_file = test_data_path("v4_enterprise_500_formulas.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Enterprise model should calculate successfully, stdout: {stdout}, stderr: {stderr}"
    );

    // Verify all tables were processed
    assert!(
        stdout.contains("revenue_monthly"),
        "Should have revenue_monthly table"
    );
    assert!(
        stdout.contains("costs_monthly"),
        "Should have costs_monthly table"
    );
    assert!(
        stdout.contains("pl_monthly"),
        "Should have pl_monthly table"
    );
    assert!(
        stdout.contains("cashflow_monthly"),
        "Should have cashflow_monthly table"
    );
    assert!(
        stdout.contains("metrics_monthly"),
        "Should have metrics_monthly table"
    );
    assert!(
        stdout.contains("quarterly_summary"),
        "Should have quarterly_summary table"
    );
    assert!(
        stdout.contains("annual_summary"),
        "Should have annual_summary table"
    );

    // Verify scalars were calculated
    assert!(
        stdout.contains("summary.total_mrr_2025"),
        "Should calculate total MRR"
    );
    assert!(
        stdout.contains("summary.final_arr"),
        "Should calculate final ARR"
    );
    assert!(
        stdout.contains("summary.final_customers"),
        "Should calculate final customers"
    );

    // Verify 24 rows in monthly tables
    assert!(
        stdout.contains("24 rows"),
        "Monthly tables should have 24 rows"
    );
}

#[test]
fn e2e_auto_upgrade_v1_to_v5() {
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("v1_model.yaml");

    // Create a v1.0.0 file
    let content = r#"_forge_version: "1.0.0"
revenue:
  value: 1000
  formula: null
costs:
  value: 600
  formula: null
profit:
  value: null
  formula: "=revenue - costs"
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    // Run calculate (not dry-run) - should auto-upgrade
    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "Calculate should succeed with auto-upgrade"
    );

    // Verify file was upgraded to 5.0.0
    let updated_content = fs::read_to_string(&yaml_file).unwrap();
    assert!(
        updated_content.contains("5.0.0"),
        "File should be upgraded to v5.0.0, got: {updated_content}"
    );
}
