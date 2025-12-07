//! Import tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;
use super::common::create_test_yaml;
use tempfile::TempDir;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_import_basic() {
    // First export a YAML to Excel, then import it back
    let dir = TempDir::new().unwrap();
    // Use proper table array format
    let yaml = create_test_yaml(
        &dir,
        "import_source.yaml",
        r#"_forge_version: "1.0.0"
sales:
  product: ["A", "B", "C"]
  revenue: [100, 200, 300]
"#,
    );
    let xlsx = dir.path().join("temp.xlsx");
    let output_yaml = dir.path().join("imported.yaml");

    // Export first
    export(yaml, xlsx.clone(), false).unwrap();

    // Now import
    let result = import(xlsx, output_yaml.clone(), false, false, false);
    assert!(result.is_ok());
    assert!(output_yaml.exists());
}

#[test]
fn test_import_verbose() {
    let dir = TempDir::new().unwrap();
    // Use proper table array format
    let yaml = create_test_yaml(
        &dir,
        "import_verbose_source.yaml",
        r#"_forge_version: "1.0.0"
data:
  x: [1, 2, 3]
"#,
    );
    let xlsx = dir.path().join("temp_verbose.xlsx");
    let output_yaml = dir.path().join("imported_verbose.yaml");

    export(yaml, xlsx.clone(), false).unwrap();

    let result = import(xlsx, output_yaml.clone(), true, false, false);
    assert!(result.is_ok());
}

#[test]
fn test_import_split_files() {
    let dir = TempDir::new().unwrap();
    // Use proper table array format
    let yaml = create_test_yaml(
        &dir,
        "import_split_source.yaml",
        r#"_forge_version: "1.0.0"
table1:
  a: [1, 2]
table2:
  b: [3, 4]
"#,
    );
    let xlsx = dir.path().join("split.xlsx");
    let output_dir = dir.path().join("split_output");

    export(yaml, xlsx.clone(), false).unwrap();

    let result = import(xlsx, output_dir.clone(), false, true, false);
    assert!(result.is_ok());
    assert!(output_dir.exists());
}

#[test]
fn test_import_split_files_with_scalars_verbose() {
    let dir = TempDir::new().unwrap();
    // Create YAML with both tables and scalars
    let yaml = create_test_yaml(
        &dir,
        "import_mixed.yaml",
        r#"_forge_version: "1.0.0"
sales:
  revenue: [100, 200, 300]
tax_rate:
  value: 0.1
  formula: null
"#,
    );
    let xlsx = dir.path().join("mixed.xlsx");
    let output_dir = dir.path().join("mixed_output");

    export(yaml, xlsx.clone(), false).unwrap();

    // verbose=true, split_files=true
    let result = import(xlsx, output_dir.clone(), true, true, false);
    assert!(result.is_ok());
    // Should create both table and scalar files
    assert!(output_dir.exists());
}

#[test]
fn test_import_multi_doc_with_scalars() {
    let dir = TempDir::new().unwrap();
    // Create YAML with tables and scalars for multi-doc output
    let yaml = create_test_yaml(
        &dir,
        "import_multidoc_mixed.yaml",
        r#"_forge_version: "1.0.0"
data:
  values: [1, 2, 3]
constant:
  value: 42
  formula: null
"#,
    );
    let xlsx = dir.path().join("multidoc.xlsx");
    let output_yaml = dir.path().join("multidoc_out.yaml");

    export(yaml, xlsx.clone(), false).unwrap();

    // multi_doc=true - should include scalars document
    let result = import(xlsx, output_yaml.clone(), false, false, true);
    assert!(result.is_ok());
}

#[test]
fn test_import_multi_doc() {
    let dir = TempDir::new().unwrap();
    // Use proper table array format
    let yaml = create_test_yaml(
        &dir,
        "import_multi_source.yaml",
        r#"_forge_version: "1.0.0"
data:
  values: [1, 2, 3]
"#,
    );
    let xlsx = dir.path().join("multi.xlsx");
    let output_yaml = dir.path().join("multi.yaml");

    export(yaml, xlsx.clone(), false).unwrap();

    let result = import(xlsx, output_yaml.clone(), false, false, true);
    assert!(result.is_ok());
}

#[test]
fn test_import_file_not_found() {
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("output.yaml");

    let result = import(
        PathBuf::from("/nonexistent.xlsx"),
        output,
        false,
        false,
        false,
    );
    assert!(result.is_err());
}
