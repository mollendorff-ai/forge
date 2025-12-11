//! End-to-end tests for schema version compatibility
//!
//! v7.2.6: Auto-upgrade killed. Only v1.0.0 and v5.0.0 supported.
//! v4.0.0 must use `forge upgrade` manually.

// Skip all e2e tests during coverage builds (ADR-006)
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

// ========== v4.0.0 Rejection Tests (v7.2.6 - auto-upgrade killed) ==========

#[test]
fn e2e_v4_mixed_format_rejected() {
    // v4.0.0 is no longer supported - must use forge upgrade
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("mixed_format.yaml");

    let mixed_content = r#"
_forge_version: "4.0.0"
sales:
  month: ["Jan", "Feb", "Mar"]
  revenue:
    value: [100, 200, 300]
    unit: "CAD"
  expenses: [50, 100, 150]
  profit: "=revenue - expenses"
"#;

    fs::write(&yaml_file, mixed_content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .output()
        .expect("Failed to execute");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // v4.0.0 should be rejected
    assert!(
        !output.status.success(),
        "v4.0.0 should be rejected, stderr: {stderr}"
    );
    assert!(
        stderr.contains("Unsupported _forge_version"),
        "Should mention unsupported version, got: {stderr}"
    );
}

#[test]
fn e2e_v4_scalar_metadata_rejected() {
    // v4.0.0 with metadata should be rejected
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
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&yaml_file)
        .output()
        .expect("Failed to execute");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "v4.0.0 should be rejected, stderr: {stderr}"
    );
}

#[test]
fn e2e_v4_enterprise_500_formulas_rejected() {
    // Large v4.0.0 model should be rejected
    let yaml_file = test_data_path("v4_enterprise_500_formulas.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "v4.0.0 should be rejected (use forge upgrade), stderr: {stderr}"
    );
    assert!(
        stderr.contains("Unsupported _forge_version"),
        "Should reject v4.0.0, got: {stderr}"
    );
}

// ========== v1.0.0 Native Support Tests ==========

#[test]
fn e2e_v1_stays_at_v1_no_auto_upgrade() {
    // v1.0.0 should work natively without upgrading (auto-upgrade killed in v7.2.6)
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("v1_model.yaml");

    // Create a v1.0.0 file with assumptions format (required for v1.0.0)
    let content = r#"_forge_version: "1.0.0"
assumptions:
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

    // Run calculate (not dry-run)
    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "v1.0.0 should calculate natively");

    // Verify file stays at v1.0.0 (no auto-upgrade)
    let updated_content = fs::read_to_string(&yaml_file).unwrap();
    assert!(
        updated_content.contains("1.0.0"),
        "File should remain at v1.0.0 (no auto-upgrade), got: {updated_content}"
    );
    assert!(
        !updated_content.contains("5.0.0"),
        "File should NOT be upgraded to v5.0.0"
    );
}

// ========== v5.0.0 Tests ==========

#[test]
fn e2e_v5_mixed_format_works() {
    // v5.0.0 with arrays should work
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("v5_mixed.yaml");

    let content = r#"
_forge_version: "5.0.0"
sales:
  month: ["Jan", "Feb", "Mar"]
  revenue:
    value: [100, 200, 300]
    unit: "CAD"
  expenses: [50, 100, 150]
  profit: "=revenue - expenses"
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "v5.0.0 should calculate, stdout: {stdout}, stderr: {stderr}"
    );

    assert!(
        stdout.contains("profit"),
        "Should calculate profit, got: {stdout}"
    );
}
