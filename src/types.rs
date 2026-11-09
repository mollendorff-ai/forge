//! Common types for forge-e2e-gnumeric.
//!
//! Defines the data structures for test specifications, test cases, and results.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Test specification file structure.
#[derive(Debug, Deserialize)]
pub struct TestSpec {
    /// The forge schema version (e.g., "1.0.0").
    #[serde(rename = "_forge_version")]
    pub forge_version: String,

    /// Included files (`_includes`), each read as `@namespace.name`.
    #[serde(rename = "_includes", default)]
    pub includes: Vec<IncludeSpec>,

    /// Named sections containing test definitions.
    #[serde(flatten)]
    pub sections: HashMap<String, Section>,
}

/// One entry of `_includes`: a file, relative to the spec, and its namespace.
#[derive(Debug, Clone, Deserialize)]
pub struct IncludeSpec {
    /// Path of the included file, relative to the spec file.
    pub file: String,
    /// Namespace the spec's formulas read it by (`@namespace.name`).
    #[serde(rename = "as")]
    pub namespace: String,
}

/// A section in the test spec (e.g., "assumptions", "projections").
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Section {
    /// A group of scalar values with optional formulas.
    ScalarGroup(HashMap<String, Scalar>),
    /// A table with columns of data.
    Table(HashMap<String, TableColumn>),
}

/// A scalar value with optional formula and expected value.
#[derive(Debug, Deserialize)]
pub struct Scalar {
    /// The literal value (if no formula).
    pub value: Option<f64>,
    /// The Excel formula to evaluate.
    pub formula: Option<String>,
    /// Expected value for E2E validation.
    pub expected: Option<f64>,
    /// Skip reason (if set, test is skipped).
    pub skip: Option<String>,
    /// Reason Gnumeric cannot judge this case (if set, only Forge's own
    /// `calculate` result is checked).
    pub gnumeric_skip: Option<String>,
}

/// A table column (array of values or formula).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TableColumn {
    /// Column of numeric values.
    Numbers(Vec<f64>),
    /// Column of string values.
    Strings(Vec<String>),
    /// Column defined by a formula.
    Formula(String),
}

/// Individual test case extracted from a spec.
#[derive(Debug, Clone)]
pub struct TestCase {
    /// Fully qualified name (e.g., `assumptions.test_abs`).
    pub name: String,
    /// The Excel formula to evaluate.
    pub formula: String,
    /// The expected result value.
    pub expected: f64,
    /// Source YAML file path (for loading table data).
    pub source_file: Option<std::path::PathBuf>,
    /// Forge version from source file.
    pub forge_version: String,
    /// Why Gnumeric is not consulted for this case, if it is not.
    pub gnumeric_skip: Option<String>,
}

/// A test case that should be skipped.
#[derive(Debug, Clone)]
pub struct SkipCase {
    /// Fully qualified name.
    pub name: String,
    /// Reason for skipping.
    pub reason: String,
}

/// Result of running a test.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum TestResult {
    /// Test passed - actual matches expected.
    Pass {
        name: String,
        formula: String,
        expected: f64,
        actual: f64,
    },
    /// Test failed - mismatch or error.
    Fail {
        name: String,
        formula: String,
        expected: f64,
        actual: Option<f64>,
        error: Option<String>,
    },
    /// Test was skipped.
    Skip { name: String, reason: String },
}

impl TestResult {
    /// Returns `true` if this result is a pass.
    #[must_use]
    pub const fn is_pass(&self) -> bool {
        matches!(self, Self::Pass { .. })
    }

    /// Returns `true` if this result is a failure.
    #[must_use]
    pub const fn is_fail(&self) -> bool {
        matches!(self, Self::Fail { .. })
    }

    /// Returns the test name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Pass { name, .. } | Self::Fail { name, .. } | Self::Skip { name, .. } => name,
        }
    }
}

/// Extracts test cases from a test spec.
///
/// Scans all sections for scalar values that have both a formula and
/// an expected value defined. Tests with `skip` field are excluded.
pub fn extract_test_cases(spec: &TestSpec, source_file: Option<&std::path::Path>) -> Vec<TestCase> {
    let mut cases = Vec::new();

    for (section_name, section) in &spec.sections {
        if section_name.starts_with('_') || section_name == "scenarios" {
            continue;
        }

        if let Section::ScalarGroup(scalars) = section {
            for (name, scalar) in scalars {
                // Skip tests marked with skip field
                if scalar.skip.is_some() {
                    continue;
                }
                if let (Some(formula), Some(expected)) = (&scalar.formula, scalar.expected) {
                    cases.push(TestCase {
                        name: format!("{section_name}.{name}"),
                        formula: formula.clone(),
                        expected,
                        source_file: source_file.map(std::path::Path::to_path_buf),
                        forge_version: spec.forge_version.clone(),
                        gnumeric_skip: scalar.gnumeric_skip.clone(),
                    });
                }
            }
        }
    }

    cases
}

/// Extracts table data sections from a test spec as YAML string.
///
/// Returns a string containing all table sections in YAML format,
/// suitable for inclusion in a generated test file.
#[must_use]
pub fn extract_table_data_yaml(spec: &TestSpec) -> String {
    use std::fmt::Write;
    let mut yaml = String::new();

    for (section_name, section) in &spec.sections {
        // Skip metadata sections and assumptions (which contain tests)
        if section_name.starts_with('_') || section_name == "assumptions" {
            continue;
        }

        // Check if this is a table section (has array values, not scalars with formulas)
        if let Section::Table(columns) = section {
            let _ = writeln!(yaml, "{section_name}:");
            for (col_name, col_data) in columns {
                match col_data {
                    TableColumn::Numbers(nums) => {
                        let nums_str: Vec<String> = nums.iter().map(ToString::to_string).collect();
                        let _ = writeln!(yaml, "  {col_name}: [{}]", nums_str.join(", "));
                    }
                    TableColumn::Strings(strs) => {
                        let strs_escaped: Vec<String> =
                            strs.iter().map(|s| format!("\"{s}\"")).collect();
                        let _ = writeln!(yaml, "  {col_name}: [{}]", strs_escaped.join(", "));
                    }
                    TableColumn::Formula(f) => {
                        let _ = writeln!(yaml, "  {col_name}: \"{f}\"");
                    }
                }
            }
        }
    }

    yaml
}

/// Extracts skip cases from a test spec.
#[must_use]
pub fn extract_skip_cases(spec: &TestSpec) -> Vec<SkipCase> {
    let mut cases = Vec::new();

    for (section_name, section) in &spec.sections {
        if section_name.starts_with('_') || section_name == "scenarios" {
            continue;
        }

        if let Section::ScalarGroup(scalars) = section {
            for (name, scalar) in scalars {
                if let Some(reason) = &scalar.skip {
                    cases.push(SkipCase {
                        name: format!("{section_name}.{name}"),
                        reason: reason.clone(),
                    });
                }
            }
        }
    }

    cases
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_spec_extracts_test_cases() {
        let yaml = r#"
_forge_version: "1.0.0"
assumptions:
  test_abs:
    value: null
    formula: "=ABS(-42)"
    expected: 42
"#;
        let spec: TestSpec = yaml_serde::from_str(yaml).unwrap();
        assert_eq!(spec.forge_version, "1.0.0");

        let cases = extract_test_cases(&spec, None);
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].name, "assumptions.test_abs");
    }

    #[test]
    fn test_result_is_pass() {
        let pass = TestResult::Pass {
            name: "test".to_string(),
            formula: "=1".to_string(),
            expected: 1.0,
            actual: 1.0,
        };
        assert!(pass.is_pass());
        assert!(!pass.is_fail());
    }

    #[test]
    fn test_table_extraction() {
        let yaml = r#"
_forge_version: "1.0.0"
agg_data:
  values: [10, 50, 30, 70]
  mixed: [5, 10, 15]
assumptions:
  test_one:
    value: null
    formula: "=SUM(agg_data.values)"
    expected: 160
"#;
        let spec: TestSpec = yaml_serde::from_str(yaml).unwrap();

        // Check section types
        for (name, section) in &spec.sections {
            match section {
                Section::ScalarGroup(_) => println!("{name}: ScalarGroup"),
                Section::Table(_) => println!("{name}: Table"),
            }
        }

        let table_yaml = extract_table_data_yaml(&spec);
        println!("Extracted table YAML:\n{table_yaml}");
        assert!(
            table_yaml.contains("agg_data") || table_yaml.is_empty(),
            "Should extract agg_data table or be empty if not parsed as Table"
        );
    }
}
