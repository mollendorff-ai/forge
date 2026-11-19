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
    use crate::mcp::server::tests::{close, example, json, ok_text};
    use crate::mcp::types::{CalculateRequest, ValidateRequest};
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

    // ─────────────────────────────────────────────────────────────────────────
    // The discovery tools run no model; what they say is checked against the
    // models that use it (T1-INTERFACE-PARITY): the schema the tool serves
    // accepts examples/bagel, the function list has the functions it calls, and
    // the example the tool serves for a first model calculates to its numbers.
    // ─────────────────────────────────────────────────────────────────────────

    /// Model format 5.4.0, the format of examples/bagel/bagel.yaml, as the tool
    /// serves it: the plan fits it and a probability that is not a number does not.
    #[test]
    fn the_schema_the_tool_serves_accepts_the_bagel_plan() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.schema(Parameters(SchemaRequest {
            version: Some("v5.4".into()),
        })));
        let validator = jsonschema::validator_for(&json(&text)).unwrap();
        let plan = example("bagel", "bagel.yaml");
        assert!(plan.contains("_forge_version: \"5.4.0\""));
        let mut model =
            serde_json::to_value(yaml_serde::from_str::<yaml_serde::Value>(&plan).unwrap())
                .unwrap();
        assert!(validator.is_valid(&model));
        model["scenarios"]["base"]["probability"] = "half".into();
        assert!(!validator.is_valid(&model));
    }

    /// The model formats the tool lists include the one bagel is written in.
    #[test]
    fn the_formats_the_tool_lists_include_the_bagel_plans() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.schema(Parameters(SchemaRequest { version: None })));
        let versions = json(&text)["available_versions"].clone();
        assert!(versions
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["version"] == "v5.4.0"));
    }

    /// SUM, MIN and NPV are what the bagel plan calls (`year.profit`,
    /// `year.worst_month_profit`, `year.npv_first_year`).
    #[test]
    fn the_function_list_has_the_functions_the_bagel_plan_calls() {
        let server = ForgeMcpServer::new();
        let listing = json(&ok_text(server.functions(Parameters(FunctionsRequest {}))));
        let names: Vec<&str> = listing["categories"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|c| c["functions"].as_array().unwrap())
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        assert_eq!(names.len(), 203);
        let plan = example("bagel", "bagel.yaml");
        for function in ["SUM", "MIN", "NPV"] {
            assert!(names.contains(&function), "{function}");
            assert!(plan.contains(&format!("{function}(")), "{function}");
        }
    }

    /// `starter`, the first model the tool offers, with the numbers of its own
    /// arithmetic: 40 x 120 - 3,000; 20 x 200 + 40 x 120 + 80 x 30; the largest
    /// of 200, 120 and 30 units. Its content is the model, as inline content.
    #[test]
    fn the_starter_example_the_tool_serves_calculates() {
        let server = ForgeMcpServer::new();
        let example = json(&ok_text(server.examples(Parameters(ExamplesRequest {
            name: Some("starter".into()),
        }))));
        assert_eq!(example["command"], "forge calculate");
        let result = json(&ok_text(server.calculate(Parameters(CalculateRequest {
            file_path: None,
            content: example["content"].as_str().map(Into::into),
            includes: None,
            dry_run: true,
            scenario: None,
        }))));
        let scalar = |name: &str| result["scalars"][name].as_f64().unwrap();
        assert!(close(scalar("plan.profit"), 1800.0, 1e-9));
        assert!(close(scalar("totals.revenue"), 11_200.0, 1e-9));
        assert!(close(scalar("totals.best_seller_units"), 200.0, 1e-9));
        let valid = json(&ok_text(server.validate(Parameters(ValidateRequest {
            file_path: None,
            content: example["content"].as_str().map(Into::into),
            includes: None,
            verbose: false,
        }))));
        assert_eq!(valid["mismatches"], serde_json::json!([]));
    }
}
