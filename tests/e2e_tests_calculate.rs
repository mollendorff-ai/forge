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
fn e2e_calculate_dry_run() {
    let file = test_data_path("test_valid_updated.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Calculate dry-run should succeed, stdout: {stdout}, stderr: {stderr}"
    );

    assert!(
        stdout.contains("Dry run") || stdout.contains("DRY RUN"),
        "Should indicate dry run mode, got: {stdout}"
    );
}

#[test]
fn e2e_v4_schema_rejected_needs_manual_upgrade() {
    // v4.0 is no longer supported - must use `forge upgrade` manually
    let file = test_data_path("v4_enterprise_model.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // v4.0.0 should be rejected (auto-upgrade killed in v7.2.6)
    assert!(
        !output.status.success(),
        "v4 calculate should fail (use forge upgrade), stderr: {stderr}"
    );

    // Verify error message mentions unsupported version
    assert!(
        stderr.contains("Unsupported _forge_version"),
        "Should reject v4.0.0, got: {stderr}"
    );
}

#[cfg(not(feature = "demo"))]
#[test]
fn e2e_v4_unique_functions_calculate() {
    // Test that UNIQUE and COUNTUNIQUE functions calculate correctly
    let yaml_file = test_data_path("v4_unique_functions.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .output()
        .expect("Failed to execute calculate");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "UNIQUE functions should calculate, stdout: {stdout}, stderr: {stderr}"
    );

    // Verify correct calculations:
    // - 3 unique products (Apple, Banana, Orange)
    // - 4 unique regions (North, South, East, West)
    // - 1 unique category (Fruit)
    // - 7 = 3 + 4
    assert!(
        stdout.contains("total_unique_products = 3"),
        "Should have 3 unique products, got: {stdout}"
    );
    assert!(
        stdout.contains("total_unique_regions = 4"),
        "Should have 4 unique regions, got: {stdout}"
    );
    assert!(
        stdout.contains("unique_categories = 1"),
        "Should have 1 unique category, got: {stdout}"
    );
    assert!(
        stdout.contains("unique_products_plus_regions = 7"),
        "Should have 7 (3+4) combined, got: {stdout}"
    );
}

#[test]
fn e2e_multi_document_yaml_calculates() {
    let file = test_data_path("test_multi_document.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Multi-document calculate should succeed, stdout: {stdout}, stderr: {stderr}"
    );

    // Should show tables from multiple documents
    assert!(
        stdout.contains("sales") || stdout.contains("Sales"),
        "Should have sales table, got: {stdout}"
    );
}

#[test]
fn e2e_v5_model_calculates() {
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("v5_calc.yaml");

    let content = r#"
_forge_version: "5.0.0"

inputs:
  tax_rate:
    value: 0.25
    formula: null

data:
  revenue: [100, 200, 300, 400]
  expenses: [50, 100, 150, 200]
  profit: "=revenue - expenses"
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

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
        "v5.0.0 model calculate should succeed, stdout: {stdout}, stderr: {stderr}"
    );

    assert!(
        stdout.contains("profit"),
        "Should show profit calculation, got: {stdout}"
    );
}

#[test]
fn e2e_upgrade_command_dry_run() {
    let yaml_file = test_data_path("budget.yaml");

    let output = Command::new(forge_binary())
        .arg("upgrade")
        .arg(&yaml_file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute upgrade");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Upgrade --dry-run should succeed, stdout: {stdout}, stderr: {stderr}"
    );
}
