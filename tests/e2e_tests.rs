use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn forge_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("release");
    path.push("mouvify-forge");

    if !path.exists() {
        path.pop();
        path.pop();
        path.push("debug");
        path.push("mouvify-forge");
    }

    path
}

fn test_data_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("test-data");
    path.push(filename);
    path
}

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
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        combined.contains("Yaml") || combined.contains("EOF") || combined.contains("scanning"),
        "Should report YAML parsing error, got: {}",
        combined
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
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        combined.contains("Eval") || combined.contains("unknown variable") || combined.contains("UNDEFINED_VARIABLE"),
        "Should report variable not found error, got: {}",
        combined
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

    assert!(
        stdout.contains("Circular dependency") || stderr.contains("Circular dependency"),
        "Should detect circular dependency"
    );
}

#[test]
fn e2e_stale_values_detected() {
    let file = test_data_path("test_stale.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    assert!(!output.status.success(), "Stale values should fail validation");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should report mismatches
    assert!(stdout.contains("Found 2 value mismatches"));
    assert!(stdout.contains("test.gross_margin"));
    assert!(stdout.contains("unit_economics.ratio"));

    // Should show current vs expected
    assert!(stdout.contains("Current:"));
    assert!(stdout.contains("Expected:"));
    assert!(stdout.contains("Diff:"));

    // Should suggest fix
    assert!(stdout.contains("Run 'mouvify-forge calculate' to update values"));
}

#[test]
fn e2e_valid_updated_yaml_passes() {
    let file = test_data_path("test_valid_updated.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "Valid YAML should pass");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("All formulas are valid"));
    assert!(stdout.contains("All values match their formulas"));
}

#[test]
fn e2e_calculate_updates_stale_file() {
    // Copy stale file to temp location
    let original = test_data_path("test_stale.yaml");
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_file = temp_dir.path().join("test_stale_copy.yaml");

    fs::copy(&original, &temp_file).unwrap();

    // Verify it starts stale
    let validate_before = Command::new(forge_binary())
        .arg("validate")
        .arg(&temp_file)
        .output()
        .unwrap();

    assert!(!validate_before.status.success(), "Should start as stale");

    // Run calculate
    let calculate = Command::new(forge_binary())
        .arg("calculate")
        .arg(&temp_file)
        .output()
        .unwrap();

    assert!(calculate.status.success(), "Calculate should succeed");

    // Verify it's now valid
    let validate_after = Command::new(forge_binary())
        .arg("validate")
        .arg(&temp_file)
        .output()
        .unwrap();

    assert!(validate_after.status.success(), "Should be valid after calculate");

    let stdout = String::from_utf8_lossy(&validate_after.stdout);
    assert!(stdout.contains("All values match their formulas"));
}

#[test]
fn e2e_verbose_output_shows_formulas() {
    let file = test_data_path("test_valid_updated.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&file)
        .arg("--dry-run")
        .arg("--verbose")
        .output()
        .expect("Failed to execute");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show parsing info
    assert!(stdout.contains("Parsing YAML file"));
    assert!(stdout.contains("Found"));
    assert!(stdout.contains("variables with formulas"));

    // Should show formulas
    assert!(stdout.contains("test.gross_margin"));
    assert!(stdout.contains("=1 - take_rate"));

    // Should show calculation
    assert!(stdout.contains("Calculating formulas in dependency order"));
    assert!(stdout.contains("Calculation Results"));
}

#[test]
fn e2e_platform_test_file_validates() {
    let file = test_data_path("test_platform.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "test_platform.yaml should be valid");
}

#[test]
fn e2e_financial_test_file_validates() {
    let file = test_data_path("test_financial.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "test_financial.yaml should be valid");
}

#[test]
fn e2e_underscore_test_file_validates() {
    let file = test_data_path("test_underscore.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "test_underscore.yaml should be valid");
}

#[test]
fn e2e_basic_test_file_validates() {
    let file = test_data_path("test.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "test.yaml should be valid");
}

// ========== Cross-File Reference Tests ==========

#[test]
fn e2e_includes_main_validates() {
    let file = test_data_path("includes_main.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "includes_main.yaml should be valid");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("All formulas are valid"));
    assert!(stdout.contains("All values match their formulas"));
}

#[test]
fn e2e_includes_complex_validates() {
    let file = test_data_path("includes_complex.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "includes_complex.yaml should be valid");
}

#[test]
fn e2e_includes_calculate_with_verbose() {
    let file = test_data_path("includes_main.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&file)
        .arg("--dry-run")
        .arg("--verbose")
        .output()
        .expect("Failed to execute");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show cross-file references in formulas
    assert!(stdout.contains("@pricing.base_price"));
    assert!(stdout.contains("@costs.unit_cost"));

    // Should show calculation results
    assert!(stdout.contains("final_price"));
    assert!(stdout.contains("margin"));
}

#[test]
fn e2e_includes_missing_file_fails_gracefully() {
    let file = test_data_path("includes_missing_file.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    assert!(!output.status.success(), "Missing included file should fail");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        combined.contains("Failed to read included file") ||
        combined.contains("this_file_does_not_exist.yaml") ||
        combined.contains("No such file"),
        "Should report missing file error, got: {}",
        combined
    );
}

#[test]
fn e2e_includes_invalid_alias_fails() {
    let file = test_data_path("includes_invalid_alias.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    assert!(!output.status.success(), "Invalid alias reference should fail");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Should fail during evaluation (variable not found)
    assert!(
        combined.contains("Eval") ||
        combined.contains("invalid_alias") ||
        combined.contains("UNDEFINED_VARIABLE"),
        "Should report invalid alias error, got: {}",
        combined
    );
}

#[test]
fn e2e_includes_revenue_with_internal_formulas() {
    let file = test_data_path("includes_revenue.yaml");

    let output = Command::new(forge_binary())
        .arg("validate")
        .arg(&file)
        .output()
        .expect("Failed to execute");

    // This file has formulas that reference variables within the same file
    assert!(output.status.success(), "includes_revenue.yaml should be valid");
}

#[test]
fn e2e_includes_calculate_updates_correctly() {
    // Copy main file AND included files to temp location
    let temp_dir = tempfile::tempdir().unwrap();

    // Copy main file
    let original_main = test_data_path("includes_main.yaml");
    let temp_main = temp_dir.path().join("includes_main.yaml");
    fs::copy(&original_main, &temp_main).unwrap();

    // Copy included files
    let pricing_orig = test_data_path("includes_pricing.yaml");
    let pricing_temp = temp_dir.path().join("includes_pricing.yaml");
    fs::copy(&pricing_orig, &pricing_temp).unwrap();

    let costs_orig = test_data_path("includes_costs.yaml");
    let costs_temp = temp_dir.path().join("includes_costs.yaml");
    fs::copy(&costs_orig, &costs_temp).unwrap();

    // Run calculate
    let calculate = Command::new(forge_binary())
        .arg("calculate")
        .arg(&temp_main)
        .output()
        .unwrap();

    assert!(calculate.status.success(), "Calculate should succeed");

    let stdout = String::from_utf8_lossy(&calculate.stdout);
    assert!(stdout.contains("File updated successfully"));

    // Verify it's valid after calculation
    let validate = Command::new(forge_binary())
        .arg("validate")
        .arg(&temp_main)
        .output()
        .unwrap();

    assert!(validate.status.success(), "Should be valid after calculate");
}

#[test]
fn e2e_includes_circular_dependency_detected() {
    let file = test_data_path("includes_with_circular_dep.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    assert!(!output.status.success(), "Circular dependency should be detected");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        combined.contains("Circular dependency"),
        "Should detect circular dependency, got: {}",
        combined
    );
}

#[test]
fn e2e_includes_mixed_local_and_included_refs() {
    let file = test_data_path("includes_mixed_refs.yaml");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "Mixed refs should work");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should calculate correctly with mix of local and included vars
    assert!(stdout.contains("total_revenue"));
    assert!(stdout.contains("total_cost"));
    assert!(stdout.contains("net_profit"));
}

#[test]
fn e2e_includes_mixed_refs_validates_after_calculate() {
    // Copy main file AND included files to temp location
    let temp_dir = tempfile::tempdir().unwrap();

    // Copy main file
    let original_main = test_data_path("includes_mixed_refs.yaml");
    let temp_main = temp_dir.path().join("includes_mixed_refs.yaml");
    fs::copy(&original_main, &temp_main).unwrap();

    // Copy included files
    let pricing_orig = test_data_path("includes_pricing.yaml");
    let pricing_temp = temp_dir.path().join("includes_pricing.yaml");
    fs::copy(&pricing_orig, &pricing_temp).unwrap();

    let costs_orig = test_data_path("includes_costs.yaml");
    let costs_temp = temp_dir.path().join("includes_costs.yaml");
    fs::copy(&costs_orig, &costs_temp).unwrap();

    // Run calculate
    let calculate = Command::new(forge_binary())
        .arg("calculate")
        .arg(&temp_main)
        .output()
        .unwrap();

    assert!(calculate.status.success(), "Calculate should succeed");

    // Verify values are correct by validation
    let validate = Command::new(forge_binary())
        .arg("validate")
        .arg(&temp_main)
        .output()
        .unwrap();

    assert!(validate.status.success(), "Should be valid after calculate");

    let stdout = String::from_utf8_lossy(&validate.stdout);
    assert!(stdout.contains("All formulas are valid"));
    assert!(stdout.contains("All values match their formulas"));
}
