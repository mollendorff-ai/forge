//! Forge MCP Server implementation
//!
//! Provides the MCP server that AI agents use to interact with Forge.
//! Implements the Model Context Protocol over stdin/stdout using JSON-RPC.

// Imports only used by non-coverage builds (see ADR-006)
#[cfg(not(coverage))]
use std::io::{BufRead, BufReader, Write};
#[cfg(any(not(coverage), test))]
use std::path::Path;

#[cfg(any(not(coverage), test))]
use serde::{Deserialize, Serialize};
#[cfg(any(not(coverage), test))]
use serde_json::{json, Value};

// Core function imports (return structured results, no printing)
#[cfg(any(not(coverage), test))]
use crate::cli::{
    audit_core, bayesian_core, bootstrap_core, compare_core, decision_tree_core, examples_core,
    export_core, functions_core, goal_seek_core, import_core, real_options_core, scenarios_core,
    schema_core, sensitivity_core, simulate_core, tornado_core, validate_core, variance_core,
};
// calculate_core is in cli::commands::mod, accessed via crate::cli
#[cfg(any(not(coverage), test))]
use crate::cli::calculate_core;

/// JSON-RPC request
#[cfg(any(not(coverage), test))]
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[serde(rename = "jsonrpc")]
    _jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

/// JSON-RPC response
#[cfg(any(not(coverage), test))]
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC error
#[cfg(any(not(coverage), test))]
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// MCP Tool definition
#[cfg(any(not(coverage), test))]
#[derive(Debug, Serialize)]
struct Tool {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
}

/// Run the MCP server synchronously over stdin/stdout
///
/// # Panics
///
/// Panics if a JSON-RPC response fails to serialize, which should never
/// happen with well-formed response structs.
///
/// # Coverage Exclusion (ADR-006)
/// This function reads from stdin forever until EOF. Cannot be unit tested.
/// The request handling logic is tested via `handle_request()`.
#[cfg(not(coverage))]
pub fn run_mcp_server_sync() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        let Ok(line) = line else { break };

        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let error_response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {e}"),
                        data: None,
                    }),
                };
                let _ = writeln!(
                    stdout,
                    "{}",
                    serde_json::to_string(&error_response).unwrap()
                );
                let _ = stdout.flush();
                continue;
            },
        };

        let response = handle_request(&request);

        if let Some(resp) = response {
            let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
            let _ = stdout.flush();
        }
    }
}

/// Stub for coverage builds - see ADR-006
#[cfg(coverage)]
pub fn run_mcp_server_sync() {}

/// Handle a JSON-RPC request
#[cfg(any(not(coverage), test))]
fn handle_request(request: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let id = request.id.clone().unwrap_or(Value::Null);

    match request.method.as_str() {
        "initialize" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {
                        "listChanged": false
                    }
                },
                "serverInfo": {
                    "name": "forge",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "instructions": "Forge MCP Server - 20 tools for AI-native financial modeling. Core: validate, calculate, audit, export, import. Analysis: sensitivity, goal-seek, break-even, variance, compare. Engines: simulate (Monte Carlo), scenarios, decision-tree, real-options, tornado, bootstrap, bayesian. Discovery: schema, functions, examples. 173 Excel-compatible functions. All tools return structured JSON."
            })),
            error: None,
        }),
        "notifications/initialized" => None, // No response for notifications
        "tools/list" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "tools": get_tools()
            })),
            error: None,
        }),
        "tools/call" => {
            let tool_name = request
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let arguments = request
                .params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));

            let result = call_tool(tool_name, &arguments);
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(result),
                error: None,
            })
        },
        "ping" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({})),
            error: None,
        }),
        _ => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", request.method),
                data: None,
            }),
        }),
    }
}

/// Get all available tools
#[cfg(any(not(coverage), test))]
// Declarative tool list - splitting would scatter related definitions
#[allow(clippy::too_many_lines)]
fn get_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "forge_validate".to_string(),
            description: "Validate a Forge YAML model file for formula errors, circular dependencies, and type mismatches.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to the YAML model file to validate"
                    },
                    "verbose": {
                        "type": "boolean",
                        "description": "Whether to show verbose output",
                        "default": false
                    }
                },
                "required": ["file_path"]
            }),
        },
        Tool {
            name: "forge_calculate".to_string(),
            description: "Calculate all formulas in a Forge YAML model and optionally update the file.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to the YAML model file to calculate"
                    },
                    "dry_run": {
                        "type": "boolean",
                        "description": "Whether to perform a dry run (don't update file)",
                        "default": false
                    }
                },
                "required": ["file_path"]
            }),
        },
        Tool {
            name: "forge_audit".to_string(),
            description: "Audit a specific variable to see its dependency tree and calculated value.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to the YAML model file"
                    },
                    "variable": {
                        "type": "string",
                        "description": "Name of the variable to audit"
                    }
                },
                "required": ["file_path", "variable"]
            }),
        },
        Tool {
            name: "forge_export".to_string(),
            description: "Export a Forge YAML model to an Excel workbook.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "yaml_path": {
                        "type": "string",
                        "description": "Path to the YAML model file"
                    },
                    "excel_path": {
                        "type": "string",
                        "description": "Path for the output Excel file"
                    }
                },
                "required": ["yaml_path", "excel_path"]
            }),
        },
        Tool {
            name: "forge_import".to_string(),
            description: "Import an Excel workbook into a Forge YAML model.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "excel_path": {
                        "type": "string",
                        "description": "Path to the Excel file to import"
                    },
                    "yaml_path": {
                        "type": "string",
                        "description": "Path for the output YAML file"
                    }
                },
                "required": ["excel_path", "yaml_path"]
            }),
        },
        // v3.0.0 Financial Analysis Tools
        Tool {
            name: "forge_sensitivity".to_string(),
            description: "Run sensitivity analysis by varying one or two input variables and observing output changes. Essential for what-if modeling and risk assessment.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to the YAML model file"
                    },
                    "vary": {
                        "type": "string",
                        "description": "Name of the input variable to vary"
                    },
                    "range": {
                        "type": "string",
                        "description": "Range for the variable: start,end,step (e.g., '80,120,10')"
                    },
                    "output": {
                        "type": "string",
                        "description": "Name of the output variable to observe"
                    },
                    "vary2": {
                        "type": "string",
                        "description": "Optional second variable for 2D analysis"
                    },
                    "range2": {
                        "type": "string",
                        "description": "Optional range for second variable"
                    }
                },
                "required": ["file_path", "vary", "range", "output"]
            }),
        },
        Tool {
            name: "forge_goal_seek".to_string(),
            description: "Find the input value needed to achieve a target output. Uses bisection solver. Example: 'What price do I need for $100K profit?'".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to the YAML model file"
                    },
                    "target": {
                        "type": "string",
                        "description": "Name of the target output variable"
                    },
                    "value": {
                        "type": "number",
                        "description": "Desired value for the target"
                    },
                    "vary": {
                        "type": "string",
                        "description": "Name of the input variable to adjust"
                    },
                    "min": {
                        "type": "number",
                        "description": "Optional minimum bound for search"
                    },
                    "max": {
                        "type": "number",
                        "description": "Optional maximum bound for search"
                    },
                    "tolerance": {
                        "type": "number",
                        "description": "Solution tolerance (default: 0.0001)"
                    }
                },
                "required": ["file_path", "target", "value", "vary"]
            }),
        },
        Tool {
            name: "forge_break_even".to_string(),
            description: "Find the break-even point where an output equals zero. Example: 'At what units does profit = 0?'".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to the YAML model file"
                    },
                    "output": {
                        "type": "string",
                        "description": "Name of the output variable to find zero crossing"
                    },
                    "vary": {
                        "type": "string",
                        "description": "Name of the input variable to adjust"
                    },
                    "min": {
                        "type": "number",
                        "description": "Optional minimum bound for search"
                    },
                    "max": {
                        "type": "number",
                        "description": "Optional maximum bound for search"
                    }
                },
                "required": ["file_path", "output", "vary"]
            }),
        },
        Tool {
            name: "forge_variance".to_string(),
            description: "Compare budget vs actual with variance analysis. Shows absolute and percentage variances with favorable/unfavorable indicators.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "budget_path": {
                        "type": "string",
                        "description": "Path to the budget YAML file"
                    },
                    "actual_path": {
                        "type": "string",
                        "description": "Path to the actual YAML file"
                    },
                    "threshold": {
                        "type": "number",
                        "description": "Variance threshold percentage for alerts (default: 10)"
                    }
                },
                "required": ["budget_path", "actual_path"]
            }),
        },
        Tool {
            name: "forge_compare".to_string(),
            description: "Compare calculation results across multiple scenarios side-by-side. Useful for what-if analysis.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to the YAML model file"
                    },
                    "scenarios": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of scenario names to compare (e.g., ['base', 'optimistic', 'pessimistic'])"
                    }
                },
                "required": ["file_path", "scenarios"]
            }),
        },
        // v10.0.0-beta.1 Analysis Tools
        Tool {
            name: "forge_simulate".to_string(),
            description: "Run Monte Carlo simulation with probabilistic distributions (MC.Normal, MC.Triangular, MC.Uniform, MC.PERT, MC.Lognormal). Returns statistics, percentiles, and threshold probabilities.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to YAML model with monte_carlo config and MC.* distribution formulas"
                    },
                    "iterations": {
                        "type": "integer",
                        "description": "Number of simulation iterations (default: from YAML config or 10000)"
                    },
                    "seed": {
                        "type": "integer",
                        "description": "Random seed for reproducibility"
                    },
                    "sampling": {
                        "type": "string",
                        "description": "Sampling method: 'random' or 'latin_hypercube' (default: from config)"
                    }
                },
                "required": ["file_path"]
            }),
        },
        Tool {
            name: "forge_scenarios".to_string(),
            description: "Run probability-weighted scenario analysis (Base/Bull/Bear). Each scenario overrides scalar values and calculates results.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to YAML model with 'scenarios' section"
                    },
                    "scenario_filter": {
                        "type": "string",
                        "description": "Optional: run only this named scenario"
                    }
                },
                "required": ["file_path"]
            }),
        },
        Tool {
            name: "forge_decision_tree".to_string(),
            description: "Analyze decision trees using backward induction. Returns optimal path, expected value, decision policy, and risk profile.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to YAML model with 'decision_tree' section"
                    }
                },
                "required": ["file_path"]
            }),
        },
        Tool {
            name: "forge_real_options".to_string(),
            description: "Value managerial flexibility (defer/expand/abandon) using real options pricing. Returns option values, exercise probabilities, and project value with options.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to YAML model with 'real_options' section"
                    }
                },
                "required": ["file_path"]
            }),
        },
        Tool {
            name: "forge_tornado".to_string(),
            description: "Generate tornado sensitivity diagram. Varies each input one-at-a-time to show which inputs have the greatest impact on the output.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to YAML model with 'tornado' section"
                    },
                    "output_var": {
                        "type": "string",
                        "description": "Optional: override the output variable to analyze"
                    }
                },
                "required": ["file_path"]
            }),
        },
        Tool {
            name: "forge_bootstrap".to_string(),
            description: "Non-parametric bootstrap resampling for confidence intervals. Returns original estimate, bootstrap mean, std error, bias, and confidence intervals.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to YAML model with 'bootstrap' section containing data and config"
                    },
                    "iterations": {
                        "type": "integer",
                        "description": "Override number of bootstrap iterations"
                    },
                    "seed": {
                        "type": "integer",
                        "description": "Random seed for reproducibility"
                    },
                    "confidence_levels": {
                        "type": "array",
                        "items": { "type": "number" },
                        "description": "Override confidence levels (e.g., [0.90, 0.95, 0.99])"
                    }
                },
                "required": ["file_path"]
            }),
        },
        Tool {
            name: "forge_bayesian".to_string(),
            description: "Bayesian network inference. Query posterior probabilities with optional evidence. Returns probability distributions for each variable state.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to YAML model with 'bayesian_network' section"
                    },
                    "query_var": {
                        "type": "string",
                        "description": "Optional: specific variable to query (omit for all nodes)"
                    },
                    "evidence": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Evidence as 'variable=state' pairs (e.g., ['economy=growth', 'market=bull'])"
                    }
                },
                "required": ["file_path"]
            }),
        },
        // v10.0.0-beta.1 Discovery Tools
        Tool {
            name: "forge_schema".to_string(),
            description: "Get JSON Schema for Forge YAML model formats. Use to understand the structure of valid Forge model files.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "version": {
                        "type": "string",
                        "description": "Schema version: 'v1' (scalar-only) or 'v5' (full enterprise with arrays, tables, Monte Carlo). Omit to list available versions."
                    }
                },
                "required": []
            }),
        },
        Tool {
            name: "forge_functions".to_string(),
            description: "List all 173 supported Excel-compatible functions with descriptions and syntax. Organized by category (Financial, Statistical, Math, Lookup, etc.).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "forge_examples".to_string(),
            description: "Get runnable YAML examples for all Forge capabilities. Use without a name to list available examples, or specify a name to get the full YAML content.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Example name (e.g., 'monte-carlo', 'scenarios', 'decision-tree', 'real-options', 'tornado', 'bootstrap', 'bayesian', 'variance', 'breakeven'). Omit to list all."
                    }
                },
                "required": []
            }),
        },
    ]
}

/// Call a tool by name
#[cfg(any(not(coverage), test))]
// Each match arm is a self-contained tool dispatch - splitting would hurt readability
#[allow(clippy::too_many_lines)]
fn call_tool(name: &str, arguments: &Value) -> Value {
    match name {
        "forge_validate" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let path = Path::new(file_path);
            match validate_core(path) {
                Ok(result) => json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&result).unwrap_or_default()
                    }],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Validation failed: {}", e)
                    }],
                    "isError": true
                }),
            }
        },
        "forge_calculate" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let dry_run = arguments
                .get("dry_run")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);

            let path = Path::new(file_path);
            let scenario = arguments.get("scenario").and_then(|v| v.as_str());
            match calculate_core(path, dry_run, scenario) {
                Ok(result) => json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&result).unwrap_or_default()
                    }],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Calculation failed: {}", e)
                    }],
                    "isError": true
                }),
            }
        },
        "forge_audit" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let variable = arguments
                .get("variable")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let path = Path::new(file_path);
            match audit_core(path, variable) {
                Ok(result) => json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&result).unwrap_or_default()
                    }],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Audit failed: {}", e)
                    }],
                    "isError": true
                }),
            }
        },
        "forge_export" => {
            let yaml_path = arguments
                .get("yaml_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let excel_path = arguments
                .get("excel_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let yaml = Path::new(yaml_path);
            let excel = Path::new(excel_path);
            match export_core(yaml, excel) {
                Ok(result) => json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&result).unwrap_or_default()
                    }],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Export failed: {}", e)
                    }],
                    "isError": true
                }),
            }
        },
        "forge_import" => {
            let excel_path = arguments
                .get("excel_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let yaml_path = arguments
                .get("yaml_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let excel = Path::new(excel_path);
            let yaml = Path::new(yaml_path);
            match import_core(excel, yaml, false, false) {
                Ok(result) => json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&result).unwrap_or_default()
                    }],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Import failed: {}", e)
                    }],
                    "isError": true
                }),
            }
        },
        // v3.0.0 Financial Analysis Tools
        "forge_sensitivity" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let vary = arguments.get("vary").and_then(|v| v.as_str()).unwrap_or("");
            let range = arguments
                .get("range")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let output = arguments
                .get("output")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let vary2 = arguments.get("vary2").and_then(|v| v.as_str());
            let range2 = arguments.get("range2").and_then(|v| v.as_str());

            let path = Path::new(file_path);
            match sensitivity_core(path, vary, range, vary2, range2, output) {
                Ok(result) => json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&result).unwrap_or_default()
                    }],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Sensitivity analysis failed: {}", e)
                    }],
                    "isError": true
                }),
            }
        },
        "forge_goal_seek" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let target = arguments
                .get("target")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let value = arguments
                .get("value")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let vary = arguments.get("vary").and_then(|v| v.as_str()).unwrap_or("");
            let min = arguments.get("min").and_then(serde_json::Value::as_f64);
            let max = arguments.get("max").and_then(serde_json::Value::as_f64);
            let tolerance = arguments
                .get("tolerance")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0001);

            let path = Path::new(file_path);
            match goal_seek_core(path, target, value, vary, (min, max), tolerance) {
                Ok(result) => json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&result).unwrap_or_default()
                    }],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Goal seek failed: {}", e)
                    }],
                    "isError": true
                }),
            }
        },
        "forge_break_even" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let output = arguments
                .get("output")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let vary = arguments.get("vary").and_then(|v| v.as_str()).unwrap_or("");
            let min = arguments.get("min").and_then(serde_json::Value::as_f64);
            let max = arguments.get("max").and_then(serde_json::Value::as_f64);

            let path = Path::new(file_path);
            match goal_seek_core(path, output, 0.0, vary, (min, max), 0.0001) {
                Ok(result) => json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&result).unwrap_or_default()
                    }],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Break-even analysis failed: {}", e)
                    }],
                    "isError": true
                }),
            }
        },
        "forge_variance" => {
            let budget_path = arguments
                .get("budget_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let actual_path = arguments
                .get("actual_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let threshold = arguments
                .get("threshold")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(10.0);

            let budget = Path::new(budget_path);
            let actual = Path::new(actual_path);
            match variance_core(budget, actual, threshold) {
                Ok(result) => json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&result).unwrap_or_default()
                    }],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Variance analysis failed: {}", e)
                    }],
                    "isError": true
                }),
            }
        },
        "forge_compare" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let scenarios: Vec<String> = arguments
                .get("scenarios")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let path = Path::new(file_path);
            match compare_core(path, &scenarios) {
                Ok(result) => json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&result).unwrap_or_default()
                    }],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Scenario comparison failed: {}", e)
                    }],
                    "isError": true
                }),
            }
        },
        // v10.0.0-beta.1 Analysis Tools
        "forge_simulate" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            // MCP iteration counts are always small, saturating cast is safe
            #[allow(clippy::cast_possible_truncation)]
            let iterations = arguments
                .get("iterations")
                .and_then(serde_json::Value::as_u64)
                .map(|n| n as usize);
            let seed = arguments.get("seed").and_then(serde_json::Value::as_u64);
            let sampling = arguments.get("sampling").and_then(|v| v.as_str());

            let path = Path::new(file_path);
            match simulate_core(path, iterations, seed, sampling) {
                Ok(result) => {
                    let text = result
                        .to_json()
                        .unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"));
                    json!({
                        "content": [{"type": "text", "text": text}],
                        "isError": false
                    })
                },
                Err(e) => json!({
                    "content": [{"type": "text", "text": format!("Simulation failed: {}", e)}],
                    "isError": true
                }),
            }
        },
        "forge_scenarios" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let scenario_filter = arguments.get("scenario_filter").and_then(|v| v.as_str());

            let path = Path::new(file_path);
            match scenarios_core(path, scenario_filter) {
                Ok(result) => json!({
                    "content": [{"type": "text", "text": serde_json::to_string(&result).unwrap_or_default()}],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{"type": "text", "text": format!("Scenario analysis failed: {}", e)}],
                    "isError": true
                }),
            }
        },
        "forge_decision_tree" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let path = Path::new(file_path);
            match decision_tree_core(path) {
                Ok(result) => json!({
                    "content": [{"type": "text", "text": serde_json::to_string(&result).unwrap_or_default()}],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{"type": "text", "text": format!("Decision tree analysis failed: {}", e)}],
                    "isError": true
                }),
            }
        },
        "forge_real_options" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let path = Path::new(file_path);
            match real_options_core(path) {
                Ok(result) => json!({
                    "content": [{"type": "text", "text": serde_json::to_string(&result).unwrap_or_default()}],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{"type": "text", "text": format!("Real options analysis failed: {}", e)}],
                    "isError": true
                }),
            }
        },
        "forge_tornado" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let output_var = arguments.get("output_var").and_then(|v| v.as_str());

            let path = Path::new(file_path);
            match tornado_core(path, output_var) {
                Ok(result) => json!({
                    "content": [{"type": "text", "text": serde_json::to_string(&result).unwrap_or_default()}],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{"type": "text", "text": format!("Tornado analysis failed: {}", e)}],
                    "isError": true
                }),
            }
        },
        "forge_bootstrap" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            #[allow(clippy::cast_possible_truncation)]
            let iterations = arguments
                .get("iterations")
                .and_then(serde_json::Value::as_u64)
                .map(|n| n as usize);
            let seed = arguments.get("seed").and_then(serde_json::Value::as_u64);
            let confidence_levels: Option<Vec<f64>> = arguments
                .get("confidence_levels")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(serde_json::Value::as_f64).collect());

            let path = Path::new(file_path);
            match bootstrap_core(path, iterations, seed, confidence_levels) {
                Ok(result) => json!({
                    "content": [{"type": "text", "text": serde_json::to_string(&result).unwrap_or_default()}],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{"type": "text", "text": format!("Bootstrap analysis failed: {}", e)}],
                    "isError": true
                }),
            }
        },
        "forge_bayesian" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let query_var = arguments.get("query_var").and_then(|v| v.as_str());
            let evidence: Vec<String> = arguments
                .get("evidence")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let path = Path::new(file_path);
            match bayesian_core(path, query_var, &evidence) {
                Ok(result) => json!({
                    "content": [{"type": "text", "text": serde_json::to_string(&result).unwrap_or_default()}],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{"type": "text", "text": format!("Bayesian inference failed: {}", e)}],
                    "isError": true
                }),
            }
        },
        // v10.0.0-beta.1 Discovery Tools
        "forge_schema" => {
            let version = arguments.get("version").and_then(|v| v.as_str());

            match schema_core(version) {
                Ok(text) => json!({
                    "content": [{"type": "text", "text": text}],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{"type": "text", "text": format!("Schema error: {}", e)}],
                    "isError": true
                }),
            }
        },
        "forge_functions" => match functions_core() {
            Ok(result) => json!({
                "content": [{"type": "text", "text": serde_json::to_string(&result).unwrap_or_default()}],
                "isError": false
            }),
            Err(e) => json!({
                "content": [{"type": "text", "text": format!("Functions list failed: {}", e)}],
                "isError": true
            }),
        },
        "forge_examples" => {
            let name = arguments.get("name").and_then(|v| v.as_str());

            match examples_core(name) {
                Ok(result) => json!({
                    "content": [{"type": "text", "text": serde_json::to_string(&result).unwrap_or_default()}],
                    "isError": false
                }),
                Err(e) => json!({
                    "content": [{"type": "text", "text": format!("Examples error: {}", e)}],
                    "isError": true
                }),
            }
        },
        _ => json!({
            "content": [{
                "type": "text",
                "text": format!("Unknown tool: {}", name)
            }],
            "isError": true
        }),
    }
}

/// Forge MCP Server struct
pub struct ForgeMcpServer;

impl ForgeMcpServer {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for ForgeMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ═══════════════════════════════════════════════════════════════════════
    // JSON-RPC REQUEST HANDLING TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_initialize_request() {
        let request = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: json!({}),
        };

        let response = handle_request(&request).unwrap();
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, json!(1));
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "forge");
    }

    #[test]
    fn test_initialize_without_id() {
        let request = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: None,
            method: "initialize".to_string(),
            params: json!({}),
        };

        let response = handle_request(&request).unwrap();
        assert_eq!(response.id, Value::Null);
    }

    #[test]
    fn test_tools_list_request() {
        let request = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: json!({}),
        };

        let response = handle_request(&request).unwrap();
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 20); // 5 core + 5 financial + 7 analysis + 3 discovery

        // Check tool names - core tools
        let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(tool_names.contains(&"forge_validate"));
        assert!(tool_names.contains(&"forge_calculate"));
        assert!(tool_names.contains(&"forge_audit"));
        assert!(tool_names.contains(&"forge_export"));
        assert!(tool_names.contains(&"forge_import"));
        // v3.0.0 financial analysis tools
        assert!(tool_names.contains(&"forge_sensitivity"));
        assert!(tool_names.contains(&"forge_goal_seek"));
        assert!(tool_names.contains(&"forge_break_even"));
        assert!(tool_names.contains(&"forge_variance"));
        assert!(tool_names.contains(&"forge_compare"));
    }

    #[test]
    fn test_ping_request() {
        let request = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "ping".to_string(),
            params: json!({}),
        };

        let response = handle_request(&request).unwrap();
        assert!(response.error.is_none());
        assert_eq!(response.result, Some(json!({})));
    }

    #[test]
    fn test_notification_no_response() {
        let request = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: None,
            method: "notifications/initialized".to_string(),
            params: json!({}),
        };

        let response = handle_request(&request);
        assert!(response.is_none());
    }

    #[test]
    fn test_unknown_method_error() {
        let request = JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(json!(4)),
            method: "unknown/method".to_string(),
            params: json!({}),
        };

        let response = handle_request(&request).unwrap();
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32601);
        assert!(error.message.contains("Method not found"));
    }

    #[test]
    fn test_unknown_tool_call() {
        let result = call_tool("unknown_tool", &json!({}));
        assert!(result["isError"].as_bool().unwrap());
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Unknown tool"));
    }

    #[test]
    fn test_get_tools_has_correct_schemas() {
        let tools = get_tools();
        assert_eq!(tools.len(), 20); // 5 core + 5 financial + 7 analysis + 3 discovery

        // Validate forge_validate schema
        let validate_tool = tools.iter().find(|t| t.name == "forge_validate").unwrap();
        let schema = &validate_tool.input_schema;
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["file_path"].is_object());

        // Validate forge_audit schema
        let audit_tool = tools.iter().find(|t| t.name == "forge_audit").unwrap();
        let required = audit_tool.input_schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("file_path")));
        assert!(required.contains(&json!("variable")));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // TOOL CALL TESTS WITH FIXTURES
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_call_tool_validate_success() {
        let result = call_tool(
            "forge_validate",
            &json!({
                "file_path": "test-data/budget.yaml"
            }),
        );
        // May succeed or fail based on file state, but should not be unknown tool
        assert!(!result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Unknown tool"));
    }

    #[test]
    fn test_call_tool_validate_nonexistent() {
        let result = call_tool(
            "forge_validate",
            &json!({
                "file_path": "nonexistent.yaml"
            }),
        );
        assert!(result["isError"].as_bool().unwrap());
    }

    #[test]
    fn test_call_tool_calculate_dry_run() {
        let result = call_tool(
            "forge_calculate",
            &json!({
                "file_path": "test-data/budget.yaml",
                "dry_run": true
            }),
        );
        // Dry run should succeed
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(!text.contains("Unknown tool"));
    }

    #[test]
    fn test_call_tool_calculate_nonexistent() {
        let result = call_tool(
            "forge_calculate",
            &json!({
                "file_path": "nonexistent.yaml",
                "dry_run": true
            }),
        );
        assert!(result["isError"].as_bool().unwrap());
    }

    #[test]
    fn test_call_tool_audit_with_variable() {
        let result = call_tool(
            "forge_audit",
            &json!({
                "file_path": "test-data/budget.yaml",
                "variable": "assumptions.profit"
            }),
        );
        // May succeed or fail, but should process correctly
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(!text.contains("Unknown tool"));
    }

    #[test]
    fn test_call_tool_export() {
        let temp_dir = TempDir::new().unwrap();
        let output = temp_dir.path().join("mcp_test_export.xlsx");

        let result = call_tool(
            "forge_export",
            &json!({
                "yaml_path": "test-data/budget.yaml",
                "excel_path": output.to_str().unwrap()
            }),
        );
        assert!(!result["isError"].as_bool().unwrap_or(true));
    }

    #[test]
    fn test_call_tool_import() {
        let temp_dir = TempDir::new().unwrap();
        let excel_path = temp_dir.path().join("import_test.xlsx");
        let yaml_path = temp_dir.path().join("imported.yaml");

        // First export to create Excel file
        call_tool(
            "forge_export",
            &json!({
                "yaml_path": "test-data/budget.yaml",
                "excel_path": excel_path.to_str().unwrap()
            }),
        );

        // Then import
        let result = call_tool(
            "forge_import",
            &json!({
                "excel_path": excel_path.to_str().unwrap(),
                "yaml_path": yaml_path.to_str().unwrap()
            }),
        );
        assert!(!result["isError"].as_bool().unwrap_or(true));
    }

    #[test]
    fn test_call_tool_sensitivity() {
        let result = call_tool(
            "forge_sensitivity",
            &json!({
                "file_path": "test-data/sensitivity_test.yaml",
                "vary": "price",
                "range": "80,120,10",
                "output": "profit"
            }),
        );
        // May succeed or fail based on file structure
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(!text.contains("Unknown tool"));
    }

    #[test]
    fn test_call_tool_sensitivity_two_var() {
        let result = call_tool(
            "forge_sensitivity",
            &json!({
                "file_path": "test-data/sensitivity_test.yaml",
                "vary": "price",
                "range": "80,120,10",
                "vary2": "quantity",
                "range2": "100,200,50",
                "output": "profit"
            }),
        );
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(!text.contains("Unknown tool"));
    }

    #[test]
    fn test_call_tool_goal_seek() {
        let result = call_tool(
            "forge_goal_seek",
            &json!({
                "file_path": "test-data/budget.yaml",
                "target": "assumptions.profit",
                "value": 0.0,
                "vary": "assumptions.revenue",
                "min": 50_000,
                "max": 200_000,
                "tolerance": 0.01
            }),
        );
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(!text.contains("Unknown tool"));
    }

    #[test]
    fn test_call_tool_break_even() {
        let result = call_tool(
            "forge_break_even",
            &json!({
                "file_path": "test-data/budget.yaml",
                "output": "assumptions.profit",
                "vary": "assumptions.revenue",
                "min": 50_000,
                "max": 200_000
            }),
        );
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(!text.contains("Unknown tool"));
    }

    #[test]
    fn test_call_tool_variance() {
        let result = call_tool(
            "forge_variance",
            &json!({
                "budget_path": "test-data/budget.yaml",
                "actual_path": "test-data/budget.yaml",
                "threshold": 10.0
            }),
        );
        assert!(!result["isError"].as_bool().unwrap_or(true));
    }

    #[test]
    fn test_call_tool_compare() {
        let result = call_tool(
            "forge_compare",
            &json!({
                "file_path": "test-data/budget.yaml",
                "scenarios": ["base", "optimistic"]
            }),
        );
        // Expected to fail - no scenarios in budget.yaml
        assert!(result["isError"].as_bool().unwrap_or(false));
    }

    #[test]
    fn test_call_tool_compare_empty_scenarios() {
        let result = call_tool(
            "forge_compare",
            &json!({
                "file_path": "test-data/budget.yaml",
                "scenarios": []
            }),
        );
        // May fail with no scenarios
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(!text.contains("Unknown tool"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // v10.0.0-beta.1 ANALYSIS TOOL TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_call_tool_simulate() {
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

        let result = call_tool(
            "forge_simulate",
            &json!({"file_path": file.to_str().unwrap()}),
        );
        assert!(
            !result["isError"].as_bool().unwrap_or(true),
            "simulate error: {}",
            result["content"][0]["text"]
        );
        let text = result["content"][0]["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert!(
            parsed["monte_carlo_results"]["iterations"]
                .as_u64()
                .unwrap()
                > 0
        );
    }

    #[test]
    fn test_call_tool_simulate_nonexistent() {
        let result = call_tool("forge_simulate", &json!({"file_path": "nonexistent.yaml"}));
        assert!(result["isError"].as_bool().unwrap());
    }

    #[test]
    fn test_call_tool_scenarios_dispatch() {
        // Test tool dispatch and argument parsing — no valid scenario fixture
        let result = call_tool(
            "forge_scenarios",
            &json!({"file_path": "test-data/budget.yaml"}),
        );
        // budget.yaml has no scenarios section, should return an error (not "Unknown tool")
        assert!(result["isError"].as_bool().unwrap());
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("scenarios"));
        assert!(!text.contains("Unknown tool"));
    }

    #[test]
    fn test_call_tool_scenarios_nonexistent() {
        let result = call_tool("forge_scenarios", &json!({"file_path": "nonexistent.yaml"}));
        assert!(result["isError"].as_bool().unwrap());
    }

    #[test]
    fn test_call_tool_decision_tree() {
        let result = call_tool(
            "forge_decision_tree",
            &json!({"file_path": "examples/decision-tree.yaml"}),
        );
        assert!(!result["isError"].as_bool().unwrap_or(true));
        let text = result["content"][0]["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert!(parsed["optimal_path"].as_array().is_some());
    }

    #[test]
    fn test_call_tool_real_options() {
        let result = call_tool(
            "forge_real_options",
            &json!({"file_path": "examples/real-options.yaml"}),
        );
        assert!(!result["isError"].as_bool().unwrap_or(true));
        let text = result["content"][0]["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert!(parsed["total_option_value"].as_f64().is_some());
    }

    #[test]
    fn test_call_tool_tornado() {
        let result = call_tool(
            "forge_tornado",
            &json!({"file_path": "examples/tornado.yaml"}),
        );
        assert!(!result["isError"].as_bool().unwrap_or(true));
        let text = result["content"][0]["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert!(parsed["base_value"].as_f64().is_some());
    }

    #[test]
    fn test_call_tool_bootstrap() {
        let result = call_tool(
            "forge_bootstrap",
            &json!({"file_path": "examples/bootstrap.yaml", "seed": 42}),
        );
        assert!(!result["isError"].as_bool().unwrap_or(true));
        let text = result["content"][0]["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert!(parsed["original_estimate"].as_f64().is_some());
    }

    #[test]
    fn test_call_tool_bayesian_dispatch() {
        // Test tool dispatch — budget.yaml has no bayesian_network section
        let result = call_tool(
            "forge_bayesian",
            &json!({"file_path": "test-data/budget.yaml"}),
        );
        assert!(result["isError"].as_bool().unwrap());
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("bayesian_network"));
        assert!(!text.contains("Unknown tool"));
    }

    #[test]
    fn test_call_tool_bayesian_with_evidence_dispatch() {
        let result = call_tool(
            "forge_bayesian",
            &json!({
                "file_path": "test-data/budget.yaml",
                "evidence": ["economy=growth"]
            }),
        );
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(!text.contains("Unknown tool"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // v10.0.0-beta.1 DISCOVERY TOOL TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_call_tool_schema_v5() {
        let result = call_tool("forge_schema", &json!({"version": "v5"}));
        assert!(!result["isError"].as_bool().unwrap_or(true));
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("$schema"));
    }

    #[test]
    fn test_call_tool_schema_v1() {
        let result = call_tool("forge_schema", &json!({"version": "v1"}));
        assert!(!result["isError"].as_bool().unwrap_or(true));
    }

    #[test]
    fn test_call_tool_schema_list() {
        let result = call_tool("forge_schema", &json!({}));
        assert!(!result["isError"].as_bool().unwrap_or(true));
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("available_versions"));
    }

    #[test]
    fn test_call_tool_schema_invalid() {
        let result = call_tool("forge_schema", &json!({"version": "v99"}));
        assert!(result["isError"].as_bool().unwrap());
    }

    #[test]
    fn test_call_tool_functions() {
        let result = call_tool("forge_functions", &json!({}));
        assert!(!result["isError"].as_bool().unwrap_or(true));
        let text = result["content"][0]["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["edition"], "enterprise");
        assert!(parsed["total"].as_u64().unwrap() >= 170);
    }

    #[test]
    fn test_call_tool_examples_list() {
        let result = call_tool("forge_examples", &json!({}));
        assert!(!result["isError"].as_bool().unwrap_or(true));
        let text = result["content"][0]["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert!(parsed.as_array().is_some_and(|a| a.len() >= 9));
    }

    #[test]
    fn test_call_tool_examples_specific() {
        let result = call_tool("forge_examples", &json!({"name": "monte-carlo"}));
        assert!(!result["isError"].as_bool().unwrap_or(true));
        let text = result["content"][0]["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["name"], "monte-carlo");
        assert!(parsed["content"]
            .as_str()
            .is_some_and(|c| c.contains("monte_carlo")));
    }

    #[test]
    fn test_call_tool_examples_unknown() {
        let result = call_tool("forge_examples", &json!({"name": "nonexistent"}));
        assert!(result["isError"].as_bool().unwrap());
    }

    // ═══════════════════════════════════════════════════════════════════════
    // STRUCTURED RESULT VERIFICATION TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_validate_returns_structured_json() {
        let result = call_tool(
            "forge_validate",
            &json!({"file_path": "test-data/budget.yaml"}),
        );
        assert!(!result["isError"].as_bool().unwrap_or(true));
        let text = result["content"][0]["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert!(parsed.get("tables_valid").is_some());
        assert!(parsed.get("scalars_valid").is_some());
        assert!(parsed.get("table_count").is_some());
    }

    #[test]
    fn test_calculate_returns_structured_json() {
        let result = call_tool(
            "forge_calculate",
            &json!({"file_path": "test-data/budget.yaml", "dry_run": true}),
        );
        assert!(!result["isError"].as_bool().unwrap_or(true));
        let text = result["content"][0]["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert!(parsed.get("tables").is_some());
        assert!(parsed.get("scalars").is_some());
        assert_eq!(parsed["dry_run"], true);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // JSON-RPC RESPONSE STRUCT TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_jsonrpc_response_serialization() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: Some(json!({"status": "ok"})),
            error: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_jsonrpc_response_with_error() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            result: None,
            error: Some(JsonRpcError {
                code: -32600,
                message: "Invalid Request".to_string(),
                data: None,
            }),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"error\""));
        assert!(json.contains("-32600"));
    }

    #[test]
    fn test_jsonrpc_error_with_data() {
        let error = JsonRpcError {
            code: -32000,
            message: "Server error".to_string(),
            data: Some(json!({"details": "more info"})),
        };
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"data\""));
        assert!(json.contains("more info"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // TOOL STRUCT TESTS
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_tool_serialization() {
        let tool = Tool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: json!({"type": "object"}),
        };
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("\"name\":\"test_tool\""));
        assert!(json.contains("\"inputSchema\""));
    }
}
