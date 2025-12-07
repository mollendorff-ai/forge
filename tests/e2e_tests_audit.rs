//! End-to-end tests for forge CLI
//!
//! # Coverage Exclusion (ADR-006)
//! These tests are skipped during coverage runs because the binaries are
//! stubbed to empty main() functions. Run without coverage for full testing.

// Skip all e2e tests during coverage builds (ADR-006)
// The binaries have stubbed main() functions that exit immediately
#![cfg(not(coverage))]

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
fn e2e_audit_shows_variable_info() {
    let file = test_data_path("v1.0/quarterly_pl.yaml");

    // Variable names in the model are qualified with section name
    let output = Command::new(forge_binary())
        .arg("audit")
        .arg(&file)
        .arg("annual_2025.avg_gross_margin")
        .output()
        .expect("Failed to execute audit");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Audit should succeed, stdout: {stdout}, stderr: {stderr}"
    );

    // Should show variable information
    assert!(
        stdout.contains("Audit Trail") || stdout.contains("audit"),
        "Should show audit header, got: {stdout}"
    );

    assert!(
        stdout.contains("avg_gross_margin") || stdout.contains("Variable"),
        "Should show variable name, got: {stdout}"
    );
}

#[test]
fn e2e_audit_shows_dependency_tree() {
    let file = test_data_path("v1.0/quarterly_pl.yaml");

    let output = Command::new(forge_binary())
        .arg("audit")
        .arg(&file)
        .arg("annual_2025.avg_gross_margin")
        .output()
        .expect("Failed to execute audit");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show dependency tree
    assert!(
        stdout.contains("Dependency")
            || stdout.contains("Tree")
            || stdout.contains("total_gross_profit"),
        "Should show dependency tree with total_gross_profit, got: {stdout}"
    );
}

#[test]
fn e2e_audit_shows_calculation_result() {
    let file = test_data_path("v1.0/quarterly_pl.yaml");

    let output = Command::new(forge_binary())
        .arg("audit")
        .arg(&file)
        .arg("annual_2025.avg_gross_margin")
        .output()
        .expect("Failed to execute audit");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show calculation result
    assert!(
        stdout.contains("Calculation") || stdout.contains("Calculated"),
        "Should show calculation result, got: {stdout}"
    );

    // Should complete successfully
    assert!(
        stdout.contains("Audit complete") || stdout.contains("✅"),
        "Should show completion message, got: {stdout}"
    );
}

#[test]
fn e2e_audit_nonexistent_variable_fails() {
    let file = test_data_path("v1.0/quarterly_pl.yaml");

    let output = Command::new(forge_binary())
        .arg("audit")
        .arg(&file)
        .arg("this_variable_does_not_exist")
        .output()
        .expect("Failed to execute audit");

    assert!(
        !output.status.success(),
        "Audit should fail for nonexistent variable"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("not found") || combined.contains("Available"),
        "Should report variable not found, got: {combined}"
    );
}

#[test]
fn e2e_audit_nonexistent_file_fails() {
    let file = test_data_path("this_file_does_not_exist.yaml");

    let output = Command::new(forge_binary())
        .arg("audit")
        .arg(&file)
        .arg("some_variable")
        .output()
        .expect("Failed to execute audit");

    assert!(
        !output.status.success(),
        "Audit should fail for nonexistent file"
    );
}
