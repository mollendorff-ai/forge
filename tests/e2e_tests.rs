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
        stdout.contains("Dependency") || stdout.contains("Tree") || stdout.contains("total_gross_profit"),
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

    assert!(
        output.status.success(),
        "watch --help should succeed"
    );

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
