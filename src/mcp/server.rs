//! Forge MCP Server implementation
//!
//! Provides the MCP server that AI agents use to interact with Forge.
//! Uses the rmcp SDK for Model Context Protocol over stdin/stdout.

use std::path::Path;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};

use crate::cli::calculate_core;
use crate::cli::{
    audit_core, bayesian_core, bootstrap_core, compare_core, decision_tree_core, examples_core,
    export_core, functions_core, goal_seek_core, import_core, real_options_core, scenarios_core,
    schema_core, sensitivity_core, simulate_core, tornado_core, validate_core, variance_core,
};

use super::types::{
    AuditRequest, BayesianRequest, BootstrapRequest, BreakEvenRequest, CalculateRequest,
    CompareRequest, DecisionTreeRequest, ExamplesRequest, ExportRequest, FunctionsRequest,
    GoalSeekRequest, ImportRequest, RealOptionsRequest, ScenariosRequest, SchemaRequest,
    SensitivityRequest, SimulateRequest, TornadoRequest, ValidateRequest, VarianceRequest,
};

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

/// Serialize a successful result to JSON string.
fn to_json<T: serde::Serialize>(result: &T) -> String {
    serde_json::to_string(result).unwrap_or_default()
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
        validate_core(Path::new(&req.file_path))
            .map(|r| to_json(&r))
            .map_err(|e| format!("Validation failed: {e}"))
    }

    #[tool(
        name = "forge_calculate",
        description = "Calculate all formulas in a Forge YAML model and optionally update the file."
    )]
    fn calculate(&self, Parameters(req): Parameters<CalculateRequest>) -> Result<String, String> {
        calculate_core(
            Path::new(&req.file_path),
            req.dry_run,
            req.scenario.as_deref(),
        )
        .map(|r| to_json(&r))
        .map_err(|e| format!("Calculation failed: {e}"))
    }

    #[tool(
        name = "forge_audit",
        description = "Audit a specific variable to see its dependency tree and calculated value."
    )]
    fn audit(&self, Parameters(req): Parameters<AuditRequest>) -> Result<String, String> {
        audit_core(Path::new(&req.file_path), &req.variable)
            .map(|r| to_json(&r))
            .map_err(|e| format!("Audit failed: {e}"))
    }

    #[tool(
        name = "forge_export",
        description = "Export a Forge YAML model to an Excel workbook."
    )]
    fn export(&self, Parameters(req): Parameters<ExportRequest>) -> Result<String, String> {
        export_core(Path::new(&req.yaml_path), Path::new(&req.excel_path))
            .map(|r| to_json(&r))
            .map_err(|e| format!("Export failed: {e}"))
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
        sensitivity_core(
            Path::new(&req.file_path),
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
        goal_seek_core(
            Path::new(&req.file_path),
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
        goal_seek_core(
            Path::new(&req.file_path),
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
        let threshold = req.threshold.unwrap_or(10.0);
        variance_core(
            Path::new(&req.budget_path),
            Path::new(&req.actual_path),
            threshold,
        )
        .map(|r| to_json(&r))
        .map_err(|e| format!("Variance analysis failed: {e}"))
    }

    #[tool(
        name = "forge_compare",
        description = "Compare calculation results across multiple scenarios side-by-side. Useful for what-if analysis."
    )]
    fn compare(&self, Parameters(req): Parameters<CompareRequest>) -> Result<String, String> {
        compare_core(Path::new(&req.file_path), &req.scenarios)
            .map(|r| to_json(&r))
            .map_err(|e| format!("Scenario comparison failed: {e}"))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ANALYSIS ENGINE TOOLS
    // ═══════════════════════════════════════════════════════════════════════

    #[tool(
        name = "forge_simulate",
        description = "Run Monte Carlo simulation with probabilistic distributions (MC.Normal, MC.Triangular, MC.Uniform, MC.PERT, MC.Lognormal). Returns statistics, percentiles, and threshold probabilities."
    )]
    fn simulate(&self, Parameters(req): Parameters<SimulateRequest>) -> Result<String, String> {
        // MCP iteration counts are always small, saturating cast is safe
        #[allow(clippy::cast_possible_truncation)]
        let iterations = req.iterations.map(|n| n as usize);

        simulate_core(
            Path::new(&req.file_path),
            iterations,
            req.seed,
            req.sampling.as_deref(),
        )
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
        scenarios_core(Path::new(&req.file_path), req.scenario_filter.as_deref())
            .map(|r| to_json(&r))
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
        decision_tree_core(Path::new(&req.file_path))
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
        real_options_core(Path::new(&req.file_path))
            .map(|r| to_json(&r))
            .map_err(|e| format!("Real options analysis failed: {e}"))
    }

    #[tool(
        name = "forge_tornado",
        description = "Generate tornado sensitivity diagram. Varies each input one-at-a-time to show which inputs have the greatest impact on the output."
    )]
    fn tornado(&self, Parameters(req): Parameters<TornadoRequest>) -> Result<String, String> {
        tornado_core(Path::new(&req.file_path), req.output_var.as_deref())
            .map(|r| to_json(&r))
            .map_err(|e| format!("Tornado analysis failed: {e}"))
    }

    #[tool(
        name = "forge_bootstrap",
        description = "Non-parametric bootstrap resampling for confidence intervals. Returns original estimate, bootstrap mean, std error, bias, and confidence intervals."
    )]
    fn bootstrap(&self, Parameters(req): Parameters<BootstrapRequest>) -> Result<String, String> {
        #[allow(clippy::cast_possible_truncation)]
        let iterations = req.iterations.map(|n| n as usize);

        bootstrap_core(
            Path::new(&req.file_path),
            iterations,
            req.seed,
            req.confidence_levels,
        )
        .map(|r| to_json(&r))
        .map_err(|e| format!("Bootstrap analysis failed: {e}"))
    }

    #[tool(
        name = "forge_bayesian",
        description = "Bayesian network inference. Query posterior probabilities with optional evidence. Returns probability distributions for each variable state."
    )]
    fn bayesian(&self, Parameters(req): Parameters<BayesianRequest>) -> Result<String, String> {
        let evidence = req.evidence.unwrap_or_default();
        bayesian_core(
            Path::new(&req.file_path),
            req.query_var.as_deref(),
            &evidence,
        )
        .map(|r| to_json(&r))
        .map_err(|e| format!("Bayesian inference failed: {e}"))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // DISCOVERY TOOLS
    // ═══════════════════════════════════════════════════════════════════════

    #[tool(
        name = "forge_schema",
        description = "Get JSON Schema for Forge YAML model formats. Use to understand the structure of valid Forge model files."
    )]
    fn schema(&self, Parameters(req): Parameters<SchemaRequest>) -> Result<String, String> {
        schema_core(req.version.as_deref()).map_err(|e| format!("Schema error: {e}"))
    }

    #[tool(
        name = "forge_functions",
        description = "List all 173 supported Excel-compatible functions with descriptions and syntax. Organized by category (Financial, Statistical, Math, Lookup, etc.)."
    )]
    fn functions(&self, Parameters(_req): Parameters<FunctionsRequest>) -> Result<String, String> {
        functions_core()
            .map(|r| to_json(&r))
            .map_err(|e| format!("Functions list failed: {e}"))
    }

    #[tool(
        name = "forge_examples",
        description = "Get runnable YAML examples for all Forge capabilities. Use without a name to list available examples, or specify a name to get the full YAML content."
    )]
    fn examples(&self, Parameters(req): Parameters<ExamplesRequest>) -> Result<String, String> {
        examples_core(req.name.as_deref())
            .map(|r| to_json(&r))
            .map_err(|e| format!("Examples error: {e}"))
    }
}

#[tool_handler]
impl ServerHandler for ForgeMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Forge MCP Server - 20 tools for AI-native financial modeling. Core: validate, calculate, audit, export, import. Analysis: sensitivity, goal-seek, break-even, variance, compare. Engines: simulate (Monte Carlo), scenarios, decision-tree, real-options, tornado, bootstrap, bayesian. Discovery: schema, functions, examples. 173 Excel-compatible functions. All tools return structured JSON.".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: rmcp::model::Implementation {
                name: "forge".into(),
                title: None,
                version: env!("CARGO_PKG_VERSION").into(),
                description: None,
                icons: None,
                website_url: None,
            },
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::handler::server::wrapper::Parameters;
    use tempfile::TempDir;

    /// Helper: get Ok text or panic with the error string
    fn ok_text(result: Result<String, String>) -> String {
        result.expect("expected Ok result")
    }

    /// Helper: get Err text or panic
    fn err_text(result: Result<String, String>) -> String {
        result.expect_err("expected Err result")
    }

    // ═══════════════════════════════════════════════════════════════════════
    // SERVER INFO TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_server_get_info() {
        let server = ForgeMcpServer::new();
        let info = server.get_info();
        assert_eq!(info.server_info.name, "forge");
        assert!(info.instructions.is_some());
        assert!(info.capabilities.tools.is_some());
    }

    #[test]
    fn test_tool_count() {
        let server = ForgeMcpServer::new();
        let tools = server.tool_router.list_all();
        assert_eq!(tools.len(), 20, "Expected 20 tools, got {}", tools.len());
    }

    #[test]
    fn test_tool_names() {
        let server = ForgeMcpServer::new();
        let tools = server.tool_router.list_all();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

        // Core tools
        assert!(names.contains(&"forge_validate"));
        assert!(names.contains(&"forge_calculate"));
        assert!(names.contains(&"forge_audit"));
        assert!(names.contains(&"forge_export"));
        assert!(names.contains(&"forge_import"));
        // Financial analysis tools
        assert!(names.contains(&"forge_sensitivity"));
        assert!(names.contains(&"forge_goal_seek"));
        assert!(names.contains(&"forge_break_even"));
        assert!(names.contains(&"forge_variance"));
        assert!(names.contains(&"forge_compare"));
        // Analysis engine tools
        assert!(names.contains(&"forge_simulate"));
        assert!(names.contains(&"forge_scenarios"));
        assert!(names.contains(&"forge_decision_tree"));
        assert!(names.contains(&"forge_real_options"));
        assert!(names.contains(&"forge_tornado"));
        assert!(names.contains(&"forge_bootstrap"));
        assert!(names.contains(&"forge_bayesian"));
        // Discovery tools
        assert!(names.contains(&"forge_schema"));
        assert!(names.contains(&"forge_functions"));
        assert!(names.contains(&"forge_examples"));
    }

    #[test]
    fn test_tool_schemas_are_objects() {
        let server = ForgeMcpServer::new();
        let tools = server.tool_router.list_all();

        for tool in &tools {
            assert_eq!(
                tool.input_schema.get("type").and_then(|v| v.as_str()),
                Some("object"),
                "Tool {} schema missing type: object",
                tool.name
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // TOOL CALL TESTS WITH FIXTURES
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_call_validate_success() {
        let server = ForgeMcpServer::new();
        let result = server.validate(Parameters(ValidateRequest {
            file_path: "test-data/budget.yaml".into(),
            verbose: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_call_validate_nonexistent() {
        let server = ForgeMcpServer::new();
        let result = server.validate(Parameters(ValidateRequest {
            file_path: "nonexistent.yaml".into(),
            verbose: false,
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_call_calculate_dry_run() {
        let server = ForgeMcpServer::new();
        let result = server.calculate(Parameters(CalculateRequest {
            file_path: "test-data/budget.yaml".into(),
            dry_run: true,
            scenario: None,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_call_calculate_nonexistent() {
        let server = ForgeMcpServer::new();
        let result = server.calculate(Parameters(CalculateRequest {
            file_path: "nonexistent.yaml".into(),
            dry_run: true,
            scenario: None,
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_call_audit() {
        let server = ForgeMcpServer::new();
        // audit may succeed or fail, but should not panic
        let _ = server.audit(Parameters(AuditRequest {
            file_path: "test-data/budget.yaml".into(),
            variable: "assumptions.profit".into(),
        }));
    }

    #[test]
    fn test_call_export() {
        let temp_dir = TempDir::new().unwrap();
        let output = temp_dir.path().join("mcp_test_export.xlsx");

        let server = ForgeMcpServer::new();
        let result = server.export(Parameters(ExportRequest {
            yaml_path: "test-data/budget.yaml".into(),
            excel_path: output.to_str().unwrap().into(),
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_call_import() {
        let temp_dir = TempDir::new().unwrap();
        let excel_path = temp_dir.path().join("import_test.xlsx");
        let yaml_path = temp_dir.path().join("imported.yaml");

        let server = ForgeMcpServer::new();

        // First export to create Excel file
        let _ = server.export(Parameters(ExportRequest {
            yaml_path: "test-data/budget.yaml".into(),
            excel_path: excel_path.to_str().unwrap().into(),
        }));

        // Then import
        let result = server.import(Parameters(ImportRequest {
            excel_path: excel_path.to_str().unwrap().into(),
            yaml_path: yaml_path.to_str().unwrap().into(),
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_call_sensitivity() {
        let server = ForgeMcpServer::new();
        // May succeed or fail depending on test fixture
        let _ = server.sensitivity(Parameters(SensitivityRequest {
            file_path: "test-data/sensitivity_test.yaml".into(),
            vary: "price".into(),
            range: "80,120,10".into(),
            output: "profit".into(),
            vary2: None,
            range2: None,
        }));
    }

    #[test]
    fn test_call_sensitivity_two_var() {
        let server = ForgeMcpServer::new();
        let _ = server.sensitivity(Parameters(SensitivityRequest {
            file_path: "test-data/sensitivity_test.yaml".into(),
            vary: "price".into(),
            range: "80,120,10".into(),
            output: "profit".into(),
            vary2: Some("quantity".into()),
            range2: Some("100,200,50".into()),
        }));
    }

    #[test]
    fn test_call_goal_seek() {
        let server = ForgeMcpServer::new();
        let _ = server.goal_seek(Parameters(GoalSeekRequest {
            file_path: "test-data/budget.yaml".into(),
            target: "assumptions.profit".into(),
            value: 0.0,
            vary: "assumptions.revenue".into(),
            min: Some(50_000.0),
            max: Some(200_000.0),
            tolerance: 0.01,
        }));
    }

    #[test]
    fn test_call_break_even() {
        let server = ForgeMcpServer::new();
        let _ = server.break_even(Parameters(BreakEvenRequest {
            file_path: "test-data/budget.yaml".into(),
            output: "assumptions.profit".into(),
            vary: "assumptions.revenue".into(),
            min: Some(50_000.0),
            max: Some(200_000.0),
        }));
    }

    #[test]
    fn test_call_variance() {
        let server = ForgeMcpServer::new();
        let result = server.variance(Parameters(VarianceRequest {
            budget_path: "test-data/budget.yaml".into(),
            actual_path: "test-data/budget.yaml".into(),
            threshold: Some(10.0),
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_call_compare() {
        let server = ForgeMcpServer::new();
        let result = server.compare(Parameters(CompareRequest {
            file_path: "test-data/budget.yaml".into(),
            scenarios: vec!["base".into(), "optimistic".into()],
        }));
        // Expected to fail - no scenarios in budget.yaml
        assert!(result.is_err());
    }

    #[test]
    fn test_call_compare_empty_scenarios() {
        let server = ForgeMcpServer::new();
        let _ = server.compare(Parameters(CompareRequest {
            file_path: "test-data/budget.yaml".into(),
            scenarios: vec![],
        }));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ANALYSIS ENGINE TOOL TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_call_simulate() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("mc.yaml");
        std::fs::write(
            &file,
            r#"
_forge_version: "5.0.0"
monte_carlo:
  enabled: true
  iterations: 100
  seed: 42
  outputs:
    - variable: revenue
      percentiles: [50]
scalars:
  revenue:
    value: 100000
    formula: "=MC.Normal(100000, 15000)"
"#,
        )
        .unwrap();

        let server = ForgeMcpServer::new();
        let text = ok_text(server.simulate(Parameters(SimulateRequest {
            file_path: file.to_str().unwrap().into(),
            iterations: None,
            seed: None,
            sampling: None,
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(
            parsed["monte_carlo_results"]["iterations"]
                .as_u64()
                .unwrap()
                > 0
        );
    }

    #[test]
    fn test_call_simulate_nonexistent() {
        let server = ForgeMcpServer::new();
        let result = server.simulate(Parameters(SimulateRequest {
            file_path: "nonexistent.yaml".into(),
            iterations: None,
            seed: None,
            sampling: None,
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_call_scenarios_dispatch() {
        let server = ForgeMcpServer::new();
        let e = err_text(server.scenarios(Parameters(ScenariosRequest {
            file_path: "test-data/budget.yaml".into(),
            scenario_filter: None,
        })));
        assert!(e.contains("scenarios"));
    }

    #[test]
    fn test_call_scenarios_nonexistent() {
        let server = ForgeMcpServer::new();
        assert!(server
            .scenarios(Parameters(ScenariosRequest {
                file_path: "nonexistent.yaml".into(),
                scenario_filter: None,
            }))
            .is_err());
    }

    #[test]
    fn test_call_decision_tree() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.decision_tree(Parameters(DecisionTreeRequest {
            file_path: "examples/decision-tree.yaml".into(),
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed["optimal_path"].as_array().is_some());
    }

    #[test]
    fn test_call_real_options() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.real_options(Parameters(RealOptionsRequest {
            file_path: "examples/real-options.yaml".into(),
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed["total_option_value"].as_f64().is_some());
    }

    #[test]
    fn test_call_tornado() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.tornado(Parameters(TornadoRequest {
            file_path: "examples/tornado.yaml".into(),
            output_var: None,
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed["base_value"].as_f64().is_some());
    }

    #[test]
    fn test_call_bootstrap() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.bootstrap(Parameters(BootstrapRequest {
            file_path: "examples/bootstrap.yaml".into(),
            iterations: None,
            seed: Some(42),
            confidence_levels: None,
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed["original_estimate"].as_f64().is_some());
    }

    #[test]
    fn test_call_bayesian_dispatch() {
        let server = ForgeMcpServer::new();
        let e = err_text(server.bayesian(Parameters(BayesianRequest {
            file_path: "test-data/budget.yaml".into(),
            query_var: None,
            evidence: None,
        })));
        assert!(e.contains("bayesian_network"));
    }

    #[test]
    fn test_call_bayesian_with_evidence() {
        let server = ForgeMcpServer::new();
        // Should fail (no bayesian_network section), but should not panic
        let _ = server.bayesian(Parameters(BayesianRequest {
            file_path: "test-data/budget.yaml".into(),
            query_var: None,
            evidence: Some(vec!["economy=growth".into()]),
        }));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // DISCOVERY TOOL TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_call_schema_v5() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.schema(Parameters(SchemaRequest {
            version: Some("v5".into()),
        })));
        assert!(text.contains("$schema"));
    }

    #[test]
    fn test_call_schema_v1() {
        let server = ForgeMcpServer::new();
        assert!(server
            .schema(Parameters(SchemaRequest {
                version: Some("v1".into()),
            }))
            .is_ok());
    }

    #[test]
    fn test_call_schema_list() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.schema(Parameters(SchemaRequest { version: None })));
        assert!(text.contains("available_versions"));
    }

    #[test]
    fn test_call_schema_invalid() {
        let server = ForgeMcpServer::new();
        assert!(server
            .schema(Parameters(SchemaRequest {
                version: Some("v99".into()),
            }))
            .is_err());
    }

    #[test]
    fn test_call_functions() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.functions(Parameters(FunctionsRequest {})));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["edition"], "enterprise");
        assert!(parsed["total"].as_u64().unwrap() >= 170);
    }

    #[test]
    fn test_call_examples_list() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.examples(Parameters(ExamplesRequest { name: None })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed.as_array().is_some_and(|a| a.len() >= 9));
    }

    #[test]
    fn test_call_examples_specific() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.examples(Parameters(ExamplesRequest {
            name: Some("monte-carlo".into()),
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["name"], "monte-carlo");
        assert!(parsed["content"]
            .as_str()
            .is_some_and(|c| c.contains("monte_carlo")));
    }

    #[test]
    fn test_call_examples_unknown() {
        let server = ForgeMcpServer::new();
        assert!(server
            .examples(Parameters(ExamplesRequest {
                name: Some("nonexistent".into()),
            }))
            .is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // STRUCTURED RESULT VERIFICATION TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_validate_returns_structured_json() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.validate(Parameters(ValidateRequest {
            file_path: "test-data/budget.yaml".into(),
            verbose: false,
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed.get("tables_valid").is_some());
        assert!(parsed.get("scalars_valid").is_some());
        assert!(parsed.get("table_count").is_some());
    }

    #[test]
    fn test_calculate_returns_structured_json() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.calculate(Parameters(CalculateRequest {
            file_path: "test-data/budget.yaml".into(),
            dry_run: true,
            scenario: None,
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed.get("tables").is_some());
        assert!(parsed.get("scalars").is_some());
        assert_eq!(parsed["dry_run"], true);
    }
}
