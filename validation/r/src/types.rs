//! Common types for forge-e2e-r.
//!
//! Defines test specifications and results for R validation.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Analytics test specification.
#[derive(Debug, Clone, Deserialize)]
pub struct AnalyticsTestSpec {
    /// Test name (populated from `HashMap` key).
    #[serde(default)]
    pub name: String,
    /// Distribution type (for Monte Carlo).
    pub distribution: Option<String>,
    /// Parameters for the distribution/analysis.
    #[serde(default)]
    pub params: HashMap<String, f64>,
    /// Random seed for reproducibility.
    #[serde(default = "default_seed")]
    pub seed: u64,
    /// Number of iterations.
    #[serde(default = "default_iterations")]
    pub iterations: usize,
    /// R validator script to use.
    pub r_validator: Option<String>,
    /// Expected results from R.
    pub r_expected: Option<RExpected>,
    /// Tolerance levels.
    pub tolerance: Option<ToleranceSpec>,
    /// The whole test entry as written in the YAML file. Category runners
    /// (bootstrap, tornado, ...) read their own fields from here.
    #[serde(skip)]
    pub raw: yaml_serde::Value,
}

const fn default_seed() -> u64 {
    42
}

const fn default_iterations() -> usize {
    10_000
}

/// Expected values from R.
#[derive(Debug, Clone, Deserialize)]
pub struct RExpected {
    pub mean: Option<f64>,
    pub std: Option<f64>,
    #[serde(default)]
    pub percentiles: HashMap<String, f64>,
}

/// Tolerance specification from YAML.
#[derive(Debug, Clone, Deserialize)]
pub struct ToleranceSpec {
    pub mean: Option<f64>,
    pub std: Option<f64>,
    pub percentiles: Option<f64>,
}

/// Result of running a test.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum TestResult {
    /// Test passed.
    Pass { name: String, details: String },
    /// Test failed.
    Fail { name: String, reason: String },
    /// Test errored.
    Error { name: String, error: String },
    /// Test was skipped.
    Skip { name: String, reason: String },
}

impl TestResult {
    #[must_use]
    pub const fn is_pass(&self) -> bool {
        matches!(self, Self::Pass { .. })
    }

    #[must_use]
    pub const fn is_fail(&self) -> bool {
        matches!(self, Self::Fail { .. })
    }

    /// Returns `true` if the test could not be run (missing R package,
    /// Forge crash, unparseable output). An error is not a pass.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Pass { name, .. }
            | Self::Fail { name, .. }
            | Self::Error { name, .. }
            | Self::Skip { name, .. } => name,
        }
    }
}

/// Analytics test file structure.
#[derive(Debug, Deserialize)]
pub struct AnalyticsTestFile {
    /// R validator for this file.
    #[serde(rename = "_r_validator")]
    pub r_validator: Option<String>,

    /// Tests in this file.
    #[serde(default)]
    pub tests: HashMap<String, AnalyticsTestSpec>,
}

/// Loads analytics test specs from a YAML file.
///
/// # Errors
///
/// Returns an error if the YAML content cannot be parsed.
pub fn load_analytics_tests(content: &str) -> anyhow::Result<Vec<AnalyticsTestSpec>> {
    let file: AnalyticsTestFile = yaml_serde::from_str(content)?;
    let document: yaml_serde::Value = yaml_serde::from_str(content)?;
    let mut tests = Vec::new();

    for (name, mut spec) in file.tests {
        spec.raw = document
            .get("tests")
            .and_then(|t| t.get(name.as_str()))
            .cloned()
            .unwrap_or(yaml_serde::Value::Null);
        spec.name = name;
        if spec.r_validator.is_none() {
            spec.r_validator.clone_from(&file.r_validator);
        }
        tests.push(spec);
    }

    Ok(tests)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_analytics_test() {
        let yaml = r#"
_r_validator: "monte_carlo_validator.R"
tests:
  test_normal:
    distribution: normal
    params:
      mean: 100
      sd: 15
    seed: 42
    iterations: 10000
"#;
        let tests = load_analytics_tests(yaml).unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].distribution, Some("normal".to_string()));
    }

    #[test]
    fn error_is_neither_pass_nor_fail_but_is_error() {
        let error = TestResult::Error {
            name: "t".to_string(),
            error: "R package 'boot' is not installed".to_string(),
        };
        assert!(error.is_error());
        assert!(!error.is_pass());
        assert!(!error.is_fail());
    }
}
