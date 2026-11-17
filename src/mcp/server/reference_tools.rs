//! The tools that describe Forge itself: the model schema, the functions
//! and the bundled examples. They read no model, so they do not use the
//! server's folder.

use super::{to_json, ForgeMcpServer};
use crate::mcp::types::{ExamplesRequest, FunctionsRequest, SchemaRequest};

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::cli::{examples_core, functions_core, schema_core};

#[tool_router(router = reference_tools, vis = "pub(super)")]
#[allow(clippy::unused_self)] // rmcp #[tool] macro requires &self on all tool methods
impl ForgeMcpServer {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::server::tests::ok_text;
    use rmcp::handler::server::wrapper::Parameters;

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
        assert!(
            parsed.get("edition").is_none(),
            "there is no enterprise edition"
        );
        assert_eq!(parsed["total"].as_u64().unwrap(), 203);
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
}
