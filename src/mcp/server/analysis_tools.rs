//! The tools that analyse a model: sensitivity, goal seek, break-even,
//! variance, scenarios and their comparison, and the tornado.

use super::{to_json, ForgeMcpServer};
use crate::mcp::types::{
    BreakEvenRequest, CompareRequest, GoalSeekRequest, ScenariosRequest, SensitivityRequest,
    TornadoRequest, VarianceRequest,
};
use std::path::Path;

use crate::inline_model::resolve_model_input;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::cli::{
    compare_core, comparison_json, goal_seek_core, scenarios_core, sensitivity_core, tornado_core,
    variance_core,
};

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

#[tool_router(router = analysis_tools, vis = "pub(super)")]
impl ForgeMcpServer {
    #[tool(
        name = "forge_sensitivity",
        description = "Run sensitivity analysis by varying one or two input variables and observing output changes. Essential for what-if modeling and risk assessment."
    )]
    fn sensitivity(
        &self,
        Parameters(req): Parameters<SensitivityRequest>,
    ) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            self.root.as_ref(),
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
            self.root.as_ref(),
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
            self.root.as_ref(),
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
    pub(super) fn variance(
        &self,
        Parameters(req): Parameters<VarianceRequest>,
    ) -> Result<String, String> {
        let (budget_path, _tmpdir_b) = resolve_model_input(
            self.root.as_ref(),
            req.budget_path.as_deref(),
            req.budget_content.as_deref(),
            req.includes.as_ref(),
        )
        .map_err(|e| e.replace("file_path or content", "budget_path or budget_content"))?;
        let (actual_path, _tmpdir_a) = resolve_model_input(
            self.root.as_ref(),
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
            self.root.as_ref(),
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        compare_core(&path, &req.scenarios)
            .map(|r| comparison_json(&r).to_string())
            .map_err(|e| format!("Scenario comparison failed: {e}"))
    }

    #[tool(
        name = "forge_scenarios",
        description = "Run probability-weighted scenario analysis (Base/Bull/Bear). Each scenario overrides scalar values and calculates results."
    )]
    fn scenarios(&self, Parameters(req): Parameters<ScenariosRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            self.root.as_ref(),
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        scenarios_core(&path, req.scenario_filter.as_deref())
            .map(|r| scenarios_json(&path, &r))
            .map_err(|e| format!("Scenario analysis failed: {e}"))
    }

    #[tool(
        name = "forge_tornado",
        description = "Generate tornado sensitivity diagram. Varies each input one-at-a-time to show which inputs have the greatest impact on the output."
    )]
    fn tornado(&self, Parameters(req): Parameters<TornadoRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            self.root.as_ref(),
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        tornado_core(&path, req.output_var.as_deref())
            .map(|r| to_json(&r))
            .map_err(|e| format!("Tornado analysis failed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::server::tests::{err_text, ok_text};
    use rmcp::handler::server::wrapper::Parameters;

    const DATED: &str = r#"_forge_version: "5.4.0"
inputs:
  last_actual: 2026-10-31
  rate: {value: 0.5, format: "0%"}
out:
  next:
    formula: "=EOMONTH(inputs.last_actual, 1) * inputs.rate"
scenarios:
  base:
    probability: 0.5
    scalars: {}
  later:
    probability: 0.5
    scalars:
      last_actual: 2026-11-30
"#;

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
    fn test_compare_gives_a_date_as_its_text() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.compare(Parameters(CompareRequest {
            file_path: None,
            content: Some(DATED.to_string()),
            includes: None,
            scenarios: vec!["base".into(), "later".into()],
        })));
        let v: serde_json::Value = serde_json::from_str(&text).expect("json");
        assert_eq!(v["values"]["inputs.last_actual"]["base"], "2026-10-31");
        assert_eq!(v["values"]["inputs.last_actual"]["later"], "2026-11-30");
        assert!(v["values"]["inputs.rate"]["base"].is_number());
    }

    #[test]
    fn test_scenarios_give_a_date_as_its_text() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.scenarios(Parameters(ScenariosRequest {
            file_path: None,
            content: Some(DATED.to_string()),
            includes: None,
            scenario_filter: None,
        })));
        let v: serde_json::Value = serde_json::from_str(&text).expect("json");
        let dates: Vec<&str> = v["scenarios"]
            .as_array()
            .expect("scenarios")
            .iter()
            .filter_map(|s| s["scalars"]["inputs.last_actual"].as_str())
            .collect();
        assert!(
            dates.contains(&"2026-10-31") && dates.contains(&"2026-11-30"),
            "{text}"
        );
        assert!(v["expected_values"]["inputs.rate"].is_number());
    }
}
