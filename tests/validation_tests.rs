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

/// Test that schema only contains valid format versions (1.0.0 and 4.0.0)
/// This prevents the bug where software versions were added to format enum
#[test]
fn test_schema_version_enum_only_contains_format_versions() {
    let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("schema")
        .join("forge-v1.0.schema.json");

    let schema_content = fs::read_to_string(&schema_path).expect("Failed to read schema file");

    let schema: serde_json::Value =
        serde_json::from_str(&schema_content).expect("Failed to parse schema JSON");

    // Get the _forge_version enum
    let version_enum = schema["properties"]["_forge_version"]["enum"]
        .as_array()
        .expect("_forge_version should have enum property");

    // Only format versions should be in the enum
    let valid_format_versions = vec!["1.0.0", "4.0.0", "5.0.0"];

    for version in version_enum {
        let v = version.as_str().expect("Version should be string");
        assert!(
            valid_format_versions.contains(&v),
            "Schema contains invalid format version '{}'. Only {:?} are valid format versions.",
            v,
            valid_format_versions
        );
    }

    // Ensure both format versions are present
    for valid in &valid_format_versions {
        assert!(
            version_enum.iter().any(|v| v.as_str() == Some(*valid)),
            "Schema missing required format version '{}'",
            valid
        );
    }
}

/// Test that _forge_version is required in schema
#[test]
fn test_schema_requires_forge_version() {
    let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("schema")
        .join("forge-v1.0.schema.json");

    let schema_content = fs::read_to_string(&schema_path).expect("Failed to read schema file");

    let schema: serde_json::Value =
        serde_json::from_str(&schema_content).expect("Failed to parse schema JSON");

    let required = schema["required"]
        .as_array()
        .expect("Schema should have required property");

    assert!(
        required
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
    // v1.0.0 format with tables
    let yaml_content = r#"
_forge_version: "1.0.0"

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
    // v1.0.0 format with scalars
    let yaml_content = r#"
_forge_version: "1.0.0"

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
    // v1.0.0 format with wrong scalar value
    let yaml_content = r#"
_forge_version: "1.0.0"

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
    let yaml_content = r#"
_forge_version: "1.0.0"

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
    // Test that row-wise formulas are calculated correctly
    let yaml_content = r#"
_forge_version: "1.0.0"

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
