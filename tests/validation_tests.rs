use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::NamedTempFile;

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
