//! Typed request structs for Forge MCP tools.
//!
//! Each struct corresponds to one of the 20 MCP tools and derives
//! `JsonSchema` so rmcp can auto-generate input schemas.

use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;

/// Parameters for the `forge_validate` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateRequest {
    /// Path to the YAML model file to validate
    pub file_path: String,
    /// Whether to show verbose output
    #[serde(default)]
    pub verbose: bool,
}

/// Parameters for the `forge_calculate` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CalculateRequest {
    /// Path to the YAML model file to calculate
    pub file_path: String,
    /// Whether to perform a dry run (don't update file)
    #[serde(default)]
    pub dry_run: bool,
    /// Scenario name to apply (uses variable overrides from 'scenarios' section)
    pub scenario: Option<String>,
}

/// Parameters for the `forge_audit` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuditRequest {
    /// Path to the YAML model file
    pub file_path: String,
    /// Name of the variable to audit
    pub variable: String,
}

/// Parameters for the `forge_export` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportRequest {
    /// Path to the YAML model file
    pub yaml_path: String,
    /// Path for the output Excel file
    pub excel_path: String,
}

/// Parameters for the `forge_import` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImportRequest {
    /// Path to the Excel file to import
    pub excel_path: String,
    /// Path for the output YAML file
    pub yaml_path: String,
}

/// Parameters for the `forge_sensitivity` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SensitivityRequest {
    /// Path to the YAML model file
    pub file_path: String,
    /// Name of the input variable to vary
    pub vary: String,
    /// Range for the variable: start,end,step (e.g., '80,120,10')
    pub range: String,
    /// Name of the output variable to observe
    pub output: String,
    /// Optional second variable for 2D analysis
    pub vary2: Option<String>,
    /// Optional range for second variable
    pub range2: Option<String>,
}

/// Parameters for the `forge_goal_seek` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GoalSeekRequest {
    /// Path to the YAML model file
    pub file_path: String,
    /// Name of the target output variable
    pub target: String,
    /// Desired value for the target
    pub value: f64,
    /// Name of the input variable to adjust
    pub vary: String,
    /// Optional minimum bound for search
    pub min: Option<f64>,
    /// Optional maximum bound for search
    pub max: Option<f64>,
    /// Solution tolerance
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
}

const fn default_tolerance() -> f64 {
    0.0001
}

/// Parameters for the `forge_break_even` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BreakEvenRequest {
    /// Path to the YAML model file
    pub file_path: String,
    /// Name of the output variable to find zero crossing
    pub output: String,
    /// Name of the input variable to adjust
    pub vary: String,
    /// Optional minimum bound for search
    pub min: Option<f64>,
    /// Optional maximum bound for search
    pub max: Option<f64>,
}

/// Parameters for the `forge_variance` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct VarianceRequest {
    /// Path to the budget YAML file
    pub budget_path: String,
    /// Path to the actual YAML file
    pub actual_path: String,
    /// Variance threshold percentage for alerts (default: 10)
    pub threshold: Option<f64>,
}

/// Parameters for the `forge_compare` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CompareRequest {
    /// Path to the YAML model file
    pub file_path: String,
    /// List of scenario names to compare
    pub scenarios: Vec<String>,
}

/// Parameters for the `forge_simulate` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimulateRequest {
    /// Path to YAML model with `monte_carlo` config and `MC.*` distribution formulas
    pub file_path: String,
    /// Number of simulation iterations (default: from YAML config or 10000)
    pub iterations: Option<u64>,
    /// Random seed for reproducibility
    pub seed: Option<u64>,
    /// Sampling method: 'random' or `latin_hypercube` (default: from config)
    pub sampling: Option<String>,
}

/// Parameters for the `forge_scenarios` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ScenariosRequest {
    /// Path to YAML model with 'scenarios' section
    pub file_path: String,
    /// Optional: run only this named scenario
    pub scenario_filter: Option<String>,
}

/// Parameters for the `forge_decision_tree` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DecisionTreeRequest {
    /// Path to YAML model with `decision_tree` section
    pub file_path: String,
}

/// Parameters for the `forge_real_options` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RealOptionsRequest {
    /// Path to YAML model with `real_options` section
    pub file_path: String,
}

/// Parameters for the `forge_tornado` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TornadoRequest {
    /// Path to YAML model with 'tornado' section
    pub file_path: String,
    /// Optional: override the output variable to analyze
    pub output_var: Option<String>,
}

/// Parameters for the `forge_bootstrap` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BootstrapRequest {
    /// Path to YAML model with 'bootstrap' section containing data and config
    pub file_path: String,
    /// Override number of bootstrap iterations
    pub iterations: Option<u64>,
    /// Random seed for reproducibility
    pub seed: Option<u64>,
    /// Override confidence levels (e.g., [0.90, 0.95, 0.99])
    pub confidence_levels: Option<Vec<f64>>,
}

/// Parameters for the `forge_bayesian` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BayesianRequest {
    /// Path to YAML model with `bayesian_network` section
    pub file_path: String,
    /// Optional: specific variable to query (omit for all nodes)
    pub query_var: Option<String>,
    /// Evidence as variable=state pairs (e.g., "economy=growth", "market=bull")
    pub evidence: Option<Vec<String>>,
}

/// Parameters for the `forge_schema` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SchemaRequest {
    /// Schema version: 'v1' (scalar-only) or 'v5' (full enterprise). Omit to list available versions.
    pub version: Option<String>,
}

/// Parameters for the `forge_functions` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FunctionsRequest {}

/// Parameters for the `forge_examples` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExamplesRequest {
    /// Example name (e.g., 'monte-carlo', 'scenarios', 'decision-tree'). Omit to list all.
    pub name: Option<String>,
}
