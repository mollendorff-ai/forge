//! Structured result types for CLI commands
//!
//! These types capture computation results that were previously only printed.
//! Used by both CLI (format & print) and MCP server (serialize to JSON).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of the validate command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub tables_valid: bool,
    pub scalars_valid: bool,
    pub table_count: usize,
    pub scalar_count: usize,
    pub mismatches: Vec<ValidationMismatch>,
}

/// A single value mismatch found during validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMismatch {
    pub name: String,
    pub current_value: f64,
    pub expected_value: f64,
    pub diff: f64,
}

/// Result of the calculate command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationResult {
    pub tables: HashMap<String, TableSummary>,
    pub scalars: HashMap<String, Option<f64>>,
    pub unit_warnings: Vec<String>,
    pub file_updated: bool,
    pub dry_run: bool,
}

/// Summary of a calculated table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSummary {
    pub name: String,
    pub column_count: usize,
    pub row_count: usize,
    pub columns: Vec<String>,
}

/// Result of the audit command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    pub variable: String,
    pub var_type: String,
    pub current_value: Option<f64>,
    pub calculated_value: Option<f64>,
    pub formula: Option<String>,
    pub dependencies: Vec<AuditDep>,
    pub is_valid: bool,
}

/// Serializable dependency info (mirrors `AuditDependency` with Serialize)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditDep {
    pub name: String,
    pub dep_type: String,
    pub formula: Option<String>,
    pub value: Option<f64>,
    pub children: Vec<AuditDep>,
}

/// Result of the export command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub input_path: String,
    pub output_path: String,
    pub table_count: usize,
    pub scalar_count: usize,
}

/// Result of the import command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub input_path: String,
    pub output_path: String,
    pub table_count: usize,
    pub scalar_count: usize,
    pub mode: String,
}

/// Result of sensitivity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityResult {
    pub vary: String,
    pub output: String,
    pub data: SensitivityData,
}

/// Sensitivity data — one-variable or two-variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SensitivityData {
    OneVar {
        entries: Vec<SensitivityEntry>,
    },
    TwoVar {
        vary2: String,
        row_values: Vec<f64>,
        col_values: Vec<f64>,
        matrix: Vec<Vec<Option<f64>>>,
    },
}

/// A single point in 1D sensitivity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityEntry {
    pub input: f64,
    pub output: Option<f64>,
    pub error: Option<String>,
}

/// Result of goal-seek (also used for break-even)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSeekResult {
    pub vary: String,
    pub target: String,
    pub target_value: f64,
    pub solution: f64,
    pub achieved: f64,
    pub error: f64,
    pub iterations: i32,
    pub converged: bool,
}

/// Result of variance analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarianceAnalysis {
    pub results: Vec<VarianceEntry>,
    pub favorable_count: usize,
    pub unfavorable_count: usize,
    pub alert_count: usize,
    pub threshold: f64,
}

/// A single variance entry (serializable version of `VarianceResult`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarianceEntry {
    pub name: String,
    pub budget: f64,
    pub actual: f64,
    pub variance: f64,
    pub variance_pct: f64,
    pub is_favorable: bool,
    pub exceeds_threshold: bool,
}

/// Result of scenario comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub scenarios: Vec<String>,
    pub variables: Vec<String>,
    pub values: HashMap<String, HashMap<String, Option<f64>>>,
}
