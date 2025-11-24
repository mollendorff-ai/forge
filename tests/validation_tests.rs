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
    let yaml_content = r#"
platform:
  take_rate:
    value: 0.10
    formula: null

test:
  gross_margin:
    value: 0.9
    formula: "=1 - take_rate"
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), yaml_content).unwrap();

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(temp_file.path())
        .output()
        .expect("Failed to execute mouvify-forge");

    assert!(
        output.status.success(),
        "Validation should pass with correct values"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("All formulas are valid"));
    assert!(stdout.contains("All values match their formulas"));
}

#[test]
fn test_validation_fails_with_stale_values() {
    let yaml_content = r#"
platform:
  take_rate:
    value: 0.10
    formula: null

test:
  gross_margin:
    value: 0.5
    formula: "=1 - take_rate"
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), yaml_content).unwrap();

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(temp_file.path())
        .output()
        .expect("Failed to execute mouvify-forge");

    assert!(
        !output.status.success(),
        "Validation should fail with wrong values"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Found 1 value mismatches"));
    assert!(stdout.contains("test.gross_margin"));
    assert!(stdout.contains("Current:  0.5"));
    assert!(stdout.contains("Expected: 0.9"));
}

#[test]
fn test_calculate_updates_stale_values() {
    let yaml_content = r#"
platform:
  take_rate:
    value: 0.10
    formula: null

test:
  gross_margin:
    value: 0.5
    formula: "=1 - take_rate"
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), yaml_content).unwrap();

    // Run calculate
    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(temp_file.path())
        .output()
        .expect("Failed to execute mouvify-forge calculate");

    assert!(output.status.success(), "Calculate should succeed");

    // Verify the value was updated
    let updated_content = fs::read_to_string(temp_file.path()).unwrap();
    assert!(updated_content.contains("0.9") || updated_content.contains("0.90"));

    // Now validation should pass
    let validate_output = Command::new(forge_binary())
        .arg("validate")
        .arg(temp_file.path())
        .output()
        .expect("Failed to execute mouvify-forge validate");

    assert!(
        validate_output.status.success(),
        "Validation should pass after calculate"
    );
}

#[test]
fn test_validation_with_multiple_mismatches() {
    let yaml_content = r#"
base:
  a:
    value: 10
    formula: null
  b:
    value: 20
    formula: null

calculated:
  sum:
    value: 99
    formula: "=a + b"
  product:
    value: 999
    formula: "=a * b"
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), yaml_content).unwrap();

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(temp_file.path())
        .output()
        .expect("Failed to execute mouvify-forge");

    assert!(!output.status.success(), "Validation should fail");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Found 2 value mismatches"));
    assert!(stdout.contains("calculated.sum"));
    assert!(stdout.contains("calculated.product"));
    assert!(stdout.contains("Expected: 30")); // sum should be 30
    assert!(stdout.contains("Expected: 200")); // product should be 200
}

#[test]
fn test_dry_run_does_not_modify_file() {
    let yaml_content = r#"
platform:
  take_rate:
    value: 0.10
    formula: null

test:
  gross_margin:
    value: 0.5
    formula: "=1 - take_rate"
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
        .expect("Failed to execute mouvify-forge");

    assert!(output.status.success(), "Dry run should succeed");

    // Verify file was NOT modified
    let after_content = fs::read_to_string(temp_file.path()).unwrap();
    assert_eq!(
        original_content, after_content,
        "Dry run should not modify file"
    );
}
