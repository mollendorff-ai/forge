//! End-to-end tests for forge CLI
//!
//! # Coverage Exclusion (ADR-006)
//! These tests are skipped during coverage runs because the binaries are
//! stubbed to empty main() functions. Run without coverage for full testing.

// Skip all e2e tests during coverage builds (ADR-006)
// The binaries have stubbed main() functions that exit immediately
#![cfg(not(coverage))]

use calamine::{open_workbook, Reader, Xlsx};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Helper to get all formulas from a sheet
fn get_sheet_formulas(path: &std::path::Path, sheet: &str) -> Vec<(usize, usize, String)> {
    let mut results = Vec::new();
    if let Ok(mut workbook) = open_workbook::<Xlsx<_>, _>(path) {
        if let Ok(range) = workbook.worksheet_formula(sheet) {
            for (row_idx, row) in range.rows().enumerate() {
                for (col_idx, cell) in row.iter().enumerate() {
                    if !cell.is_empty() {
                        results.push((row_idx, col_idx, cell.clone()));
                    }
                }
            }
        }
    }
    results
}

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
fn e2e_export_basic_yaml_to_excel() {
    let yaml_file = test_data_path("export_basic.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("export_basic.xlsx");

    let output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    assert!(
        output.status.success(),
        "Export should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify Excel file was created
    assert!(excel_file.exists(), "Excel file should be created");

    // Verify Excel file has non-zero size
    let metadata = fs::metadata(&excel_file).unwrap();
    assert!(metadata.len() > 0, "Excel file should not be empty");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Export Complete!") || stdout.contains("exported successfully"),
        "Should show success message, got: {}",
        stdout
    );
}

#[test]
fn e2e_export_with_formulas_translates_correctly() {
    let yaml_file = test_data_path("export_with_formulas.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("export_with_formulas.xlsx");

    let output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    assert!(
        output.status.success(),
        "Export with formulas should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify Excel file was created
    assert!(
        excel_file.exists(),
        "Excel file with formulas should be created"
    );

    // Verify file is valid Excel format (non-zero size)
    let metadata = fs::metadata(&excel_file).unwrap();
    assert!(metadata.len() > 0, "Excel file should not be empty");
}

#[test]
fn e2e_export_nonexistent_file_fails_gracefully() {
    let yaml_file = test_data_path("this_file_does_not_exist.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("output.xlsx");

    let output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    assert!(
        !output.status.success(),
        "Export should fail for nonexistent file"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("No such file")
            || combined.contains("not found")
            || combined.contains("Failed to read"),
        "Should report file not found error, got: {combined}"
    );
}

#[test]
fn e2e_export_malformed_yaml_fails_gracefully() {
    let yaml_file = test_data_path("test_malformed.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("output.xlsx");

    let output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    assert!(
        !output.status.success(),
        "Export should fail for malformed YAML"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("Yaml") || combined.contains("Parse") || combined.contains("scanning"),
        "Should report YAML parsing error, got: {combined}"
    );
}

#[test]
fn e2e_export_cross_table_refs_use_column_letters() {
    // This test would have caught the bug where we exported "table!revenue2"
    // instead of "table!A2"
    let yaml_file = test_data_path("export_cross_table.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("cross_table.xlsx");

    let output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    assert!(
        output.status.success(),
        "Export should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read the Excel file and verify formula syntax
    let formulas = get_sheet_formulas(&excel_file, "targets");

    // Should have formulas in the targets sheet
    assert!(
        !formulas.is_empty(),
        "Should have formulas in targets sheet"
    );

    // Verify formulas use column letters (A, B, C) not column names
    for (row, col, formula) in &formulas {
        // Formulas should NOT contain patterns like "sales!revenue" (column name)
        // They SHOULD contain patterns like "'sales'!A" (column letter)
        assert!(
            !formula.contains("!revenue")
                && !formula.contains("!cost")
                && !formula.contains("!profit"),
            "Formula at ({}, {}) should use column letters, not names. Got: {}",
            row,
            col,
            formula
        );

        // Cross-table refs should have quoted sheet names for LibreOffice compatibility
        if formula.contains("sales") {
            assert!(
                formula.contains("'sales'!"),
                "Cross-table reference should quote sheet name. Got: {}",
                formula
            );
        }
    }
}

#[test]
fn e2e_export_scalar_formulas_are_actual_formulas() {
    // This test would have caught the bug where scalar formulas were
    // exported as text strings instead of actual Excel formulas
    let yaml_file = test_data_path("export_cross_table.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("scalar_formulas.xlsx");

    let output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    assert!(
        output.status.success(),
        "Export should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read the Scalars sheet and verify formulas exist
    let formulas = get_sheet_formulas(&excel_file, "Scalars");

    // Should have formulas in the Scalars sheet (total_revenue, total_profit, etc.)
    assert!(
        !formulas.is_empty(),
        "Scalars sheet should have actual formulas, not just text values"
    );

    // Verify at least one SUM formula exists
    let has_sum = formulas.iter().any(|(_, _, f)| f.contains("SUM"));
    assert!(
        has_sum,
        "Should have SUM formulas in Scalars sheet. Found formulas: {:?}",
        formulas
    );
}

#[test]
fn e2e_export_aggregation_formulas_have_correct_range() {
    // Verify that SUM(table.column) translates to SUM('table'!A2:A4) not SUM('table'!A2)
    let yaml_file = test_data_path("export_cross_table.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("aggregation_formulas.xlsx");

    let output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    assert!(output.status.success());

    let formulas = get_sheet_formulas(&excel_file, "Scalars");

    // Find SUM formulas and verify they have ranges (colon notation)
    for (_, _, formula) in &formulas {
        if formula.contains("SUM") {
            assert!(
                formula.contains(":"),
                "SUM formula should have a range (A2:A4), not a single cell. Got: {}",
                formula
            );
        }
    }
}

#[test]
fn e2e_export_row_formulas_translate_correctly() {
    // Verify row formulas like "=revenue - cost" become "=A2-B2"
    let yaml_file = test_data_path("export_with_formulas.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("row_formulas.xlsx");

    let output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    assert!(output.status.success());

    let formulas = get_sheet_formulas(&excel_file, "pl_statement");

    // Should have row formulas
    assert!(!formulas.is_empty(), "Should have row formulas");

    // Formulas should use cell references like A2, B2, not variable names
    for (row, col, formula) in &formulas {
        // Should not contain raw variable names (they should be translated)
        assert!(
            !formula.contains("revenue") && !formula.contains("cogs"),
            "Formula at ({}, {}) should use cell refs, not variable names. Got: {}",
            row,
            col,
            formula
        );
    }
}

#[test]
fn e2e_export_sheet_names_are_quoted() {
    // LibreOffice requires quoted sheet names in cross-sheet references
    let yaml_file = test_data_path("export_cross_table.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("quoted_sheets.xlsx");

    let output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    assert!(output.status.success());

    // Check targets sheet for cross-table refs to sales
    let formulas = get_sheet_formulas(&excel_file, "targets");

    for (_, _, formula) in &formulas {
        // If it references another sheet, the sheet name should be quoted
        if formula.contains("!") && !formula.starts_with("=") {
            // This is a cross-sheet reference
            assert!(
                formula.contains("'"),
                "Cross-sheet reference should have quoted sheet name. Got: {}",
                formula
            );
        }
    }

    // Also check Scalars sheet
    let scalar_formulas = get_sheet_formulas(&excel_file, "Scalars");
    for (_, _, formula) in &scalar_formulas {
        if formula.contains("sales") || formula.contains("targets") {
            assert!(
                formula.contains("'sales'") || formula.contains("'targets'"),
                "Scalar formula should quote sheet names. Got: {}",
                formula
            );
        }
    }
}

#[test]
fn e2e_v4_enterprise_model_exports_to_excel() {
    // v4.0 model should export to Excel
    let yaml_file = test_data_path("v4_enterprise_model.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("v4_enterprise.xlsx");

    let output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "v4 export should succeed, stdout: {stdout}, stderr: {stderr}"
    );

    // Verify Excel file was created
    assert!(excel_file.exists(), "Excel file should be created");

    // Verify Excel file has non-zero size
    let metadata = fs::metadata(&excel_file).unwrap();
    assert!(
        metadata.len() > 1000,
        "Excel file should have substantial content"
    );
}

#[test]
fn e2e_v4_enterprise_model_export_to_excel() {
    // Test that enterprise model exports to Excel correctly
    let yaml_file = test_data_path("v4_enterprise_500_formulas.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("enterprise.xlsx");

    let output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Enterprise model export should succeed, stdout: {stdout}, stderr: {stderr}"
    );

    // Verify Excel file was created and has substantial size
    assert!(excel_file.exists(), "Excel file should be created");
    let metadata = fs::metadata(&excel_file).unwrap();
    assert!(
        metadata.len() > 10000,
        "Enterprise Excel file should be substantial (>10KB), got {} bytes",
        metadata.len()
    );
}

#[cfg(feature = "full")]
#[test]
fn e2e_v4_unique_functions_export() {
    // Test that UNIQUE functions export to Excel correctly
    let yaml_file = test_data_path("v4_unique_functions.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("unique_test.xlsx");

    let output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "UNIQUE model export should succeed, stdout: {stdout}, stderr: {stderr}"
    );

    // Verify Excel file was created
    assert!(excel_file.exists(), "Excel file should be created");
    let metadata = fs::metadata(&excel_file).unwrap();
    assert!(
        metadata.len() > 1000,
        "Excel file should have content (>1KB), got {} bytes",
        metadata.len()
    );
}

#[test]
fn e2e_multi_document_yaml_exports_to_excel() {
    let yaml_file = test_data_path("test_multi_document.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("multi_doc.xlsx");

    let output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Multi-doc export should succeed, stdout: {stdout}, stderr: {stderr}"
    );

    // Verify Excel file was created
    assert!(excel_file.exists(), "Excel file should be created");
    let metadata = fs::metadata(&excel_file).unwrap();
    assert!(
        metadata.len() > 1000,
        "Excel file should have content (>1KB), got {} bytes",
        metadata.len()
    );
}

#[test]
fn e2e_model_with_includes_exports_to_excel() {
    let yaml_file = test_data_path("v4_with_includes.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("with_includes.xlsx");

    let output = Command::new(forge_binary())
        .arg("export")
        .arg(&yaml_file)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Export with includes should succeed, stdout: {stdout}, stderr: {stderr}"
    );

    // Verify Excel file was created
    assert!(excel_file.exists(), "Excel file should be created");

    // Read workbook to verify included sheets exist
    let workbook: Xlsx<_> = open_workbook(&excel_file).expect("Failed to open workbook");
    let sheet_names = workbook.sheet_names().to_vec();

    println!("Sheets in exported workbook: {:?}", sheet_names);

    // Should have sheets for included content with namespace prefix
    assert!(
        sheet_names
            .iter()
            .any(|s| s.contains("sources") || s.contains("revenue")),
        "Should have namespaced sheets from includes, got: {:?}",
        sheet_names
    );
}
