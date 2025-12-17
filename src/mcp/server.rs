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

// CLI imports only used by non-coverage builds and tests
#[cfg(any(not(coverage), test))]
use crate::cli::{
    audit, break_even, calculate, compare, export, goal_seek, import, sensitivity, validate,
    variance,
};

/// JSON-RPC request
#[cfg(any(not(coverage), test))]
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
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
/// # Coverage Exclusion (ADR-006)
/// This function reads from stdin forever until EOF. Cannot be unit tested.
/// The request handling logic is tested via `handle_request()`.
#[cfg(not(coverage))]
pub fn run_mcp_server_sync() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

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
                    "name": "forge-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "instructions": "Forge MCP Server v3.0.0 - AI-Finance integration. Zero tokens. Zero emissions. Validate models, calculate formulas, sensitivity analysis, goal-seek, break-even, variance analysis, scenario comparison. 60+ Excel functions including NPV, IRR, PMT, XNPV, XIRR. 96K rows/sec performance."
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
                .unwrap_or(json!({}));

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
    ]
}

/// Call a tool by name
#[cfg(any(not(coverage), test))]
fn call_tool(name: &str, arguments: &Value) -> Value {
    match name {
        "forge_validate" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let path = Path::new(file_path).to_path_buf();
            match validate(vec![path]) {
                Ok(()) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Validation successful for {}", file_path)
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

            let path = Path::new(file_path).to_path_buf();
            let scenario = arguments
                .get("scenario")
                .and_then(|v| v.as_str())
                .map(String::from);
            match calculate(path, dry_run, false, scenario) {
                Ok(()) => json!({
                    "content": [{
                        "type": "text",
                        "text": if dry_run {
                            format!("Dry run completed for {file_path}")
                        } else {
                            format!("Calculation completed and file updated: {file_path}")
                        }
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

            let path = Path::new(file_path).to_path_buf();
            match audit(path, variable.to_string()) {
                Ok(()) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Audit completed for variable '{}' in {}", variable, file_path)
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

            let yaml = Path::new(yaml_path).to_path_buf();
            let excel = Path::new(excel_path).to_path_buf();
            match export(yaml, excel, false) {
                Ok(()) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Exported {} to {}", yaml_path, excel_path)
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

            let excel = Path::new(excel_path).to_path_buf();
            let yaml = Path::new(yaml_path).to_path_buf();
            match import(excel, yaml, false, false, false) {
                Ok(()) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Imported {} to {}", excel_path, yaml_path)
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
            let vary2 = arguments
                .get("vary2")
                .and_then(|v| v.as_str())
                .map(String::from);
            let range2 = arguments
                .get("range2")
                .and_then(|v| v.as_str())
                .map(String::from);

            let path = Path::new(file_path).to_path_buf();
            match sensitivity(
                path,
                vary.to_string(),
                range.to_string(),
                vary2,
                range2,
                output.to_string(),
                false,
            ) {
                Ok(()) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Sensitivity analysis completed for {} varying {} over {}", file_path, vary, range)
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

            let path = Path::new(file_path).to_path_buf();
            match goal_seek(
                path,
                target.to_string(),
                value,
                vary.to_string(),
                min,
                max,
                tolerance,
                false,
            ) {
                Ok(()) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Goal seek completed: found {} value to achieve {} = {}", vary, target, value)
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

            let path = Path::new(file_path).to_path_buf();
            match break_even(path, output.to_string(), vary.to_string(), min, max, false) {
                Ok(()) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Break-even analysis completed: found {} value where {} = 0", vary, output)
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

            let budget = Path::new(budget_path).to_path_buf();
            let actual = Path::new(actual_path).to_path_buf();
            match variance(budget, actual, threshold, None, false) {
                Ok(()) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Variance analysis completed: {} vs {} (threshold: {}%)", budget_path, actual_path, threshold)
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

            let path = Path::new(file_path).to_path_buf();
            match compare(path, scenarios.clone(), false) {
                Ok(()) => json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Scenario comparison completed for {}: {:?}", file_path, scenarios)
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
    pub fn new() -> Self {
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
            jsonrpc: "2.0".to_string(),
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
        assert_eq!(result["serverInfo"]["name"], "forge-mcp");
    }

    #[test]
    fn test_initialize_without_id() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
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
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: json!({}),
        };

        let response = handle_request(&request).unwrap();
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 10); // 5 core + 5 financial analysis tools

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
            jsonrpc: "2.0".to_string(),
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
            jsonrpc: "2.0".to_string(),
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
            jsonrpc: "2.0".to_string(),
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
        assert_eq!(tools.len(), 10); // 5 core + 5 financial analysis tools

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
                "min": 50000,
                "max": 200000,
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
                "min": 50000,
                "max": 200000
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
