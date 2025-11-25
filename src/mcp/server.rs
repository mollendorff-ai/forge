//! Forge MCP Server implementation
//!
//! Provides the MCP server that AI agents use to interact with Forge.
//! Implements the Model Context Protocol over stdin/stdout using JSON-RPC.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::cli::{audit, calculate, export, import, validate};

/// JSON-RPC request
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
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// MCP Tool definition
#[derive(Debug, Serialize)]
struct Tool {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
}

/// Run the MCP server synchronously over stdin/stdout
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
                        message: format!("Parse error: {}", e),
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
            }
        };

        let response = handle_request(&request);

        if let Some(resp) = response {
            let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
            let _ = stdout.flush();
        }
    }
}

/// Handle a JSON-RPC request
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
                "instructions": "Forge MCP Server - YAML formula calculator with Excel-style arrays. Validate financial models, calculate formulas, audit dependencies, import/export Excel. Supports 50+ functions including NPV, IRR, PMT."
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
        }
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
    ]
}

/// Call a tool by name
fn call_tool(name: &str, arguments: &Value) -> Value {
    match name {
        "forge_validate" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let path = Path::new(file_path).to_path_buf();
            match validate(path) {
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
        }
        "forge_calculate" => {
            let file_path = arguments
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let dry_run = arguments
                .get("dry_run")
                .and_then(|v| v.as_bool())
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
                            format!("Dry run completed for {}", file_path)
                        } else {
                            format!("Calculation completed and file updated: {}", file_path)
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
        }
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
        }
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
        }
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
            match import(excel, yaml, false) {
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
        }
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
        assert_eq!(tools.len(), 5);

        // Check tool names
        let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(tool_names.contains(&"forge_validate"));
        assert!(tool_names.contains(&"forge_calculate"));
        assert!(tool_names.contains(&"forge_audit"));
        assert!(tool_names.contains(&"forge_export"));
        assert!(tool_names.contains(&"forge_import"));
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
        assert_eq!(tools.len(), 5);

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
}
