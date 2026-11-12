//! Tests for the MCP server (`server.rs`), kept apart to keep files short.

use super::*;
use rmcp::handler::server::wrapper::Parameters;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper: get Ok text or panic with the error string
fn ok_text(result: Result<String, String>) -> String {
    result.expect("expected Ok result")
}

/// Helper: get Err text or panic
fn err_text(result: Result<String, String>) -> String {
    result.expect_err("expected Err result")
}

/// Minimal valid YAML model for inline content tests
const INLINE_YAML: &str = r#"
_forge_version: "5.0.0"
scalars:
  revenue:
    value: 100000
  costs:
    value: 60000
  profit:
    value: 0
    formula: "=revenue - costs"
"#;

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
        file_path: Some("test-data/budget.yaml".into()),
        content: None,
        includes: None,
        verbose: false,
    }));
    assert!(result.is_ok());
}

#[test]
fn test_call_validate_nonexistent() {
    let server = ForgeMcpServer::new();
    let result = server.validate(Parameters(ValidateRequest {
        file_path: Some("nonexistent.yaml".into()),
        content: None,
        includes: None,
        verbose: false,
    }));
    assert!(result.is_err());
}

#[test]
fn test_call_calculate_dry_run() {
    let server = ForgeMcpServer::new();
    let result = server.calculate(Parameters(CalculateRequest {
        file_path: Some("test-data/budget.yaml".into()),
        content: None,
        includes: None,
        dry_run: true,
        scenario: None,
    }));
    assert!(result.is_ok());
}

#[test]
fn test_call_calculate_nonexistent() {
    let server = ForgeMcpServer::new();
    let result = server.calculate(Parameters(CalculateRequest {
        file_path: Some("nonexistent.yaml".into()),
        content: None,
        includes: None,
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
        file_path: Some("test-data/budget.yaml".into()),
        content: None,
        includes: None,
        variable: "assumptions.profit".into(),
    }));
}

#[test]
fn test_call_export() {
    let temp_dir = TempDir::new().unwrap();
    let output = temp_dir.path().join("mcp_test_export.xlsx");

    let server = ForgeMcpServer::new();
    let result = server.export(Parameters(ExportRequest {
        yaml_path: Some("test-data/budget.yaml".into()),
        content: None,
        includes: None,
        excel_path: Some(output.to_str().unwrap().into()),
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
        yaml_path: Some("test-data/budget.yaml".into()),
        content: None,
        includes: None,
        excel_path: Some(excel_path.to_str().unwrap().into()),
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
        file_path: Some("test-data/sensitivity_test.yaml".into()),
        content: None,
        includes: None,
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
        file_path: Some("test-data/sensitivity_test.yaml".into()),
        content: None,
        includes: None,
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
        file_path: Some("test-data/budget.yaml".into()),
        content: None,
        includes: None,
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
        file_path: Some("test-data/budget.yaml".into()),
        content: None,
        includes: None,
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
        budget_path: Some("test-data/budget.yaml".into()),
        budget_content: None,
        actual_path: Some("test-data/budget.yaml".into()),
        actual_content: None,
        includes: None,
        threshold: Some(10.0),
    }));
    assert!(result.is_ok());
}

#[test]
fn test_call_compare() {
    let server = ForgeMcpServer::new();
    let result = server.compare(Parameters(CompareRequest {
        file_path: Some("test-data/budget.yaml".into()),
        content: None,
        includes: None,
        scenarios: vec!["base".into(), "optimistic".into()],
    }));
    // Expected to fail - no scenarios in budget.yaml
    assert!(result.is_err());
}

#[test]
fn test_call_compare_empty_scenarios() {
    let server = ForgeMcpServer::new();
    let _ = server.compare(Parameters(CompareRequest {
        file_path: Some("test-data/budget.yaml".into()),
        content: None,
        includes: None,
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
        file_path: Some(file.to_str().unwrap().into()),
        content: None,
        includes: None,
        iterations: None,
        seed: None,
        sampling: None,
        scenario: None,
        seek: None,
        vary: None,
        min: None,
        max: None,
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
        file_path: Some("nonexistent.yaml".into()),
        content: None,
        includes: None,
        iterations: None,
        seed: None,
        sampling: None,
        scenario: None,
        seek: None,
        vary: None,
        min: None,
        max: None,
    }));
    assert!(result.is_err());
}

#[test]
fn test_call_scenarios_dispatch() {
    let server = ForgeMcpServer::new();
    let e = err_text(server.scenarios(Parameters(ScenariosRequest {
        file_path: Some("test-data/budget.yaml".into()),
        content: None,
        includes: None,
        scenario_filter: None,
    })));
    assert!(e.contains("scenarios"));
}

#[test]
fn test_call_scenarios_nonexistent() {
    let server = ForgeMcpServer::new();
    assert!(server
        .scenarios(Parameters(ScenariosRequest {
            file_path: Some("nonexistent.yaml".into()),
            content: None,
            includes: None,
            scenario_filter: None,
        }))
        .is_err());
}

#[test]
fn test_call_scenarios_structured_format() {
    // Validates that the ScenarioDefinition format (probability + description + scalars)
    // is accepted by both the model parser and the scenario engine.
    let content = r#"
_forge_version: "5.0.0"

price:
  value: 100
  formula: null
units:
  value: 50
  formula: null
revenue:
  value: null
  formula: "=price * units"

scenarios:
  high:
    probability: 0.5
    description: "High price scenario"
    scalars:
      price: 200
  low:
    probability: 0.5
    description: "Low price scenario"
    scalars:
      price: 50
"#;
    let server = ForgeMcpServer::new();
    let text = ok_text(server.scenarios(Parameters(ScenariosRequest {
        file_path: None,
        content: Some(content.to_string()),
        includes: None,
        scenario_filter: None,
    })));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    // Both scenarios should appear
    assert!(parsed["scenarios"].as_array().is_some());
    let scenarios = parsed["scenarios"].as_array().unwrap();
    assert_eq!(scenarios.len(), 2);
    // Expected value should be probability-weighted: 0.5*10000 + 0.5*2500 = 6250
    let ev = parsed["expected_values"]["revenue"].as_f64().unwrap();
    assert!((ev - 6250.0).abs() < 1.0, "Expected ~6250, got {ev}");
}

#[test]
fn test_call_compare_structured_format() {
    // Validates that forge_compare works with the ScenarioDefinition format.
    let content = r#"
_forge_version: "5.0.0"

price:
  value: 100
  formula: null
units:
  value: 50
  formula: null
revenue:
  value: null
  formula: "=price * units"

scenarios:
  high:
    probability: 0.5
    description: "High price"
    scalars:
      price: 200
  low:
    probability: 0.5
    description: "Low price"
    scalars:
      price: 50
"#;
    let server = ForgeMcpServer::new();
    let text = ok_text(server.compare(Parameters(CompareRequest {
        file_path: None,
        content: Some(content.to_string()),
        includes: None,
        scenarios: vec!["high".to_string(), "low".to_string()],
    })));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    // Both scenarios should appear in results
    assert!(parsed["scenarios"].as_array().is_some());
    let scenarios = parsed["scenarios"].as_array().unwrap();
    assert_eq!(scenarios.len(), 2);
}

#[test]
fn test_call_scenarios_grouped_scalar_override() {
    // Bug 1: scenario overrides must resolve short names to grouped paths.
    // "growth_rate" in the scenario should override "assumptions.growth_rate".
    let content = r#"
_forge_version: "5.0.0"

assumptions:
  growth_rate:
    value: 0.10
    formula: null
  revenue:
    value: 1000000
    formula: null

profit:
  value: null
  formula: "=assumptions.revenue * assumptions.growth_rate"

scenarios:
  bull:
    probability: 0.5
    scalars:
      growth_rate: 0.25
  bear:
    probability: 0.5
    scalars:
      growth_rate: 0.02
"#;
    let server = ForgeMcpServer::new();
    let text = ok_text(server.scenarios(Parameters(ScenariosRequest {
        file_path: None,
        content: Some(content.to_string()),
        includes: None,
        scenario_filter: None,
    })));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    let scenarios = parsed["scenarios"].as_array().unwrap();
    // Verify each scenario actually overrode the grouped scalar
    for s in scenarios {
        let name = s["name"].as_str().unwrap();
        let profit = s["scalars"]["profit"].as_f64().unwrap();
        match name {
            "bull" => assert!(
                (profit - 250_000.0).abs() < 1.0,
                "bull profit: expected 250000, got {profit}"
            ),
            "bear" => assert!(
                (profit - 20_000.0).abs() < 1.0,
                "bear profit: expected 20000, got {profit}"
            ),
            _ => panic!("unexpected scenario: {name}"),
        }
    }
}

#[test]
fn test_call_compare_grouped_scalar_override() {
    // forge_compare with grouped scalars: short name resolves to group.name
    let content = r#"
_forge_version: "5.0.0"

assumptions:
  growth_rate:
    value: 0.10
    formula: null
  revenue:
    value: 1000000
    formula: null

profit:
  value: null
  formula: "=assumptions.revenue * assumptions.growth_rate"

scenarios:
  bull:
    growth_rate: 0.25
  bear:
    growth_rate: 0.02
"#;
    let server = ForgeMcpServer::new();
    let text = ok_text(server.compare(Parameters(CompareRequest {
        file_path: None,
        content: Some(content.to_string()),
        includes: None,
        scenarios: vec!["bull".to_string(), "bear".to_string()],
    })));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    let values = &parsed["values"];
    // profit should differ between scenarios (overrides reached the grouped variable)
    let bull_profit = values["profit"]["bull"].as_f64().unwrap();
    let bear_profit = values["profit"]["bear"].as_f64().unwrap();
    assert!(
        (bull_profit - 250_000.0).abs() < 1.0,
        "bull profit: expected 250000, got {bull_profit}"
    );
    assert!(
        (bear_profit - 20_000.0).abs() < 1.0,
        "bear profit: expected 20000, got {bear_profit}"
    );
}

#[test]
fn test_call_decision_tree() {
    let server = ForgeMcpServer::new();
    let text = ok_text(server.decision_tree(Parameters(DecisionTreeRequest {
        file_path: Some("examples/decision-tree.yaml".into()),
        content: None,
        includes: None,
    })));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["optimal_path"].as_array().is_some());
}

#[test]
fn test_call_real_options() {
    let server = ForgeMcpServer::new();
    let text = ok_text(server.real_options(Parameters(RealOptionsRequest {
        file_path: Some("examples/real-options.yaml".into()),
        content: None,
        includes: None,
    })));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["project_value_with_options"].as_f64().is_some());
    assert!(parsed.get("total_option_value").is_none());
}

#[test]
fn test_call_tornado() {
    let server = ForgeMcpServer::new();
    let text = ok_text(server.tornado(Parameters(TornadoRequest {
        file_path: Some("examples/tornado.yaml".into()),
        content: None,
        includes: None,
        output_var: None,
    })));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["base_value"].as_f64().is_some());
}

#[test]
fn test_call_bootstrap() {
    let server = ForgeMcpServer::new();
    let text = ok_text(server.bootstrap(Parameters(BootstrapRequest {
        file_path: Some("examples/bootstrap.yaml".into()),
        content: None,
        includes: None,
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
        file_path: Some("test-data/budget.yaml".into()),
        content: None,
        includes: None,
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
        file_path: Some("test-data/budget.yaml".into()),
        content: None,
        includes: None,
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
    assert!(
        parsed.get("edition").is_none(),
        "there is no enterprise edition"
    );
    assert_eq!(parsed["total"].as_u64().unwrap(), 202);
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
        file_path: Some("test-data/budget.yaml".into()),
        content: None,
        includes: None,
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
        file_path: Some("test-data/budget.yaml".into()),
        content: None,
        includes: None,
        dry_run: true,
        scenario: None,
    })));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.get("tables").is_some());
    assert!(parsed.get("scalars").is_some());
    assert_eq!(parsed["dry_run"], true);
}

// ═══════════════════════════════════════════════════════════════════════
// INLINE CONTENT TESTS
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn test_resolve_model_input_file_path() {
    let (path, tmpdir) = resolve_model_input(Some("test-data/budget.yaml"), None, None).unwrap();
    assert_eq!(path, PathBuf::from("test-data/budget.yaml"));
    assert!(tmpdir.is_none());
}

#[test]
fn test_resolve_model_input_content() {
    let (path, tmpdir) = resolve_model_input(None, Some(INLINE_YAML), None).unwrap();
    assert!(tmpdir.is_some());
    assert!(path.exists());
    assert!(path.ends_with("model.yaml"));
}

#[test]
fn test_resolve_model_input_both_error() {
    let result = resolve_model_input(Some("file.yaml"), Some("content"), None);
    assert_eq!(
        result.unwrap_err(),
        "Provide either file_path or content, not both"
    );
}

#[test]
fn test_resolve_model_input_neither_error() {
    let result = resolve_model_input(None, None, None);
    assert_eq!(
        result.unwrap_err(),
        "Either file_path or content is required"
    );
}

#[test]
fn test_inline_validate() {
    let server = ForgeMcpServer::new();
    let text = ok_text(server.validate(Parameters(ValidateRequest {
        file_path: None,
        content: Some(INLINE_YAML.into()),
        includes: None,
        verbose: false,
    })));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.get("scalars_valid").is_some());
}

#[test]
fn test_inline_calculate_dry_run() {
    let server = ForgeMcpServer::new();
    let text = ok_text(server.calculate(Parameters(CalculateRequest {
        file_path: None,
        content: Some(INLINE_YAML.into()),
        includes: None,
        dry_run: true,
        scenario: None,
    })));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.get("scalars").is_some());
    assert_eq!(parsed["dry_run"], true);
    // dry_run should not have calculated_content
    assert!(parsed.get("calculated_content").is_none());
}

#[test]
fn test_inline_calculate_write_back() {
    let server = ForgeMcpServer::new();
    let text = ok_text(server.calculate(Parameters(CalculateRequest {
        file_path: None,
        content: Some(INLINE_YAML.into()),
        includes: None,
        dry_run: false,
        scenario: None,
    })));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    // Non-dry-run with inline content should include calculated_content
    assert!(parsed.get("calculated_content").is_some());
    let calculated = parsed["calculated_content"].as_str().unwrap();
    assert!(calculated.contains("profit"));
}

/// Every inline-content example in `docs/MCP.md` must run against the server,
/// so the guide cannot drift from the model format again.
#[test]
fn test_mcp_doc_inline_examples_run() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/MCP.md");
    let doc = std::fs::read_to_string(path).expect("docs/MCP.md should exist");
    let mut ran = 0;
    for line in doc.lines().filter(|l| l.starts_with("{\"jsonrpc\"")) {
        let call: serde_json::Value =
            serde_json::from_str(line).expect("doc example should be valid JSON");
        let name = call["params"]["name"].as_str().expect("tool name");
        let args = call["params"]["arguments"].clone();
        if args.get("content").is_none() {
            continue; // file_path examples name files that do not exist here
        }
        let server = ForgeMcpServer::new();
        let result = match name {
            "forge_validate" => server.validate(Parameters(
                serde_json::from_value(args).expect("validate arguments"),
            )),
            "forge_calculate" => server.calculate(Parameters(
                serde_json::from_value(args).expect("calculate arguments"),
            )),
            other => panic!("docs/MCP.md has an inline example for {other}; add it here"),
        };
        if let Err(e) = result {
            panic!("docs/MCP.md example for {name} failed: {e}\n{line}");
        }
        ran += 1;
    }
    assert!(ran >= 3, "expected at least 3 inline examples, ran {ran}");
}

#[test]
fn test_inline_with_includes() {
    let yaml_with_includes = r#"
_forge_version: "5.0.0"
_includes:
  - file: "pricing.yaml"
    as: "pricing"
scalars:
  total:
    value: 100
"#;
    let pricing_yaml = r#"
_forge_version: "5.0.0"
scalars:
  price:
    value: 50
"#;
    let mut includes = HashMap::new();
    includes.insert("pricing".into(), pricing_yaml.into());

    let server = ForgeMcpServer::new();
    let text = ok_text(server.validate(Parameters(ValidateRequest {
        file_path: None,
        content: Some(yaml_with_includes.into()),
        includes: Some(includes),
        verbose: false,
    })));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.get("scalars_valid").is_some());
}

#[test]
fn test_inline_includes_missing_namespace() {
    let yaml_with_includes = r#"
_forge_version: "5.0.0"
_includes:
  - file: "pricing.yaml"
    as: "pricing"
scalars:
  total:
    value: 100
"#;
    // Provide includes map but with wrong namespace
    let mut includes = HashMap::new();
    includes.insert("wrong_namespace".into(), "content".into());

    let server = ForgeMcpServer::new();
    let result = server.validate(Parameters(ValidateRequest {
        file_path: None,
        content: Some(yaml_with_includes.into()),
        includes: Some(includes),
        verbose: false,
    }));
    let err = err_text(result);
    assert!(err.contains("pricing"));
}

#[test]
fn test_inline_includes_no_map() {
    let yaml_with_includes = r#"
_forge_version: "5.0.0"
_includes:
  - file: "pricing.yaml"
    as: "pricing"
scalars:
  total:
    value: 100
"#;
    let server = ForgeMcpServer::new();
    let result = server.validate(Parameters(ValidateRequest {
        file_path: None,
        content: Some(yaml_with_includes.into()),
        includes: None,
        verbose: false,
    }));
    let err = err_text(result);
    assert!(err.contains("includes"));
}

#[test]
fn test_inline_includes_nested_dir() {
    let yaml_with_includes = r#"
_forge_version: "5.0.0"
_includes:
  - file: "data/sub/pricing.yaml"
    as: "pricing"
scalars:
  total:
    value: 100
"#;
    let pricing_yaml = r#"
_forge_version: "5.0.0"
scalars:
  price:
    value: 25
"#;
    let mut includes = HashMap::new();
    includes.insert("pricing".into(), pricing_yaml.into());

    let server = ForgeMcpServer::new();
    let text = ok_text(server.validate(Parameters(ValidateRequest {
        file_path: None,
        content: Some(yaml_with_includes.into()),
        includes: Some(includes),
        verbose: false,
    })));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.get("scalars_valid").is_some());
}

fn validate_inline_with_include(file: &str, include_content: &str) -> Result<String, String> {
    let yaml = format!(
            "_forge_version: \"5.0.0\"\n_includes:\n  - file: \"{file}\"\n    as: \"ext\"\nscalars:\n  total:\n    value: 100\n"
        );
    let mut includes = HashMap::new();
    includes.insert("ext".to_string(), include_content.to_string());
    ForgeMcpServer::new().validate(Parameters(ValidateRequest {
        file_path: None,
        content: Some(yaml),
        includes: Some(includes),
        verbose: false,
    }))
}

#[test]
fn test_inline_include_absolute_path_writes_nothing() {
    let outside = TempDir::new().unwrap();
    let target = outside.path().join("written.yaml");
    let err = err_text(validate_inline_with_include(
        target.to_str().unwrap(),
        "planted: true\n",
    ));
    assert!(err.contains("not allowed"), "unexpected error: {err}");
    assert!(!target.exists(), "include escaped to an absolute path");
}

#[test]
fn test_inline_include_parent_path_writes_nothing() {
    let name = format!("forge-escape-{}.yaml", std::process::id());
    let err = err_text(validate_inline_with_include(
        &format!("../{name}"),
        "planted: true\n",
    ));
    assert!(err.contains("not allowed"), "unexpected error: {err}");
    assert!(!std::env::temp_dir().join(&name).exists());
}

#[test]
fn test_inline_include_hidden_parent_path_rejected() {
    let err = err_text(validate_inline_with_include(
        "data/../../escape.yaml",
        "planted: true\n",
    ));
    assert!(err.contains("not allowed"), "unexpected error: {err}");
}

#[test]
fn test_inline_nested_include_cannot_read_outside() {
    let outside = TempDir::new().unwrap();
    let secret = outside.path().join("secret.yaml");
    std::fs::write(
        &secret,
        "_forge_version: \"5.0.0\"\nscalars:\n  secret:\n    value: 42\n",
    )
    .unwrap();
    let content = format!(
        "_forge_version: \"5.0.0\"\n_includes:\n  - file: \"{}\"\n    as: \"s\"\n",
        secret.display()
    );
    let err = err_text(validate_inline_with_include("pricing.yaml", &content));
    assert!(err.contains("not allowed"), "unexpected error: {err}");
}

#[test]
fn test_inline_include_dot_slash_path_allowed() {
    let content = "_forge_version: \"5.0.0\"\nscalars:\n  price:\n    value: 25\n";
    let text = ok_text(validate_inline_with_include("./data/pricing.yaml", content));
    assert!(text.contains("scalars_valid"));
}

#[test]
fn test_inline_variance() {
    let budget = r#"
_forge_version: "5.0.0"
scalars:
  revenue:
    value: 100000
  costs:
    value: 60000
"#;
    let actual = r#"
_forge_version: "5.0.0"
scalars:
  revenue:
    value: 95000
  costs:
    value: 65000
"#;
    let server = ForgeMcpServer::new();
    let result = server.variance(Parameters(VarianceRequest {
        budget_path: None,
        budget_content: Some(budget.into()),
        actual_path: None,
        actual_content: Some(actual.into()),
        includes: None,
        threshold: Some(10.0),
    }));
    assert!(result.is_ok());
}

#[test]
fn test_inline_export() {
    let temp_dir = TempDir::new().unwrap();
    let output = temp_dir.path().join("inline_export.xlsx");

    let server = ForgeMcpServer::new();
    let result = server.export(Parameters(ExportRequest {
        yaml_path: None,
        content: Some(INLINE_YAML.into()),
        includes: None,
        excel_path: Some(output.to_str().unwrap().into()),
    }));
    assert!(result.is_ok());
}

#[test]
fn test_inline_export_buffer() {
    // When excel_path is omitted, should return base64-encoded Excel content
    let server = ForgeMcpServer::new();
    let text = ok_text(server.export(Parameters(ExportRequest {
        yaml_path: None,
        content: Some(INLINE_YAML.into()),
        includes: None,
        excel_path: None,
    })));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["excel_base64"].as_str().is_some());
    assert!(parsed["byte_count"].as_u64().unwrap() > 0);
    // Verify it's valid base64 that decodes to an xlsx magic number (PK\x03\x04)
    let b64 = parsed["excel_base64"].as_str().unwrap();
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64).unwrap();
    assert_eq!(
        &bytes[..4],
        b"PK\x03\x04",
        "Should be a valid ZIP/XLSX header"
    );
}
