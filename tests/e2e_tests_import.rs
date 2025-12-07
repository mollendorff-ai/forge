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
fn e2e_import_excel_to_yaml() {
    // First, create an Excel file by exporting
    let yaml_file = test_data_path("export_basic.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("for_import.xlsx");

    // Export to create Excel file
    let export_output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    assert!(
        export_output.status.success(),
        "Export should succeed before import test"
    );

    // Now test import
    let imported_yaml = temp_dir.path().join("imported.yaml");

    let import_output = Command::new(forge_binary())
        .arg("import")
        .arg(&excel_file)
        .arg(&imported_yaml)
        .output()
        .expect("Failed to execute import");

    assert!(
        import_output.status.success(),
        "Import should succeed, stderr: {}",
        String::from_utf8_lossy(&import_output.stderr)
    );

    // Verify YAML file was created
    assert!(
        imported_yaml.exists(),
        "Imported YAML file should be created"
    );

    // Verify YAML file has content
    let imported_content = fs::read_to_string(&imported_yaml).unwrap();
    assert!(
        !imported_content.is_empty(),
        "Imported YAML should not be empty"
    );

    // Verify it contains expected table name
    assert!(
        imported_content.contains("financial_summary"),
        "Should contain the table name"
    );
}

#[test]
fn e2e_import_nonexistent_excel_fails_gracefully() {
    let excel_file = test_data_path("this_file_does_not_exist.xlsx");
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("output.yaml");

    let output = Command::new(forge_binary())
        .arg("import")
        .arg(&excel_file)
        .arg(&yaml_file)
        .output()
        .expect("Failed to execute import");

    assert!(
        !output.status.success(),
        "Import should fail for nonexistent file"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("No such file")
            || combined.contains("not found")
            || combined.contains("Failed"),
        "Should report file not found error, got: {combined}"
    );
}

#[test]
fn e2e_import_with_split_files() {
    // First create Excel with multiple sheets by exporting multi-doc YAML
    let yaml_file = test_data_path("test_multi_document.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("for_split_import.xlsx");

    let export_output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    assert!(
        export_output.status.success(),
        "Export should succeed before split import test"
    );

    // Now test import with --split-files
    // For --split-files mode, the output is a DIRECTORY where files will be created
    let output_dir = temp_dir.path().join("split_output");

    let import_output = Command::new(forge_binary())
        .arg("import")
        .arg(&excel_file)
        .arg(&output_dir) // Pass directory, not file
        .arg("--split-files")
        .output()
        .expect("Failed to execute import with --split-files");

    let stdout = String::from_utf8_lossy(&import_output.stdout);
    let stderr = String::from_utf8_lossy(&import_output.stderr);

    assert!(
        import_output.status.success(),
        "Import with --split-files should succeed, stdout: {stdout}, stderr: {stderr}"
    );

    // Should create separate YAML files per sheet
    let yaml_files: Vec<_> = fs::read_dir(&output_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "yaml")
                .unwrap_or(false)
        })
        .collect();

    assert!(
        yaml_files.len() >= 2,
        "Should create multiple YAML files, found {}",
        yaml_files.len()
    );
}

#[test]
fn e2e_import_with_multi_doc() {
    // First create Excel with multiple sheets
    let yaml_file = test_data_path("test_multi_document.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("for_multi_import.xlsx");

    let export_output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    assert!(
        export_output.status.success(),
        "Export should succeed before multi-doc import test"
    );

    // Now test import with --multi-doc
    let imported_yaml = temp_dir.path().join("multi_doc_imported.yaml");

    let import_output = Command::new(forge_binary())
        .arg("import")
        .arg(&excel_file)
        .arg(&imported_yaml)
        .arg("--multi-doc")
        .output()
        .expect("Failed to execute import with --multi-doc");

    let stdout = String::from_utf8_lossy(&import_output.stdout);
    let stderr = String::from_utf8_lossy(&import_output.stderr);

    assert!(
        import_output.status.success(),
        "Import with --multi-doc should succeed, stdout: {stdout}, stderr: {stderr}"
    );

    // Verify YAML file was created
    assert!(imported_yaml.exists(), "Imported YAML should exist");

    // Verify it's a multi-document YAML (contains ---)
    let content = fs::read_to_string(&imported_yaml).expect("Failed to read imported YAML");
    assert!(
        content.contains("---"),
        "Should be multi-document YAML with --- separators, got: {}",
        &content[..content.len().min(500)]
    );
}

#[test]
fn e2e_import_default_single_file() {
    // Default import should create single file (no flags)
    let yaml_file = test_data_path("export_basic.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("for_default_import.xlsx");

    let export_output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    assert!(export_output.status.success());

    // Import without flags
    let imported_yaml = temp_dir.path().join("default_imported.yaml");

    let import_output = Command::new(forge_binary())
        .arg("import")
        .arg(&excel_file)
        .arg(&imported_yaml)
        .output()
        .expect("Failed to execute default import");

    let stdout = String::from_utf8_lossy(&import_output.stdout);
    let stderr = String::from_utf8_lossy(&import_output.stderr);

    assert!(
        import_output.status.success(),
        "Default import should succeed, stdout: {stdout}, stderr: {stderr}"
    );

    // Should create single YAML file
    assert!(imported_yaml.exists(), "Should create single YAML file");

    // Should not be multi-doc (no --- at start after potential header)
    let content = fs::read_to_string(&imported_yaml).expect("Failed to read");
    let trimmed = content.trim_start();
    let is_multi_doc = trimmed.starts_with("---") && trimmed[3..].contains("\n---");
    assert!(
        !is_multi_doc,
        "Default import should not create multi-doc YAML"
    );
}
