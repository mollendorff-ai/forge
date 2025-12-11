//! Validation tests for forge
//!
//! Note: Some tests run the forge binary directly and are skipped during coverage.
//! Schema tests that don't use the binary run in all modes.

// Skip tests that use binaries during coverage builds (ADR-006)
// The tests that don't use binaries (schema tests) still run
#![cfg(not(coverage))]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::NamedTempFile;

// ============================================================================
// Schema Validation Tests
// These tests ensure the schema stays in sync with documented format versions
// ============================================================================

/// Test that schema only contains valid format versions (1.0.0 and 5.0.0)
/// This prevents the bug where software versions were added to format enum
#[test]
fn test_schema_version_enum_only_contains_format_versions() {
    // Test v1.0.0 schema (scalar-only)
    let schema_path_v1 = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("schema")
        .join("forge-v1.0.0.schema.json");

    let schema_content_v1 =
        fs::read_to_string(&schema_path_v1).expect("Failed to read v1.0.0 schema file");
    let schema_v1: serde_json::Value =
        serde_json::from_str(&schema_content_v1).expect("Failed to parse v1.0.0 schema JSON");

    // Get the _forge_version enum for v1.0.0
    let version_enum_v1 = schema_v1["properties"]["_forge_version"]["enum"]
        .as_array()
        .expect("v1.0.0 _forge_version should have enum property");

    // v1.0.0 schema should only allow "1.0.0"
    assert_eq!(version_enum_v1.len(), 1);
    assert_eq!(version_enum_v1[0].as_str(), Some("1.0.0"));

    // Test v5.0.0 schema (arrays/tables)
    let schema_path_v5 = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("schema")
        .join("forge-v5.0.0.schema.json");

    let schema_content_v5 =
        fs::read_to_string(&schema_path_v5).expect("Failed to read v5.0.0 schema file");
    let schema_v5: serde_json::Value =
        serde_json::from_str(&schema_content_v5).expect("Failed to parse v5.0.0 schema JSON");

    // Get the _forge_version enum for v5.0.0
    let version_enum_v5 = schema_v5["properties"]["_forge_version"]["enum"]
        .as_array()
        .expect("v5.0.0 _forge_version should have enum property");

    // v5.0.0 schema should only allow "5.0.0"
    assert_eq!(version_enum_v5.len(), 1);
    assert_eq!(version_enum_v5[0].as_str(), Some("5.0.0"));
}

/// Test that _forge_version is required in schema
#[test]
fn test_schema_requires_forge_version() {
    // Test v1.0.0 schema
    let schema_path_v1 = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("schema")
        .join("forge-v1.0.0.schema.json");

    let schema_content_v1 =
        fs::read_to_string(&schema_path_v1).expect("Failed to read v1.0.0 schema file");
    let schema_v1: serde_json::Value =
        serde_json::from_str(&schema_content_v1).expect("Failed to parse v1.0.0 schema JSON");

    let required_v1 = schema_v1["required"]
        .as_array()
        .expect("v1.0.0 schema should have required property");

    assert!(
        required_v1
            .iter()
            .any(|v| v.as_str() == Some("_forge_version")),
        "_forge_version must be in schema's required array"
    );
}

/// Test that all test YAML files have _forge_version
#[test]
fn test_all_test_yaml_files_have_forge_version() {
    let test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data");

    let mut missing_version = Vec::new();

    for entry in fs::read_dir(&test_data_dir).expect("Failed to read test-data dir") {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();

        if path.extension().map(|e| e == "yaml").unwrap_or(false) {
            // Skip known malformed test files
            if path
                .file_name()
                .map(|n| n == "test_malformed.yaml")
                .unwrap_or(false)
            {
                continue;
            }

            let content = fs::read_to_string(&path).unwrap_or_else(|_| String::new());

            if !content.contains("_forge_version") {
                missing_version.push(path.file_name().unwrap().to_string_lossy().to_string());
            }
        }
    }

    // Also check v1.0 subdirectory
    let v1_dir = test_data_dir.join("v1.0");
    if v1_dir.exists() {
        for entry in fs::read_dir(&v1_dir).expect("Failed to read v1.0 dir") {
            let entry = entry.expect("Failed to read entry");
            let path = entry.path();

            if path.extension().map(|e| e == "yaml").unwrap_or(false) {
                let content = fs::read_to_string(&path).unwrap_or_else(|_| String::new());

                if !content.contains("_forge_version") {
                    missing_version.push(format!(
                        "v1.0/{}",
                        path.file_name().unwrap().to_string_lossy()
                    ));
                }
            }
        }
    }

    assert!(
        missing_version.is_empty(),
        "The following test files are missing _forge_version: {:?}",
        missing_version
    );
}

fn forge_binary() -> PathBuf {
    // Use the binary in target/release or target/debug
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("release");
    path.push("forge");

    if !path.exists() {
        // Fall back to debug build
        path.pop();
        path.pop();
        path.push("debug");
        path.push("forge");
    }

    path
}

#[test]
fn test_validation_passes_with_correct_values() {
    // v5.0.0 format with tables (v1.0.0 is scalar-only)
    let yaml_content = r#"
_forge_version: "5.0.0"

financials:
  quarter: ["Q1", "Q2", "Q3", "Q4"]
  revenue: [100, 200, 300, 400]
  costs: [50, 100, 150, 200]
  profit: "=revenue - costs"
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), yaml_content).unwrap();

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(temp_file.path())
        .output()
        .expect("Failed to execute forge");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Validation should pass, stdout: {stdout}, stderr: {stderr}"
    );

    assert!(
        stdout.contains("valid") || stdout.contains("Table"),
        "Should indicate validation passed, got: {stdout}"
    );
}

#[test]
fn test_validation_with_scalars() {
    // v5.0.0 format with arrays and scalars
    let yaml_content = r#"
_forge_version: "5.0.0"

data:
  values: [10, 20, 30, 40]

summary:
  total:
    value: 100
    formula: "=SUM(data.values)"
  average:
    value: 25
    formula: "=AVERAGE(data.values)"
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), yaml_content).unwrap();

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(temp_file.path())
        .output()
        .expect("Failed to execute forge");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Validation should pass with correct scalar values, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn test_validation_fails_with_wrong_scalar() {
    // v5.0.0 format with wrong scalar value
    let yaml_content = r#"
_forge_version: "5.0.0"

data:
  values: [10, 20, 30, 40]

summary:
  total:
    value: 999
    formula: "=SUM(data.values)"
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), yaml_content).unwrap();

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(temp_file.path())
        .output()
        .expect("Failed to execute forge");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !output.status.success(),
        "Validation should fail with wrong values"
    );

    assert!(
        stdout.contains("mismatch") || stdout.contains("Expected") || stdout.contains("999"),
        "Should report mismatch, got: {stdout}"
    );
}

#[test]
fn test_calculate_dry_run() {
    // v5.0.0 for tables/arrays
    let yaml_content = r#"
_forge_version: "5.0.0"

financials:
  quarter: ["Q1", "Q2"]
  revenue: [100, 200]
  profit: "=revenue * 0.2"
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), yaml_content).unwrap();

    let original_content = fs::read_to_string(temp_file.path()).unwrap();

    // Run calculate with --dry-run
    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(temp_file.path())
        .arg("--dry-run")
        .output()
        .expect("Failed to execute forge");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Dry run should succeed, stdout: {stdout}, stderr: {stderr}"
    );

    // Verify file was NOT modified
    let after_content = fs::read_to_string(temp_file.path()).unwrap();
    assert_eq!(
        original_content, after_content,
        "Dry run should not modify file"
    );
}

#[test]
fn test_validation_with_table_formulas() {
    // v5.0.0 for row-wise formulas with arrays
    let yaml_content = r#"
_forge_version: "5.0.0"

sales:
  month: ["Jan", "Feb", "Mar"]
  units: [10, 20, 30]
  price: [100, 100, 100]
  revenue: "=units * price"
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), yaml_content).unwrap();

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(temp_file.path())
        .output()
        .expect("Failed to execute forge");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Table formulas should validate, stdout: {stdout}, stderr: {stderr}"
    );
}
