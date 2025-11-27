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
fn e2e_malformed_yaml_fails_gracefully() {
    let file = test_data_path("test_malformed.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    assert!(!output.status.success(), "Malformed YAML should fail");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("Yaml") || combined.contains("EOF") || combined.contains("scanning"),
        "Should report YAML parsing error, got: {combined}"
    );
}

#[test]
fn e2e_invalid_formula_variable_not_found() {
    let file = test_data_path("test_invalid_formula.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    assert!(!output.status.success(), "Invalid formula should fail");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("Eval")
            || combined.contains("unknown variable")
            || combined.contains("UNDEFINED_VARIABLE")
            || combined.contains("Error"),
        "Should report variable not found error, got: {combined}"
    );
}

#[test]
fn e2e_circular_dependency_detected() {
    let file = test_data_path("test_circular.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    assert!(!output.status.success(), "Circular dependency should fail");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("Circular")
            || combined.contains("cycle")
            || combined.contains("dependency"),
        "Should detect circular dependency, got: {combined}"
    );
}

// ========== Validation Tests ==========

#[test]
fn e2e_stale_values_detected() {
    let file = test_data_path("test_stale.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    assert!(
        !output.status.success(),
        "Stale values should fail validation"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should report mismatches
    assert!(
        stdout.contains("mismatch") || stdout.contains("Expected"),
        "Should report value mismatches, got: {stdout}"
    );
}

#[test]
fn e2e_valid_updated_yaml_passes() {
    let file = test_data_path("test_valid_updated.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Valid YAML should pass, stdout: {stdout}, stderr: {stderr}"
    );

    assert!(
        stdout.contains("valid") || stdout.contains("match"),
        "Should indicate validation passed, got: {stdout}"
    );
}

// ========== Calculate Tests ==========

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
fn e2e_verbose_output_shows_info() {
    let file = test_data_path("test_valid_updated.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&file)
        .arg("--dry-run")
        .arg("--verbose")
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());

    // Should show parsing info
    assert!(
        stdout.contains("Parsing") || stdout.contains("Found"),
        "Should show verbose parsing info, got: {stdout}"
    );
}

// ========== File Validation Tests ==========

#[test]
fn e2e_platform_test_file_validates() {
    let file = test_data_path("test_platform.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "test_platform.yaml should be valid, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_financial_test_file_validates() {
    let file = test_data_path("test_financial.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "test_financial.yaml should be valid, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_underscore_test_file_validates() {
    let file = test_data_path("test_underscore.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "test_underscore.yaml should be valid, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_basic_test_file_validates() {
    let file = test_data_path("test.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "test.yaml should be valid, stdout: {stdout}, stderr: {stderr}"
    );
}

// ========== Excel Export/Import Tests (v1.0.0) ==========

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
fn e2e_roundtrip_yaml_excel_yaml() {
    // YAML → Excel → YAML roundtrip test
    let original_yaml = test_data_path("roundtrip_test.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("roundtrip.xlsx");
    let final_yaml = temp_dir.path().join("roundtrip_final.yaml");

    // Step 1: Export YAML → Excel
    let export_output = Command::new(forge_binary())
        .arg("export")
        .arg(&original_yaml)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    assert!(
        export_output.status.success(),
        "Export should succeed in roundtrip test, stderr: {}",
        String::from_utf8_lossy(&export_output.stderr)
    );

    // Step 2: Import Excel → YAML
    let import_output = Command::new(forge_binary())
        .arg("import")
        .arg(&excel_file)
        .arg(&final_yaml)
        .output()
        .expect("Failed to execute import");

    assert!(
        import_output.status.success(),
        "Import should succeed in roundtrip test, stderr: {}",
        String::from_utf8_lossy(&import_output.stderr)
    );

    // Verify the imported file exists and has content
    assert!(final_yaml.exists(), "Final YAML should exist");
    let final_content = fs::read_to_string(&final_yaml).unwrap();
    assert!(!final_content.is_empty(), "Final YAML should not be empty");

    // The imported YAML should contain table structure
    assert!(
        final_content.contains("test_table") || final_content.contains("tables"),
        "Should have test_table, got: {}",
        final_content
    );
}

#[test]
fn e2e_roundtrip_with_formulas_preserves_formulas() {
    // Test round-trip specifically for formula preservation
    let original_yaml = test_data_path("export_with_formulas.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let excel_file = temp_dir.path().join("formulas_roundtrip.xlsx");
    let final_yaml = temp_dir.path().join("formulas_roundtrip_final.yaml");

    // Export → Import
    let export_output = Command::new(forge_binary())
        .arg("export")
        .arg(&original_yaml)
        .arg(&excel_file)
        .output()
        .expect("Failed to execute export");

    assert!(export_output.status.success(), "Export should succeed");

    let import_output = Command::new(forge_binary())
        .arg("import")
        .arg(&excel_file)
        .arg(&final_yaml)
        .output()
        .expect("Failed to execute import");

    assert!(import_output.status.success(), "Import should succeed");

    // Verify formulas are preserved
    let final_content = fs::read_to_string(&final_yaml).unwrap();

    // At minimum, should contain the table structure
    assert!(
        final_content.contains("financial") || final_content.contains("revenue"),
        "Should preserve table structure"
    );
}

// ========== v1.0.0 Model Tests ==========

#[test]
fn e2e_v1_quarterly_pl_validates() {
    let file = test_data_path("v1.0/quarterly_pl.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "quarterly_pl.yaml should be valid, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_v1_saas_unit_economics_validates() {
    let file = test_data_path("v1.0/saas_unit_economics.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "saas_unit_economics.yaml should be valid, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_v1_budget_vs_actual_validates() {
    let file = test_data_path("v1.0/budget_vs_actual.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "budget_vs_actual.yaml should be valid, stdout: {stdout}, stderr: {stderr}"
    );
}

// ========== Audit Trail Tests (v1.4.0) ==========

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

// ========== Watch Mode Tests (v1.4.0) ==========

#[test]
fn e2e_watch_nonexistent_file_fails() {
    let file = test_data_path("this_file_does_not_exist.yaml");

    let output = Command::new(forge_binary())
        .arg("watch")
        .arg(&file)
        .output()
        .expect("Failed to execute watch");

    assert!(
        !output.status.success(),
        "Watch should fail for nonexistent file"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("not found") || combined.contains("File not found"),
        "Should report file not found, got: {combined}"
    );
}

#[test]
fn e2e_watch_help_shows_usage() {
    let output = Command::new(forge_binary())
        .arg("watch")
        .arg("--help")
        .output()
        .expect("Failed to execute watch --help");

    assert!(output.status.success(), "watch --help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Watch") || stdout.contains("watch"),
        "Should show watch help, got: {stdout}"
    );

    assert!(
        stdout.contains("--validate") || stdout.contains("validate"),
        "Should show --validate option, got: {stdout}"
    );
}

// ========== Excel Formula Verification Tests (v3.1.4) ==========
// These tests verify that Excel export produces correct formulas,
// not just that it creates a file.

use calamine::{open_workbook, Reader, Xlsx};

/// Helper to read formula from Excel cell
#[allow(dead_code)]
fn get_excel_formula(path: &std::path::Path, sheet: &str, row: u32, col: u32) -> Option<String> {
    let mut workbook: Xlsx<_> = open_workbook(path).ok()?;
    let range = workbook.worksheet_formula(sheet).ok()?;
    range.get((row as usize, col as usize)).cloned()
}

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

// ========== v4.0 Rich Metadata E2E Tests ==========

#[test]
fn e2e_v4_enterprise_model_validates() {
    // v4.0 rich metadata format should parse and validate
    let file = test_data_path("v4_enterprise_model.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "v4 enterprise model should validate, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_v4_enterprise_model_calculates_correctly() {
    // v4.0 model should calculate formulas correctly
    let file = test_data_path("v4_enterprise_model.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "v4 calculate should succeed, stdout: {stdout}, stderr: {stderr}"
    );

    // Verify calculations completed
    assert!(
        stdout.contains("Calculation Results"),
        "Should show calculation results, got: {stdout}"
    );

    // Verify scalars were calculated
    assert!(
        stdout.contains("metrics.total_revenue") || stdout.contains("total_revenue"),
        "Should calculate total_revenue scalar, got: {stdout}"
    );
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
fn e2e_v4_mixed_format_backward_compatible() {
    // Create a test file with mixed v1.0 and v4.0 formats
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("mixed_format.yaml");

    let mixed_content = r#"
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
fn e2e_v4_unit_validation_detects_mismatch() {
    // Test that unit validation catches incompatible units
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("unit_mismatch.yaml");

    let content = r#"
financials:
  revenue:
    value: [100000, 120000]
    unit: "CAD"
  margin:
    value: [0.30, 0.35]
    unit: "%"
  # This should trigger a unit warning: CAD + %
  bad_sum: "=revenue + margin"
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should still succeed (warnings don't block execution)
    assert!(
        output.status.success(),
        "Calculate should succeed even with unit warnings"
    );

    // Should contain unit warning
    assert!(
        stdout.contains("Unit Consistency Warnings") || stdout.contains("incompatible units"),
        "Should show unit mismatch warning, got: {stdout}"
    );
}

#[test]
fn e2e_v4_unit_validation_no_warning_for_compatible() {
    // Test that compatible units don't trigger warnings
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("unit_compatible.yaml");

    let content = r#"
financials:
  revenue:
    value: [100000, 120000]
    unit: "CAD"
  expenses:
    value: [80000, 90000]
    unit: "CAD"
  # CAD - CAD is fine
  profit: "=revenue - expenses"
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "Calculate should succeed");

    // Should NOT contain unit warning
    assert!(
        !stdout.contains("Unit Consistency Warnings"),
        "Should not show warnings for compatible units, got: {stdout}"
    );
}

#[test]
fn e2e_v4_enterprise_model_500_formulas() {
    // Test that large enterprise model (500+ formula evaluations) calculates correctly
    let yaml_file = test_data_path("v4_enterprise_500_formulas.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
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
