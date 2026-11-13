//! Structured result types for CLI commands
//!
//! These types capture computation results that were previously only printed.
//! Used by both CLI (format & print) and MCP server (serialize to JSON).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
///
/// In JSON a date scalar's value is its text (`"2026-10-31"`), as a cell of a
/// date column is (ADR-060); here it is its day number in `scalars`, named in
/// `dates`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "CalculationWire", into = "CalculationWire")]
pub struct CalculationResult {
    pub tables: BTreeMap<String, TableSummary>,
    pub scalars: BTreeMap<String, Option<f64>>,
    /// The scalars whose value is a date
    pub dates: std::collections::BTreeSet<String>,
    /// Each scalar with a `format:`, as that format shows it (ADR-061)
    pub formatted: BTreeMap<String, String>,
    pub unit_warnings: Vec<String>,
    pub file_updated: bool,
    pub dry_run: bool,
}

impl CalculationResult {
    /// A scalar's value as the model shows it: the date's text for a date,
    /// else the number.
    #[must_use]
    pub fn shown(&self, name: &str) -> Option<String> {
        let value = self.scalars.get(name).copied().flatten()?;
        Some(if self.dates.contains(name) {
            crate::core::array_calculator::evaluator::serial_date_text(value)
        } else {
            value.to_string()
        })
    }
}

/// [`CalculationResult`] as JSON: each scalar a number, a date's text or null.
#[derive(Serialize, Deserialize)]
struct CalculationWire {
    tables: BTreeMap<String, TableSummary>,
    scalars: BTreeMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    formatted: BTreeMap<String, String>,
    unit_warnings: Vec<String>,
    file_updated: bool,
    dry_run: bool,
}

impl From<CalculationResult> for CalculationWire {
    fn from(r: CalculationResult) -> Self {
        let scalars = r
            .scalars
            .iter()
            .map(|(name, value)| {
                let json = match value {
                    Some(n) if r.dates.contains(name) => serde_json::Value::String(
                        crate::core::array_calculator::evaluator::serial_date_text(*n),
                    ),
                    Some(n) => serde_json::json!(n),
                    None => serde_json::Value::Null,
                };
                (name.clone(), json)
            })
            .collect();
        Self {
            tables: r.tables,
            scalars,
            formatted: r.formatted,
            unit_warnings: r.unit_warnings,
            file_updated: r.file_updated,
            dry_run: r.dry_run,
        }
    }
}

impl From<CalculationWire> for CalculationResult {
    fn from(w: CalculationWire) -> Self {
        let mut dates = std::collections::BTreeSet::new();
        let scalars = w
            .scalars
            .into_iter()
            .map(|(name, json)| {
                let value = match &json {
                    serde_json::Value::String(text) => {
                        let serial =
                            crate::core::array_calculator::evaluator::excel_date_serial(text);
                        if serial.is_some() {
                            dates.insert(name.clone());
                        }
                        serial
                    },
                    other => other.as_f64(),
                };
                (name, value)
            })
            .collect();
        Self {
            tables: w.tables,
            scalars,
            dates,
            formatted: w.formatted,
            unit_warnings: w.unit_warnings,
            file_updated: w.file_updated,
            dry_run: w.dry_run,
        }
    }
}

/// Summary of a calculated table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSummary {
    pub name: String,
    pub column_count: usize,
    pub row_count: usize,
    pub columns: Vec<String>,
    /// Calculated values of every column, in row order: numbers, text, dates
    /// (as text) or booleans. Keyed by column name, in name order.
    #[serde(default)]
    pub values: BTreeMap<String, Vec<serde_json::Value>>,
    /// Each column with a `format:`, as that format shows its values
    /// (ADR-061); an empty cell is ""
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub formatted: BTreeMap<String, Vec<String>>,
}

/// Result of the audit command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    pub variable: String,
    pub var_type: String,
    pub current_value: Option<f64>,
    pub calculated_value: Option<f64>,
    pub formula: Option<String>,
    /// For a table column: its calculated values, in row order (`None` is an empty cell)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calculated_values: Option<Vec<Option<f64>>>,
    pub dependencies: Vec<AuditDep>,
    /// The value in the file matches the calculated one, and the model calculates
    pub is_valid: bool,
    /// Why the model does not calculate, if it does not (values are then the file's)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calculation_error: Option<String>,
    /// What to check: NPV or IRR reading a column with empty cells (ADR-059)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    /// The calculated value as a date, when it is one (ADR-060)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calculated_date: Option<String>,
}

/// One name a formula reads, with what it reads in turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditDep {
    pub name: String,
    /// `Scalar`, `Included scalar`, `Column[table]` or `Unknown`
    pub dep_type: String,
    pub formula: Option<String>,
    /// A scalar's calculated value
    pub value: Option<f64>,
    pub children: Vec<Self>,
    /// A column's calculated values, in row order (`None` is an empty cell, ADR-059)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<Option<f64>>>,
    /// The included file it comes from
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Shown earlier in the tree, so its dependencies are not repeated
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub repeated: bool,
    /// A scalar's value as a date, when it is one (ADR-060)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// Result of the export command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub input_path: String,
    pub output_path: String,
    pub table_count: usize,
    pub scalar_count: usize,
    /// Formulas written as values because Excel cannot express them, and
    /// parts of the model that are not exported at all (scenarios)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// Result of an in-memory (buffer) export — returned when no `excel_path` is given
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportBufferResult {
    pub table_count: usize,
    pub scalar_count: usize,
    /// Base64-encoded .xlsx file content
    pub excel_base64: String,
    /// Size of the raw Excel file in bytes
    pub byte_count: usize,
    /// What the export could not do, for example that the model does not calculate
    pub warnings: Vec<String>,
}

/// Result of the import command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub input_path: String,
    pub output_path: String,
    pub table_count: usize,
    pub scalar_count: usize,
    pub mode: String,
    /// Formulas imported as values because Forge cannot express them
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    /// Included files written beside the model, from the sheets of an included file
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub included_files: Vec<String>,
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
    /// Variances whose direction is unknown (no `better:` and no telling name)
    pub no_direction_count: usize,
    /// Variances exactly on budget (not counted as favorable)
    pub on_target_count: usize,
    pub alert_count: usize,
    pub threshold: f64,
    /// Scalars missing or empty in one of the two models; not compared
    pub not_compared: Vec<String>,
}

/// A single variance entry (serializable version of `VarianceResult`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarianceEntry {
    pub name: String,
    pub budget: f64,
    pub actual: f64,
    pub variance: f64,
    /// Variance as a percentage of the size of the budget
    pub variance_pct: f64,
    /// `null` when Forge cannot tell whether higher or lower is better
    pub is_favorable: Option<bool>,
    /// "higher", "lower" or "unknown"
    pub better: String,
    pub exceeds_threshold: bool,
}

/// Result of scenario comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub scenarios: Vec<String>,
    pub variables: Vec<String>,
    /// Variable, then scenario, in model and request order
    pub values: indexmap::IndexMap<String, indexmap::IndexMap<String, Option<f64>>>,
    /// The `description` of each scenario that has one
    #[serde(default, skip_serializing_if = "indexmap::IndexMap::is_empty")]
    pub descriptions: indexmap::IndexMap<String, String>,
    /// The variables whose values are dates, held as day numbers in `values`
    /// and shown as dates, also in JSON (ADR-060)
    #[serde(skip)]
    pub dates: std::collections::BTreeSet<String>,
}
