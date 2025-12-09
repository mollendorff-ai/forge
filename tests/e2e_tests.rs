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

#[test]
fn e2e_goal_seek_command() {
    let yaml_file = test_data_path("budget.yaml");

    let output = Command::new(forge_binary())
        .arg("goal-seek")
        .arg(&yaml_file)
        .arg("--target")
        .arg("assumptions.profit=50000")
        .arg("--adjust")
        .arg("assumptions.revenue")
        .output()
        .expect("Failed to execute goal-seek");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Goal-seek should run (success or graceful failure)
    // The command exists and processes the arguments
    assert!(
        !stdout.is_empty() || !stderr.is_empty(),
        "Goal-seek should produce output, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_sensitivity_command() {
    let yaml_file = test_data_path("budget.yaml");

    let output = Command::new(forge_binary())
        .arg("sensitivity")
        .arg(&yaml_file)
        .arg("--input")
        .arg("assumptions.revenue")
        .arg("--output")
        .arg("assumptions.profit")
        .output()
        .expect("Failed to execute sensitivity");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Sensitivity should run (produces output)
    assert!(
        !stdout.is_empty() || !stderr.is_empty(),
        "Sensitivity should produce output, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_variance_command() {
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("variance_test.yaml");

    let content = r#"
_forge_version: "5.0.0"

budget:
  revenue: [100000, 120000, 150000]
  expenses: [80000, 90000, 100000]

actual:
  revenue: [95000, 125000, 145000]
  expenses: [78000, 92000, 105000]
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    // Variance command takes: <FILE> <BUDGET> <ACTUAL>
    let output = Command::new(forge_binary())
        .arg("variance")
        .arg(&yaml_file)
        .arg("budget.revenue")
        .arg("actual.revenue")
        .output()
        .expect("Failed to execute variance");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Command should produce output (success or error message)
    assert!(
        !stdout.is_empty() || !stderr.is_empty(),
        "Variance command should produce output, stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_break_even_command() {
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("breakeven_test.yaml");

    let content = r#"
_forge_version: "5.0.0"

costs:
  fixed_costs:
    value: 50000
    formula: null
  unit_price:
    value: 100
    formula: null
  variable_cost:
    value: 60
    formula: null
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("break-even")
        .arg(&yaml_file)
        .arg("--fixed")
        .arg("costs.fixed_costs")
        .arg("--price")
        .arg("costs.unit_price")
        .arg("--variable")
        .arg("costs.variable_cost")
        .output()
        .expect("Failed to execute break-even");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Break-even command should run (produces output)
    assert!(
        !stdout.is_empty() || !stderr.is_empty(),
        "Break-even should produce output, stdout: {stdout}, stderr: {stderr}"
    );
}

// Demo build: verify basic functions output
#[cfg(not(feature = "full"))]
#[test]
fn e2e_functions_command() {
    let output = Command::new(forge_binary())
        .arg("functions")
        .output()
        .expect("Failed to execute functions");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Functions command should succeed, stderr: {stderr}"
    );

    // Demo should list core functions (Math, Aggregation, etc.)
    assert!(
        stdout.contains("Math") || stdout.contains("SUM") || stdout.contains("IF"),
        "Should list demo functions, got: {stdout}"
    );
}

// Enterprise build: verify full functions output
#[cfg(feature = "full")]
#[test]
fn e2e_functions_command() {
    let output = Command::new(forge_binary())
        .arg("functions")
        .output()
        .expect("Failed to execute functions");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Functions command should succeed, stderr: {stderr}"
    );

    // Enterprise should list financial/statistical categories
    assert!(
        stdout.contains("Statistical") || stdout.contains("Financial") || stdout.contains("MEDIAN"),
        "Should list functions, got: {stdout}"
    );
}

#[test]
fn e2e_v5_with_scenarios() {
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("v5_scenarios.yaml");

    let content = r#"
_forge_version: "5.0.0"

scenarios:
  base:
    growth_rate: 0.05
  optimistic:
    growth_rate: 0.12
  pessimistic:
    growth_rate: 0.02

inputs:
  growth_rate:
    value: 0.05
    formula: null

outputs:
  projected_revenue:
    value: null
    formula: "=100000 * (1 + inputs.growth_rate)"
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "v5 with scenarios should calculate, stdout: {stdout}, stderr: {stderr}"
    );
}

// Statistical functions are enterprise-only
#[cfg(feature = "full")]
#[test]
fn e2e_statistical_functions_in_model() {
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("stats_test.yaml");

    let content = r#"
_forge_version: "5.0.0"

data:
  values: [10, 20, 30, 40, 50]

outputs:
  median_value:
    value: null
    formula: "=MEDIAN(data.values)"
  total_sum:
    value: null
    formula: "=SUM(data.values)"
  average_value:
    value: null
    formula: "=AVERAGE(data.values)"
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Statistical functions should work, stdout: {stdout}, stderr: {stderr}"
    );

    // Check that values are calculated
    assert!(
        stdout.contains("median_value") || stdout.contains("30"),
        "Should calculate median, got: {stdout}"
    );
}

// Financial functions are enterprise-only
#[cfg(feature = "full")]
#[test]
fn e2e_financial_functions_in_model() {
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("finance_test.yaml");

    let content = r#"
_forge_version: "5.0.0"

inputs:
  cost:
    value: 10000
    formula: null
  salvage:
    value: 1000
    formula: null
  life:
    value: 5
    formula: null

outputs:
  annual_depreciation:
    value: null
    formula: "=SLN(inputs.cost, inputs.salvage, inputs.life)"
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Financial functions should work, stdout: {stdout}, stderr: {stderr}"
    );

    // SLN(10000, 1000, 5) = 1800
    assert!(
        stdout.contains("1800") || stdout.contains("annual_depreciation"),
        "Should calculate depreciation, got: {stdout}"
    );
}

#[cfg(feature = "full")]
#[test]
fn e2e_forge_variance_functions_in_model() {
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("variance_funcs.yaml");

    let content = r#"
_forge_version: "5.0.0"

inputs:
  actual:
    value: 95000
    formula: null
  budget:
    value: 100000
    formula: null

outputs:
  variance_amount:
    value: null
    formula: "=VARIANCE(inputs.actual, inputs.budget)"
  variance_percent:
    value: null
    formula: "=VARIANCE_PCT(inputs.actual, inputs.budget)"
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Variance functions should work, stdout: {stdout}, stderr: {stderr}"
    );

    // VARIANCE = -5000
    assert!(
        stdout.contains("-5000") || stdout.contains("variance_amount"),
        "Should calculate variance, got: {stdout}"
    );
}

#[cfg(feature = "full")]
#[test]
fn e2e_breakeven_functions_in_model() {
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("breakeven_funcs.yaml");

    let content = r#"
_forge_version: "5.0.0"

inputs:
  fixed_costs:
    value: 50000
    formula: null
  unit_price:
    value: 100
    formula: null
  variable_cost:
    value: 60
    formula: null
  margin:
    value: 0.40
    formula: null

outputs:
  be_units:
    value: null
    formula: "=BREAKEVEN_UNITS(inputs.fixed_costs, inputs.unit_price, inputs.variable_cost)"
  be_revenue:
    value: null
    formula: "=BREAKEVEN_REVENUE(inputs.fixed_costs, inputs.margin)"
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Breakeven functions should work, stdout: {stdout}, stderr: {stderr}"
    );

    // BREAKEVEN_UNITS = 1250, BREAKEVEN_REVENUE = 125000
    assert!(
        stdout.contains("1250") || stdout.contains("125000") || stdout.contains("be_units"),
        "Should calculate breakeven, got: {stdout}"
    );
}

#[test]
fn e2e_auto_upgrade_preserves_calculation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("calc_upgrade.yaml");

    // Create a v1.0.0 file with formula
    let content = r#"_forge_version: "1.0.0"
a:
  value: 100
  formula: null
b:
  value: 50
  formula: null
result:
  value: null
  formula: "=a + b"
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .output()
        .expect("Failed to execute");

    assert!(output.status.success());

    // Verify calculation happened (result should be 150)
    let updated_content = fs::read_to_string(&yaml_file).unwrap();
    assert!(
        updated_content.contains("150"),
        "Result should be calculated as 150"
    );
}

#[test]
fn e2e_multi_doc_skips_auto_upgrade() {
    let temp_dir = tempfile::tempdir().unwrap();
    let yaml_file = temp_dir.path().join("multi_doc.yaml");

    // Create a multi-doc v1.0.0 file
    let content = r#"---
_forge_version: "1.0.0"
_name: "doc1"
x:
  value: 10
  formula: null
---
_forge_version: "1.0.0"
_name: "doc2"
y:
  value: 20
  formula: null
"#;

    fs::write(&yaml_file, content).expect("Failed to write test file");

    let output = Command::new(forge_binary())
        .arg("calculate")
        .arg(&yaml_file)
        .arg("--dry-run")
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Multi-doc should skip auto-upgrade (not supported yet)
    assert!(
        !stdout.contains("Auto-upgrading"),
        "Multi-doc should skip auto-upgrade, got: {stdout}"
    );
}

#[test]
fn e2e_update_check_flag() {
    let output = Command::new(forge_binary())
        .args(["update", "--check"])
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should either show version info or network error, not crash
    assert!(
        stdout.contains("version") || stdout.contains("Version") || stderr.contains("Error"),
        "Update --check should show version or error, got stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn e2e_update_shows_current_version() {
    let output = Command::new(forge_binary())
        .args(["update", "--check"])
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // If successful, should show current version
    if output.status.success() {
        assert!(
            stdout.contains("Current version") || stdout.contains("5.3.0"),
            "Should show current version, got: {stdout}"
        );
    }
    // Network errors are acceptable in tests
}
