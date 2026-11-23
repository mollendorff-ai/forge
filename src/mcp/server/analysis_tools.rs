//! The tools that analyse a model: sensitivity, goal seek, break-even,
//! variance, scenarios and their comparison, the tornado, and games.

use super::{to_json, ForgeMcpServer};
use crate::mcp::types::{
    BreakEvenRequest, CompareRequest, GameRequest, GoalSeekRequest, ScenariosRequest,
    SensitivityRequest, TornadoRequest, VarianceRequest,
};
use std::path::Path;

use crate::inline_model::resolve_model_input;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::cli::{
    compare_core, comparison_json, game_core, goal_seek_core, scenarios_core, sensitivity_core,
    tornado_core, variance_core,
};

/// Scenario results as JSON, a date's value as its text, as in `forge calculate`
/// (ADR-060): the engine holds a date as a day number.
fn scenarios_json(path: &Path, results: &crate::scenarios::ScenarioResults) -> String {
    let mut json =
        serde_json::to_value(crate::core::precision::Rounded(results)).unwrap_or_default();
    // The run parsed this file, so reading it again does not fail
    let dates: Vec<String> = crate::parser::parse_model(path)
        .map(|m| {
            m.scalars
                .iter()
                .filter(|(_, v)| v.is_date)
                .map(|(k, _)| k.clone())
                .collect()
        })
        .unwrap_or_default();
    // A date is an input, so it is in a scenario's scalars and the expected
    // values, and never in the ranges, which are of calculated outputs
    let as_text = |v: &mut serde_json::Value| {
        let day = v.as_f64().expect("a date is a number");
        *v = serde_json::Value::String(crate::core::array_calculator::evaluator::serial_date_text(
            day,
        ));
    };
    for name in &dates {
        for scenario in json["scenarios"]
            .as_array_mut()
            .expect("results have scenarios")
        {
            as_text(&mut scenario["scalars"][name]);
        }
        as_text(&mut json["expected_values"][name]);
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
        description = "Run probability-weighted scenario analysis (Base/Bull/Bear). Each scenario overrides scalar values and calculates results. A scenario without a probability is left out of the weighting and named in `unweighted`; forge_compare puts it beside the others."
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

    #[tool(
        name = "forge_game",
        description = "Find the equilibria of the games in a model's 'games' section, and play its tournaments (ADR-068). For a game played once: strictly dominated strategies in the order removed, weakly dominated ones, pure Nash equilibria (any number of players), mixed ones (two players, support enumeration, with the ties of a degenerate game named; in a symmetric game of two strategies of any number of players, the mixes in which all play alike, each with `stable`), each strategy's payoff against an even mix, and with a `dynamics:` key the share of each strategy generation by generation (`dynamics.path`, ADR-073). For a tournament of built-in strategies (tit_for_tat, grim_trigger, ...): each player's score, rank and cooperation, the match table and the collectively stable players, as exact expected values. Each report has `kind` table or tournament. The same numbers are scalars formulas can read (stag_hunt.equilibrium_1.me.stag, hawk_dove.dynamics.share.hawk, axelrod.score.tft)."
    )]
    fn game(&self, Parameters(req): Parameters<GameRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            self.root.as_ref(),
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        game_core(&path, req.game.as_deref(), req.scenario.as_deref())
            .map(|r| to_json(&r))
            .map_err(|e| format!("Game analysis failed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::server::tests::{
        bagel_source, close, err_text, example_source, json, ok_text, Source,
    };
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
    fn forge_scenarios_names_the_scenarios_it_does_not_weigh() {
        // ADR-067: a scenario without a probability is a choice, left out of
        // the weighting and named in `unweighted`; the chances still weigh
        let content = r#"
_forge_version: "5.1.0"
price: 100
units: 50
revenue: {formula: "=price * units"}
scenarios:
  high:
    probability: 0.5
    scalars:
      price: 200
  double_units:
    scalars:
      units: 100
  low:
    probability: 0.5
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
        assert_eq!(parsed["unweighted"], serde_json::json!(["double_units"]));
        assert_eq!(parsed["scenarios"].as_array().unwrap().len(), 2);
        // 0.5 * 10,000 + 0.5 * 2,500
        assert_eq!(
            parsed["expected_values"]["revenue"],
            serde_json::json!(6250.0)
        );
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
        // Each scenario actually overrode the grouped scalar
        let profits: Vec<(String, f64)> = scenarios
            .iter()
            .map(|s| {
                (
                    s["name"].as_str().unwrap().to_string(),
                    s["scalars"]["profit"].as_f64().unwrap().round(),
                )
            })
            .collect();
        assert_eq!(
            profits,
            [
                ("bear".to_string(), 20_000.0),
                ("bull".to_string(), 250_000.0)
            ]
        );
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
        // The weighted date is a date too, and a calculated output stays a number
        assert_eq!(v["expected_values"]["inputs.last_actual"], "2026-11-15");
        assert_eq!(
            v["ranges"]["out.next"],
            serde_json::json!([23178.0, 23193.5])
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // The numbers of examples/bagel through the tools (T1-INTERFACE-PARITY):
    // each is asserted by the command-line tests in cli/commands/examples.rs
    // (`bagel_example`, the Python is quoted there), here by file path and as
    // inline content with `costs.yaml` passed in `includes`.
    // ─────────────────────────────────────────────────────────────────────────

    /// The bagel plan, by path or inline with `costs.yaml`
    fn plan(inline: bool) -> Source {
        bagel_source(inline, "bagel.yaml", &[("costs", "costs.yaml")])
    }

    /// `price_per_dozen` and `opening_dozens_per_day` moved together: the four
    /// cells the tornado test asserts (14 and 18 dollars, 45 and 65 dozens, each
    /// with the other at the plan) and the plan itself.
    #[test]
    fn sensitivity_gives_the_tornados_cells_by_path_and_inline() {
        for inline in [false, true] {
            let (file_path, content, includes) = plan(inline);
            let result = json(&ok_text(ForgeMcpServer::new().sensitivity(Parameters(
                SensitivityRequest {
                    file_path,
                    content,
                    includes,
                    vary: "assumptions.price_per_dozen".into(),
                    range: "14,18,2".into(),
                    output: "year.profit".into(),
                    vary2: Some("assumptions.opening_dozens_per_day".into()),
                    range2: Some("45,65,10".into()),
                },
            ))));
            let grid = &result["data"]["TwoVar"];
            assert_eq!(grid["row_values"], serde_json::json!([14.0, 16.0, 18.0]));
            assert_eq!(grid["col_values"], serde_json::json!([45.0, 55.0, 65.0]));
            let cell =
                |price: usize, opening: usize| grid["matrix"][price][opening].as_f64().unwrap();
            assert!(close(cell(0, 1), -46_309.92, 0.006), "inline {inline}");
            assert!(close(cell(2, 1), 47_882.56, 0.006), "inline {inline}");
            assert!(close(cell(1, 0), -56_463.56, 0.006), "inline {inline}");
            assert!(close(cell(1, 2), 58_036.20, 0.006), "inline {inline}");
            assert!(close(cell(1, 1), 786.320_361_7, 1e-6), "inline {inline}");
        }
    }

    /// `break_even_price_and_staffing_for_sixty_thousand`, as a goal seek and
    /// as a break-even.
    #[test]
    fn goal_seek_and_break_even_find_the_cli_prices_by_path_and_inline() {
        let server = ForgeMcpServer::new();
        for inline in [false, true] {
            let (file_path, content, includes) = plan(inline);
            let hours = json(&ok_text(server.goal_seek(Parameters(GoalSeekRequest {
                file_path,
                content,
                includes,
                target: "year.profit".into(),
                value: 60_000.0,
                vary: "assumptions.staff_hours_per_day".into(),
                min: Some(8.0),
                max: Some(40.0),
                tolerance: 0.0001,
            }))));
            assert_eq!(hours["converged"], true);
            let hours = hours["solution"].as_f64().unwrap();
            assert!(close(hours, 24.268_7, 1e-3), "inline {inline}: {hours}");

            let (file_path, content, includes) = plan(inline);
            let price = json(&ok_text(server.break_even(Parameters(BreakEvenRequest {
                file_path,
                content,
                includes,
                output: "year.profit".into(),
                vary: "assumptions.price_per_dozen".into(),
                min: Some(10.0),
                max: Some(30.0),
            }))));
            assert_eq!(price["converged"], true);
            let price = price["solution"].as_f64().unwrap();
            assert!(close(price, 15.966_6, 1e-3), "inline {inline}: {price}");
        }
    }

    /// The flour price in the included file, which the scenario `flour_shock` sets
    /// to 1.70. The year's profit is linear in it: 786.3203617 at 1.15 and
    /// -16943.271075 at 1.70 (the two numbers of `flour_shock_changes_the_included_price`),
    /// so it is zero at 1.15 + 786.3203617 / ((786.3203617 + 16943.271075) / 0.55).
    #[test]
    fn break_even_varies_a_value_in_the_included_file_by_path_and_inline() {
        let zero = 1.15 + 786.320_361_7 / ((786.320_361_7 + 16_943.271_075) / 0.55);
        for inline in [false, true] {
            let (file_path, content, includes) = plan(inline);
            let result = json(&ok_text(ForgeMcpServer::new().break_even(Parameters(
                BreakEvenRequest {
                    file_path,
                    content,
                    includes,
                    output: "year.profit".into(),
                    vary: "@costs.ingredients.flour_per_kg".into(),
                    min: Some(0.5),
                    max: Some(5.0),
                },
            ))));
            let flour = result["solution"].as_f64().unwrap();
            assert!(close(flour, zero, 1e-3), "inline {inline}: {flour}");
        }
    }

    /// `variance_judges_costs_as_costs`: both files include `costs.yaml`, which
    /// the one `includes` map gives for both.
    #[test]
    fn variance_judges_costs_as_costs_by_path_and_inline() {
        for inline in [false, true] {
            let (budget_path, budget_content, includes) = plan(inline);
            let (actual_path, actual_content, _) =
                bagel_source(inline, "actual.yaml", &[("costs", "costs.yaml")]);
            let result = json(&ok_text(ForgeMcpServer::new().variance(Parameters(
                VarianceRequest {
                    budget_path,
                    budget_content,
                    actual_path,
                    actual_content,
                    includes,
                    threshold: None,
                },
            ))));
            let rows = result["results"].as_array().unwrap();
            let get = |name: &str| rows.iter().find(|r| r["name"] == name).unwrap();
            assert_eq!(get("assumptions.rent_per_month")["is_favorable"], false);
            assert_eq!(get("assumptions.staff_hours_per_day")["better"], "lower");
            assert_eq!(
                get("assumptions.staff_hours_per_day")["is_favorable"],
                false
            );
            assert!(get("assumptions.price_per_dozen")["is_favorable"].is_null());
            let npv = get("year.npv_first_year");
            let pct = npv["variance_pct"].as_f64().unwrap();
            assert!(close(pct, -28.959_419, 1e-5), "inline {inline}: {pct}");
            assert_eq!(npv["is_favorable"], false);
            assert_eq!(result["not_compared"], serde_json::json!([]));
        }
    }

    /// `first_year_return_is_defined_whatever_the_profit`: the base case and the
    /// flour shock side by side.
    #[test]
    fn compare_sets_the_flour_shock_beside_the_plan_by_path_and_inline() {
        for inline in [false, true] {
            let (file_path, content, includes) = plan(inline);
            let result = json(&ok_text(ForgeMcpServer::new().compare(Parameters(
                CompareRequest {
                    file_path,
                    content,
                    includes,
                    scenarios: vec!["base".into(), "flour_shock".into()],
                },
            ))));
            assert_eq!(
                result["scenarios"],
                serde_json::json!(["base", "flour_shock"])
            );
            let row = &result["values"]["year.first_year_return"];
            assert!(close(
                row["base"].as_f64().unwrap(),
                786.320_361_7 / 95_000.0,
                1e-9
            ));
            let shock = row["flour_shock"].as_f64().unwrap();
            assert!(
                close(shock, -16_943.271_075 / 95_000.0, 1e-9),
                "inline {inline}: {shock}"
            );
            let profit = result["values"]["year.profit"]["flour_shock"]
                .as_f64()
                .unwrap();
            assert!(
                close(profit, -16_943.271_075, 1e-6),
                "inline {inline}: {profit}"
            );
        }
    }

    /// `flour_shock_changes_the_included_price`: the shock's profit, and the
    /// expected value over the four scenarios (0.5 x 786.32 + 0.1 x -16943.27 +
    /// 0.2 x -71197.20 + 0.2 x 80811.24).
    #[test]
    fn scenarios_weight_the_four_by_path_and_inline() {
        let server = ForgeMcpServer::new();
        for inline in [false, true] {
            let (file_path, content, includes) = plan(inline);
            let all = json(&ok_text(server.scenarios(Parameters(ScenariosRequest {
                file_path,
                content,
                includes,
                scenario_filter: None,
            }))));
            let expected = all["expected_values"]["year.profit"].as_f64().unwrap();
            assert!(
                close(expected, 621.641_286, 1e-5),
                "inline {inline}: {expected}"
            );
            assert_eq!(all["scenarios"].as_array().map(Vec::len), Some(4));

            let (file_path, content, includes) = plan(inline);
            let one = json(&ok_text(server.scenarios(Parameters(ScenariosRequest {
                file_path,
                content,
                includes,
                scenario_filter: Some("flour_shock".into()),
            }))));
            assert_eq!(one["scenarios"][0]["name"], "flour_shock");
            let profit = one["scenarios"][0]["outputs"]["year.profit"]
                .as_f64()
                .unwrap();
            assert!(
                close(profit, -16_943.271_075, 1e-6),
                "inline {inline}: {profit}"
            );
        }
    }

    /// `tornado_matches_the_independent_calculation`: each input alone at its
    /// low and high, largest swing first.
    #[test]
    fn tornado_matches_the_independent_calculation_by_path_and_inline() {
        let expected = [
            ("assumptions.opening_dozens_per_day", -56_463.56, 58_036.20),
            ("assumptions.price_per_dozen", -46_309.92, 47_882.56),
            ("assumptions.staff_hours_per_day", 46_740.32, -29_849.68),
            ("assumptions.rent_per_month", 10_386.32, -13_613.68),
            ("assumptions.card_fee_rate", 3_880.94, -760.99),
        ];
        for inline in [false, true] {
            let (file_path, content, includes) = plan(inline);
            let result = json(&ok_text(ForgeMcpServer::new().tornado(Parameters(
                TornadoRequest {
                    file_path,
                    content,
                    includes,
                    output_var: None,
                },
            ))));
            let bars = result["bars"].as_array().unwrap();
            assert_eq!(bars.len(), expected.len());
            for (bar, (name, low, high)) in bars.iter().zip(expected) {
                assert_eq!(bar["input_name"], name);
                assert!(
                    close(bar["output_at_low"].as_f64().unwrap(), low, 0.006),
                    "{name}"
                );
                assert!(
                    close(bar["output_at_high"].as_f64().unwrap(), high, 0.006),
                    "{name}"
                );
            }
        }
    }

    /// examples/acquisition, which includes the buyer and the target
    fn takeover(inline: bool) -> Source {
        example_source(
            inline,
            "acquisition",
            "deal.yaml",
            &[("buyer", "buyer.yaml"), ("target", "target.yaml")],
        )
    }

    /// `each_scenario_matches_the_independent_calculation`: the four weighted
    /// scenarios of the takeover create 139.4853 on average, against 224.5079
    /// at the plan.
    #[test]
    fn scenarios_weight_the_takeovers_four_cases_by_path_and_inline() {
        for inline in [false, true] {
            let (file_path, content, includes) = takeover(inline);
            let result = json(&ok_text(ForgeMcpServer::new().scenarios(Parameters(
                ScenariosRequest {
                    file_path,
                    content,
                    includes,
                    scenario_filter: None,
                },
            ))));
            let weighted = result["expected_values"]["valuation.value_created"]
                .as_f64()
                .unwrap();
            assert!(
                close(weighted, 139.485_303_520_619_45, 1e-9),
                "inline {inline}: {weighted}"
            );
        }
    }

    /// `goal_seek_moves_an_input_of_an_included_file`: Quillon's free cash flow,
    /// a value in `target.yaml`, at which the takeover creates nothing; the
    /// README says 51.40 a year, from 62. Written as the scenarios write it.
    #[test]
    fn break_even_moves_an_input_of_the_included_target_by_path_and_inline() {
        for inline in [false, true] {
            let (file_path, content, includes) = takeover(inline);
            let result = json(&ok_text(ForgeMcpServer::new().break_even(Parameters(
                BreakEvenRequest {
                    file_path,
                    content,
                    includes,
                    output: "valuation.value_created".into(),
                    vary: "@target.company.free_cash_flow_next_year".into(),
                    min: None,
                    max: None,
                },
            ))));
            assert_eq!(result["converged"], true, "inline {inline}");
            assert_eq!(result["vary"], "@target.company.free_cash_flow_next_year");
            let cash = result["solution"].as_f64().unwrap();
            assert!(close(cash, 51.40, 0.005), "inline {inline}: {cash}");
        }
    }

    /// kinship-protocol's tournament and the stag hunt, inline: the scores
    /// are the exact ones of src/games/standings.rs (by hand, and against
    /// axelrod 4.14.0), the mixed equilibrium nashpy 0.0.43's
    #[test]
    fn game_reports_tables_and_tournaments() {
        let content = r#"_forge_version: "5.5.0"
c: 0
games:
  stag_hunt:
    players: [me, you]
    strategies: [stag, hare]
    symmetric: true
    payoffs:
      stag: {stag: 4, hare: 0}
      hare: {stag: =3 - c, hare: =3 - c}
  dilemma:
    players: [me, you]
    strategies: [cooperate, defect]
    symmetric: true
    payoffs:
      cooperate: {cooperate: 3, defect: 0}
      defect: {cooperate: 5, defect: 1}
  axelrod:
    tournament:
      round: dilemma
      players: {grim: grim_trigger, tft: tit_for_tat, ad: always_defect, ac: always_cooperate, rand: random}
      rounds: 200
scenarios:
  costly:
    c: 1
"#;
        let server = ForgeMcpServer::new();
        let ask = |game: Option<&str>, scenario: Option<&str>| {
            server.game(Parameters(GameRequest {
                file_path: None,
                content: Some(content.into()),
                includes: None,
                game: game.map(Into::into),
                scenario: scenario.map(Into::into),
            }))
        };
        let all = json(&ok_text(ask(None, None)));
        assert_eq!(all[0]["kind"], "table");
        assert!(close(
            all[0]["equilibria"][2]["players"]["me"]["strategies"]["stag"]
                .as_f64()
                .unwrap(),
            0.75,
            1e-12
        ));
        assert_eq!(all[2]["kind"], "tournament");
        assert_eq!(all[2]["ranking"][0]["player"], "grim");
        assert!(close(
            all[2]["ranking"][0]["score"].as_f64().unwrap(),
            2.596,
            1e-12
        ));
        assert_eq!(all[2]["stable"], serde_json::json!(["grim", "tft", "ad"]));
        // c = 1: the mix is stag at (3 - 1) / 4
        let costly = json(&ok_text(ask(Some("stag_hunt"), Some("costly"))));
        assert_eq!(costly.as_array().unwrap().len(), 1);
        assert!(close(
            costly[0]["equilibria"][2]["players"]["me"]["strategies"]["stag"]
                .as_f64()
                .unwrap(),
            0.5,
            1e-12
        ));
        let err = err_text(ask(Some("nope"), None));
        assert!(
            err.starts_with("Game analysis failed: ")
                && err
                    .contains("Game 'nope' not found; the model has: stag_hunt, dilemma, axelrod"),
            "{err}"
        );
    }

    /// The games of many players and the dynamics (ADR-073), by path and as
    /// text with their includes: the bagel's price war across the city and
    /// the street's festival of five shops (worked out by hand in
    /// src/cli/commands/game.rs), and the volunteer's dilemma of five
    /// players, 1 - 0.4^(1/4) (Python 3.12)
    #[test]
    fn game_reports_games_of_five_players_and_the_dynamics() {
        let server = ForgeMcpServer::new();
        let includes = [("plan", "bagel.yaml"), ("costs", "costs.yaml")];
        for inline in [false, true] {
            let ask = |(file_path, content, includes): Source| {
                json(&ok_text(server.game(Parameters(GameRequest {
                    file_path,
                    content,
                    includes,
                    game: None,
                    scenario: None,
                }))))
            };
            let war = ask(bagel_source(inline, "price_war.yaml", &includes));
            let path = war[0]["dynamics"]["path"].as_array().unwrap();
            assert_eq!(path.len(), 25, "inline {inline}");
            assert_eq!(path[0]["shares"]["cut"].as_f64(), Some(0.1));
            let cut = path[24]["shares"]["cut"].as_f64().unwrap();
            assert!(close(cut, 0.991_336_378_216_362_2, 1e-12), "{cut}");

            let street = ask(bagel_source(inline, "street_fund.yaml", &includes));
            assert_eq!(street[0]["players"].as_array().unwrap().len(), 5);
            assert_eq!(street[0]["cells"].as_array().unwrap().len(), 32);
            assert_eq!(street[0]["symmetric_searched"], true);
            assert_eq!(street[0]["equilibria"].as_array().unwrap().len(), 1);
            assert_eq!(
                street[0]["equilibria"][0]["players"]["deli"]["strategies"]["keep"],
                1.0
            );
            assert_eq!(street[0]["equilibria"][0]["stable"], true);
            let pay = street[0]["even_mix"]["lune"]["pay"].as_f64().unwrap();
            assert!(close(pay, 875.4368, 1e-9), "{pay}");

            let dilemma = ask(example_source(
                inline,
                "games",
                "volunteers-dilemma.yaml",
                &[],
            ));
            let mixed = &dilemma[0]["equilibria"][5];
            assert_eq!(mixed["kind"], "mixed");
            assert_eq!(mixed["stable"], true);
            let share = mixed["players"]["a5"]["strategies"]["volunteer"]
                .as_f64()
                .unwrap();
            assert!(close(share, 0.204_729_271_232_949, 1e-12), "{share}");
        }
    }

    /// What every tool that reads a model says when it is given none
    const ASKS: &str = "Either file_path or content is required";

    fn request<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> Parameters<T> {
        Parameters(serde_json::from_value(value).expect("a request"))
    }

    #[test]
    fn each_analysis_tool_asks_for_a_model_when_it_gets_none() {
        use serde_json::json;
        let server = ForgeMcpServer::new();
        assert_eq!(
            server
                .sensitivity(request(
                    json!({"vary": "a", "range": "1,2,1", "output": "b"})
                ))
                .unwrap_err(),
            ASKS
        );
        assert_eq!(
            server
                .goal_seek(request(
                    json!({"target": "b", "value": 1.0, "vary": "a", "tolerance": 0.1})
                ))
                .unwrap_err(),
            ASKS
        );
        assert_eq!(
            server
                .break_even(request(json!({"output": "b", "vary": "a"})))
                .unwrap_err(),
            ASKS
        );
        assert_eq!(
            server
                .compare(request(json!({"scenarios": ["base"]})))
                .unwrap_err(),
            ASKS
        );
        assert_eq!(server.scenarios(request(json!({}))).unwrap_err(), ASKS);
        assert_eq!(server.tornado(request(json!({}))).unwrap_err(), ASKS);
        assert_eq!(server.game(request(json!({}))).unwrap_err(), ASKS);
    }

    #[test]
    fn a_failed_sensitivity_or_variance_says_why() {
        let server = ForgeMcpServer::new();
        let priced =
            "_forge_version: \"5.1.0\"\nprice: 4\nout:\n  total: {formula: \"=price * 2\"}\n";
        assert_eq!(
            err_text(server.sensitivity(Parameters(SensitivityRequest {
                file_path: None,
                content: Some(priced.to_string()),
                includes: None,
                vary: "nope".into(),
                range: "1,2,1".into(),
                output: "out.total".into(),
                vary2: None,
                range2: None,
            }))),
            "Sensitivity analysis failed: Validation error: Variable 'nope' not found. Available scalars: [\"price\", \"out.total\"]"
        );
        let variance = |budget: Option<&str>, actual: &str| {
            err_text(server.variance(Parameters(VarianceRequest {
                budget_path: None,
                budget_content: budget.map(str::to_string),
                actual_path: None,
                actual_content: Some(actual.to_string()),
                includes: None,
                threshold: None,
            })))
        };
        assert_eq!(
            variance(None, priced),
            "Either budget_path or budget_content is required"
        );
        let broken = "_forge_version: \"5.1.0\"\nout:\n  total: {formula: \"=nope + 1\"}\n";
        assert_eq!(
            variance(Some(broken), broken),
            "Variance analysis failed: Formula evaluation error: out.total: Unknown variable: nope (formula: =nope + 1)"
        );
    }

    #[test]
    fn a_failed_goal_seek_break_even_or_tornado_says_why() {
        let server = ForgeMcpServer::new();
        let content = Some(
            "_forge_version: \"5.1.0\"\nprice: 4\nout:\n  total: {formula: \"=price * 2\"}\n"
                .to_string(),
        );
        assert_eq!(
            err_text(server.goal_seek(Parameters(GoalSeekRequest {
                file_path: None,
                content: content.clone(),
                includes: None,
                target: "nope".into(),
                value: 1.0,
                vary: "price".into(),
                min: Some(0.0),
                max: Some(10.0),
                tolerance: 0.01,
            }))),
            "Goal seek failed: Validation error: Output variable 'nope' not found in model"
        );
        assert_eq!(
            err_text(server.break_even(Parameters(BreakEvenRequest {
                file_path: None,
                content: content.clone(),
                includes: None,
                output: "nope".into(),
                vary: "price".into(),
                min: Some(0.0),
                max: Some(10.0),
            }))),
            "Break-even analysis failed: Validation error: Output variable 'nope' not found in model"
        );
        assert_eq!(
            err_text(server.tornado(Parameters(TornadoRequest {
                file_path: None,
                content,
                includes: None,
                output_var: None,
            }))),
            "Tornado analysis failed: Validation error: No 'tornado' section found in YAML"
        );
    }
}
