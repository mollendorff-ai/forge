//! Forge MCP Server implementation
//!
//! Provides the MCP server that AI agents use to interact with Forge.
//! Uses the rmcp SDK for Model Context Protocol over stdin/stdout.

use std::path::Path;

use crate::inline_model::resolve_model_input;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerConfig};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};

use crate::cli::calculate_core;
use crate::cli::{
    audit_core, bayesian_core, bootstrap_core, compare_core, comparison_json, decision_tree_core,
    examples_core, export_buffer_core, export_core, functions_core, goal_seek_core, import_core,
    real_options_core, scenarios_core, schema_core, sensitivity_core, simulate_core, tornado_core,
    validate_core, variance_core,
};

use super::types::{
    AuditRequest, BayesianRequest, BootstrapRequest, BreakEvenRequest, CalculateRequest,
    CompareRequest, DecisionTreeRequest, ExamplesRequest, ExportRequest, FunctionsRequest,
    GoalSeekRequest, ImportRequest, RealOptionsRequest, ScenariosRequest, SchemaRequest,
    SensitivityRequest, SimulateRequest, TornadoRequest, ValidateRequest, VarianceRequest,
};

/// The bootstrap result without the full list of resampled statistics.
///
/// The list has one number per iteration (10,000 by default, about 37,000
/// tokens), which fills an agent's context for no gain. Nine percentiles of the
/// same sorted list describe its shape; `forge bootstrap --output` still writes
/// every value.
fn compact_bootstrap(result: &crate::bootstrap::BootstrapResult) -> serde_json::Value {
    let mut value = serde_json::to_value(result).unwrap_or_default();
    let sorted = &result.distribution;
    if let Some(map) = value.as_object_mut() {
        map.remove("distribution");
        if !sorted.is_empty() {
            let at = |p: f64| {
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    clippy::cast_precision_loss
                )]
                let i = ((p * (sorted.len() - 1) as f64).round() as usize).min(sorted.len() - 1);
                sorted[i]
            };
            let percentiles: Vec<serde_json::Value> = [1, 5, 10, 25, 50, 75, 90, 95, 99]
                .iter()
                .map(|p| serde_json::json!({"percentile": p, "value": at(f64::from(*p) / 100.0)}))
                .collect();
            map.insert(
                "distribution_percentiles".to_string(),
                serde_json::Value::Array(percentiles),
            );
            map.insert(
                "distribution_size".to_string(),
                serde_json::json!(sorted.len()),
            );
        }
    }
    value
}

// ═══════════════════════════════════════════════════════════════════════════
// SERVER
// ═══════════════════════════════════════════════════════════════════════════

/// Forge MCP Server
#[derive(Debug, Clone)]
pub struct ForgeMcpServer {
    tool_router: ToolRouter<Self>,
}

impl ForgeMcpServer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for ForgeMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Serialize a successful result to JSON string, numbers to 15 significant
/// digits (`core::precision`).
fn to_json<T: serde::Serialize>(result: &T) -> String {
    serde_json::to_string(&crate::core::precision::Rounded(result)).unwrap_or_default()
}

/// Scenario results as JSON, a date's value as its text, as in `forge calculate`
/// (ADR-060): the engine holds a date as a day number.
fn scenarios_json(path: &Path, results: &crate::scenarios::ScenarioResults) -> String {
    let mut json =
        serde_json::to_value(crate::core::precision::Rounded(results)).unwrap_or_default();
    let dates: Vec<String> = crate::parser::parse_model(path).map_or_else(
        |_| Vec::new(),
        |m| {
            m.scalars
                .iter()
                .filter(|(_, v)| v.is_date)
                .map(|(k, _)| k.clone())
                .collect()
        },
    );
    let text = |v: &serde_json::Value| {
        v.as_f64().map_or_else(
            || v.clone(),
            |n| {
                serde_json::Value::String(
                    crate::core::array_calculator::evaluator::serial_date_text(n),
                )
            },
        )
    };
    for name in &dates {
        if let Some(scenarios) = json["scenarios"].as_array_mut() {
            for scenario in scenarios {
                if let Some(v) = scenario["scalars"].get_mut(name) {
                    *v = text(v);
                }
            }
        }
        if let Some(v) = json["expected_values"].get_mut(name) {
            *v = text(v);
        }
        if let Some(range) = json["ranges"].get_mut(name).and_then(|r| r.as_array_mut()) {
            for end in range {
                *end = text(end);
            }
        }
    }
    json.to_string()
}

#[tool_router]
#[allow(clippy::unused_self)] // rmcp #[tool] macro requires &self on all tool methods
impl ForgeMcpServer {
    // ═══════════════════════════════════════════════════════════════════════
    // CORE TOOLS
    // ═══════════════════════════════════════════════════════════════════════

    #[tool(
        name = "forge_validate",
        description = "Validate a Forge YAML model file for formula errors, circular dependencies, and type mismatches."
    )]
    fn validate(&self, Parameters(req): Parameters<ValidateRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        validate_core(&path)
            .map(|r| to_json(&r))
            .map_err(|e| format!("Validation failed: {e}"))
    }

    #[tool(
        name = "forge_calculate",
        description = "Calculate all formulas in a Forge YAML model. Unless dry_run, writes each formula's result next to it in the file, changing nothing else (comments kept)."
    )]
    fn calculate(&self, Parameters(req): Parameters<CalculateRequest>) -> Result<String, String> {
        let is_inline = req.content.is_some();
        let (path, _tmpdir) = resolve_model_input(
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        let result = calculate_core(path.as_path(), req.dry_run, req.scenario.as_deref())
            .map_err(|e| format!("Calculation failed: {e}"))?;

        let mut json_val: serde_json::Value =
            serde_json::to_value(&result).map_err(|e| format!("Serialization failed: {e}"))?;

        // When using inline content with dry_run=false, read back the calculated file
        if is_inline && !req.dry_run {
            if let Ok(calculated) = std::fs::read_to_string(&path) {
                json_val["calculated_content"] = serde_json::Value::String(calculated);
            }
        }

        serde_json::to_string(&crate::core::precision::Rounded(&json_val))
            .map_err(|e| format!("Serialization failed: {e}"))
    }

    #[tool(
        name = "forge_audit",
        description = "Audit a specific variable to see its dependency tree and calculated value."
    )]
    fn audit(&self, Parameters(req): Parameters<AuditRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        audit_core(&path, &req.variable)
            .map(|r| to_json(&r))
            .map_err(|e| format!("Audit failed: {e}"))
    }

    #[tool(
        name = "forge_export",
        description = "Export a Forge YAML model to an Excel workbook. \
            If `excel_path` is provided, writes the .xlsx file to disk. \
            If omitted, returns the workbook inline as base64 in the \
            `excel_base64` response field (for sandboxed clients)."
    )]
    fn export(&self, Parameters(req): Parameters<ExportRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            req.yaml_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        req.excel_path.as_ref().map_or_else(
            || {
                export_buffer_core(&path)
                    .map(|r| to_json(&r))
                    .map_err(|e| format!("Export failed: {e}"))
            },
            |excel_path| {
                export_core(&path, Path::new(excel_path))
                    .map(|r| to_json(&r))
                    .map_err(|e| format!("Export failed: {e}"))
            },
        )
    }

    #[tool(
        name = "forge_import",
        description = "Import an Excel workbook into a Forge YAML model."
    )]
    fn import(&self, Parameters(req): Parameters<ImportRequest>) -> Result<String, String> {
        import_core(
            Path::new(&req.excel_path),
            Path::new(&req.yaml_path),
            false,
            false,
        )
        .map(|r| to_json(&r))
        .map_err(|e| format!("Import failed: {e}"))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // FINANCIAL ANALYSIS TOOLS
    // ═══════════════════════════════════════════════════════════════════════

    #[tool(
        name = "forge_sensitivity",
        description = "Run sensitivity analysis by varying one or two input variables and observing output changes. Essential for what-if modeling and risk assessment."
    )]
    fn sensitivity(
        &self,
        Parameters(req): Parameters<SensitivityRequest>,
    ) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        sensitivity_core(
            &path,
            &req.vary,
            &req.range,
            req.vary2.as_deref(),
            req.range2.as_deref(),
            &req.output,
        )
        .map(|r| to_json(&r))
        .map_err(|e| format!("Sensitivity analysis failed: {e}"))
    }

    #[tool(
        name = "forge_goal_seek",
        description = "Find the input value needed to achieve a target output. Uses bisection solver. Example: 'What price do I need for $100K profit?'"
    )]
    fn goal_seek(&self, Parameters(req): Parameters<GoalSeekRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        goal_seek_core(
            &path,
            &req.target,
            req.value,
            &req.vary,
            (req.min, req.max),
            req.tolerance,
        )
        .map(|r| to_json(&r))
        .map_err(|e| format!("Goal seek failed: {e}"))
    }

    #[tool(
        name = "forge_break_even",
        description = "Find the break-even point where an output equals zero. Example: 'At what units does profit = 0?'"
    )]
    fn break_even(&self, Parameters(req): Parameters<BreakEvenRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        goal_seek_core(
            &path,
            &req.output,
            0.0,
            &req.vary,
            (req.min, req.max),
            0.0001,
        )
        .map(|r| to_json(&r))
        .map_err(|e| format!("Break-even analysis failed: {e}"))
    }

    #[tool(
        name = "forge_variance",
        description = "Compare budget vs actual with variance analysis. Shows absolute and percentage variances with favorable/unfavorable indicators."
    )]
    fn variance(&self, Parameters(req): Parameters<VarianceRequest>) -> Result<String, String> {
        let (budget_path, _tmpdir_b) = resolve_model_input(
            req.budget_path.as_deref(),
            req.budget_content.as_deref(),
            req.includes.as_ref(),
        )
        .map_err(|e| e.replace("file_path or content", "budget_path or budget_content"))?;
        let (actual_path, _tmpdir_a) = resolve_model_input(
            req.actual_path.as_deref(),
            req.actual_content.as_deref(),
            req.includes.as_ref(),
        )
        .map_err(|e| e.replace("file_path or content", "actual_path or actual_content"))?;
        let threshold = req.threshold.unwrap_or(10.0);
        variance_core(&budget_path, &actual_path, threshold)
            .map(|r| to_json(&r))
            .map_err(|e| format!("Variance analysis failed: {e}"))
    }

    #[tool(
        name = "forge_compare",
        description = "Compare calculation results across multiple scenarios side-by-side. Useful for what-if analysis."
    )]
    fn compare(&self, Parameters(req): Parameters<CompareRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        compare_core(&path, &req.scenarios)
            .map(|r| comparison_json(&r).to_string())
            .map_err(|e| format!("Scenario comparison failed: {e}"))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ANALYSIS ENGINE TOOLS
    // ═══════════════════════════════════════════════════════════════════════

    #[tool(
        name = "forge_simulate",
        description = "Run Monte Carlo simulation with probabilistic distributions (MC.Normal, MC.Triangular, MC.Uniform, MC.PERT, MC.Lognormal). Returns statistics, percentiles, threshold probabilities, and drivers (each MC.* input's rank correlation with each output). An MC.* input may be in an included file. `scenario` applies a scenario's overrides first. An output may be a table column (one result per row). With `seek` (\"P(year.profit > 0) = 0.8\"), `vary`, `min` and `max`, returns the value of that input at which the probability is first met."
    )]
    fn simulate(&self, Parameters(req): Parameters<SimulateRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        // MCP iteration counts are always small, saturating cast is safe
        #[allow(clippy::cast_possible_truncation)]
        let iterations = req.iterations.map(|n| n as usize);

        let options = crate::cli::SimulateOptions {
            iterations,
            seed: req.seed,
            sampling: req.sampling.as_deref(),
            scenario: req.scenario.as_deref(),
        };
        if let Some(seek) = req.seek.as_deref() {
            let (Some(vary), Some(min), Some(max)) = (req.vary.as_deref(), req.min, req.max) else {
                return Err("seek needs vary, min and max".to_string());
            };
            let goal = crate::cli::SeekGoal::parse(seek)?;
            return crate::cli::probability_seek_core(&path, options, &goal, vary, min, max)
                .map_err(|e| format!("Seek failed: {e}"))
                .and_then(|r| {
                    serde_json::to_string_pretty(&crate::core::precision::Rounded(&r))
                        .map_err(|e| format!("Serialization failed: {e}"))
                });
        }
        simulate_core(&path, options)
            .map_err(|e| format!("Simulation failed: {e}"))
            .and_then(|r| {
                r.to_json()
                    .map_err(|e| format!("Serialization failed: {e}"))
            })
    }

    #[tool(
        name = "forge_scenarios",
        description = "Run probability-weighted scenario analysis (Base/Bull/Bear). Each scenario overrides scalar values and calculates results."
    )]
    fn scenarios(&self, Parameters(req): Parameters<ScenariosRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        scenarios_core(&path, req.scenario_filter.as_deref())
            .map(|r| scenarios_json(&path, &r))
            .map_err(|e| format!("Scenario analysis failed: {e}"))
    }

    #[tool(
        name = "forge_decision_tree",
        description = "Analyze decision trees using backward induction. Returns optimal path, expected value, decision policy, and risk profile."
    )]
    fn decision_tree(
        &self,
        Parameters(req): Parameters<DecisionTreeRequest>,
    ) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        decision_tree_core(&path)
            .map(|r| to_json(&r))
            .map_err(|e| format!("Decision tree analysis failed: {e}"))
    }

    #[tool(
        name = "forge_real_options",
        description = "Value managerial flexibility (defer/expand/abandon) using real options pricing. Returns option values, exercise probabilities, and project value with options."
    )]
    fn real_options(
        &self,
        Parameters(req): Parameters<RealOptionsRequest>,
    ) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        real_options_core(&path)
            .map(|r| to_json(&r))
            .map_err(|e| format!("Real options analysis failed: {e}"))
    }

    #[tool(
        name = "forge_tornado",
        description = "Generate tornado sensitivity diagram. Varies each input one-at-a-time to show which inputs have the greatest impact on the output."
    )]
    fn tornado(&self, Parameters(req): Parameters<TornadoRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        tornado_core(&path, req.output_var.as_deref())
            .map(|r| to_json(&r))
            .map_err(|e| format!("Tornado analysis failed: {e}"))
    }

    #[tool(
        name = "forge_bootstrap",
        description = "Non-parametric bootstrap resampling for confidence intervals. Returns original estimate, bootstrap mean, std error, bias, confidence intervals and nine percentiles of the bootstrap distribution (not every resample)."
    )]
    fn bootstrap(&self, Parameters(req): Parameters<BootstrapRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        #[allow(clippy::cast_possible_truncation)]
        let iterations = req.iterations.map(|n| n as usize);

        let result = bootstrap_core(&path, iterations, req.seed, req.confidence_levels)
            .map_err(|e| format!("Bootstrap analysis failed: {e}"))?;
        Ok(to_json(&compact_bootstrap(&result)))
    }

    #[tool(
        name = "forge_bayesian",
        description = "Bayesian network inference. Query posterior probabilities with optional evidence. Returns probability distributions for each variable state."
    )]
    fn bayesian(&self, Parameters(req): Parameters<BayesianRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        let evidence = req.evidence.unwrap_or_default();
        bayesian_core(&path, req.query_var.as_deref(), &evidence)
            .map(|r| to_json(&r))
            .map_err(|e| format!("Bayesian inference failed: {e}"))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // DISCOVERY TOOLS
    // ═══════════════════════════════════════════════════════════════════════

    #[tool(
        name = "forge_schema",
        description = "Get JSON Schema for Forge YAML model formats. Write new models as _forge_version \"5.1.0\": inputs as plain numbers (price: 16), formula scalars as {formula: \"=...\"}, tables as lists and row-formula strings. For a table over time use \"5.2.0\": a period column (month: {start: 2026-12, count: 12, unit: month}) gives the rows, and PREV(value, first_row_value) reads the row above. \"5.3.0\" allows an empty cell (null) in a column of numbers. \"5.4.0\" allows a date as a scalar (last_actual: 2026-10-31) and format: beside unit:, an Excel number format that changes how a number is shown, never its value (price: {value: 16, format: \"$#,##0.00\"}); results then carry a formatted map."
    )]
    fn schema(&self, Parameters(req): Parameters<SchemaRequest>) -> Result<String, String> {
        schema_core(req.version.as_deref()).map_err(|e| format!("Schema error: {e}"))
    }

    #[tool(
        name = "forge_functions",
        description = "List all 203 supported Excel-compatible functions with descriptions and syntax. Organized by category (Financial, Statistical, Math, Lookup, etc.)."
    )]
    fn functions(&self, Parameters(_req): Parameters<FunctionsRequest>) -> Result<String, String> {
        functions_core()
            .map(|r| to_json(&r))
            .map_err(|e| format!("Functions list failed: {e}"))
    }

    #[tool(
        name = "forge_examples",
        description = "Get runnable YAML examples for all Forge capabilities. Use without a name to list available examples, or specify a name to get the full YAML content. `starter` is a first model to start from."
    )]
    fn examples(&self, Parameters(req): Parameters<ExamplesRequest>) -> Result<String, String> {
        examples_core(req.name.as_deref())
            .map(|r| to_json(&r))
            .map_err(|e| format!("Examples error: {e}"))
    }
}

#[allow(unknown_lints, clippy::unused_async_trait_impl)]
#[tool_handler(router = self.tool_router)]
impl ServerHandler for ForgeMcpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Forge MCP Server - 20 tools for AI-native financial modeling. Core: validate, calculate, audit, export, import. Analysis: sensitivity, goal-seek, break-even, variance, compare. Engines: simulate (Monte Carlo), scenarios, decision-tree, real-options, tornado, bootstrap, bayesian. Discovery: schema, functions, examples. 202 Excel-compatible functions. All tools return structured JSON. A new model: start from forge_examples name=starter; _forge_version is the model format version (5.1.0), not the Forge version; in a group of scalars a formula is {formula: \"=...\"}, in a table a column formula is a bare \"=...\".")
            .with_server_info(
                rmcp::model::Implementation::new("forge", env!("CARGO_PKG_VERSION"))
            )
    }
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
