//! The tools for uncertainty: Monte Carlo, decision trees, real options,
//! the bootstrap and Bayesian networks.

use super::{to_json, ForgeMcpServer};
use crate::core::precision;
use crate::mcp::types::{
    BayesianRequest, BootstrapRequest, DecisionTreeRequest, RealOptionsRequest, SimulateRequest,
};
use num_traits::ToPrimitive;

use crate::inline_model::resolve_model_input;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::cli::{
    bayesian_core, bootstrap_core, decision_tree_core, real_options_core, simulate_core,
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
                let i = (p * precision::count(sorted.len() - 1))
                    .round()
                    .to_usize()
                    .unwrap_or(0)
                    .min(sorted.len() - 1);
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

#[tool_router(router = uncertainty_tools, vis = "pub(super)")]
impl ForgeMcpServer {
    #[tool(
        name = "forge_simulate",
        description = "Run Monte Carlo simulation with probabilistic distributions (MC.Normal, MC.Triangular, MC.Uniform, MC.PERT, MC.Lognormal). Returns statistics, percentiles, threshold probabilities, and drivers (each MC.* input's rank correlation with each output). An MC.* input may be in an included file. `scenario` applies a scenario's overrides first. An output may be a table column (one result per row). With `seek` (\"P(year.profit > 0) = 0.8\"), `vary`, `min` and `max`, returns the value of that input at which the probability is first met."
    )]
    fn simulate(&self, Parameters(req): Parameters<SimulateRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            self.root.as_ref(),
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        let iterations = req.iterations;

        let outputs = req.outputs.unwrap_or_default();
        if req.seek.is_some() && !outputs.is_empty() {
            return Err(
                "outputs cannot be used with seek, which reports its own output".to_string(),
            );
        }
        let options = crate::cli::SimulateOptions {
            iterations,
            seed: req.seed,
            sampling: req.sampling.as_deref(),
            scenario: req.scenario.as_deref(),
            outputs: &outputs,
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
            .map(|r| r.to_json())
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
            self.root.as_ref(),
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
            self.root.as_ref(),
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        real_options_core(&path)
            .map(|r| to_json(&r))
            .map_err(|e| format!("Real options analysis failed: {e}"))
    }

    #[tool(
        name = "forge_bootstrap",
        description = "Non-parametric bootstrap resampling for confidence intervals. Returns original estimate, bootstrap mean, std error, bias, confidence intervals and nine percentiles of the bootstrap distribution (not every resample)."
    )]
    fn bootstrap(&self, Parameters(req): Parameters<BootstrapRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            self.root.as_ref(),
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        let iterations = req.iterations;

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
            self.root.as_ref(),
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        let evidence = req.evidence.unwrap_or_default();
        bayesian_core(&path, req.query_var.as_deref(), &evidence)
            .map(|r| to_json(&r))
            .map_err(|e| format!("Bayesian inference failed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::server::tests::{
        bagel_source, close, err_text, example_source, json, ok_text, Source,
    };
    use rmcp::handler::server::wrapper::Parameters;

    use tempfile::TempDir;

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
            outputs: None,
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(
            parsed["monte_carlo_results"]["iterations"]
                .as_u64()
                .unwrap()
                > 0
        );
    }

    /// T1-SIM-SHOW-ONE: `outputs` reports only the outputs named, and is refused
    /// beside `seek`, which reports its own output.
    #[test]
    fn test_call_simulate_outputs_filter() {
        let content = r#"
_forge_version: "5.0.0"
monte_carlo:
  iterations: 100
  seed: 42
  outputs:
    - variable: revenue
    - variable: costs
scalars:
  revenue:
    value: 100
    formula: "=MC.Normal(100, 10)"
  costs:
    value: 60
    formula: "=MC.Normal(60, 5)"
"#;
        let request = |outputs: Option<Vec<String>>, seek: Option<String>| SimulateRequest {
            file_path: None,
            content: Some(content.to_string()),
            includes: None,
            iterations: None,
            seed: None,
            sampling: None,
            scenario: None,
            seek,
            vary: None,
            min: None,
            max: None,
            outputs,
        };
        let server = ForgeMcpServer::new();
        let text = ok_text(server.simulate(Parameters(request(Some(vec!["costs".into()]), None))));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        let outputs = parsed["monte_carlo_results"]["outputs"]
            .as_object()
            .unwrap();
        assert_eq!(outputs.keys().collect::<Vec<_>>(), ["costs"]);

        let err = err_text(server.simulate(Parameters(request(Some(vec!["profit".into()]), None))));
        assert!(
            err.contains(
                "--output-var profit: not an output of monte_carlo (outputs: revenue, costs)"
            ),
            "{err}"
        );

        let err = err_text(server.simulate(Parameters(request(
            Some(vec!["costs".into()]),
            Some("P(costs > 50) = 0.5".into()),
        ))));
        assert_eq!(
            err,
            "outputs cannot be used with seek, which reports its own output"
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
            outputs: None,
        }));
        assert!(result.is_err());
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

    // ─────────────────────────────────────────────────────────────────────────
    // The numbers of examples/bagel through the tools (T1-INTERFACE-PARITY):
    // each is asserted by the command-line tests in cli/commands/examples.rs
    // (`bagel_example`, the Python is quoted there), here by file path and as
    // inline content with the included files passed in `includes`.
    // ─────────────────────────────────────────────────────────────────────────

    /// The Monte Carlo bagel model, by path or inline. It includes
    /// `costs_risk.yaml` (as `costs`), which includes `costs.yaml` (as `base`):
    /// an inline request names both.
    fn simulated(inline: bool) -> Source {
        bagel_source(
            inline,
            "bagel_simulate.yaml",
            &[("costs", "costs_risk.yaml"), ("base", "costs.yaml")],
        )
    }

    /// 1,000 runs of the bagel year, seed 7 (the file's), as the tool answers
    fn simulate(inline: bool, scenario: Option<&str>, seek: Option<&str>) -> serde_json::Value {
        let (file_path, content, includes) = simulated(inline);
        json(&ok_text(ForgeMcpServer::new().simulate(Parameters(
            SimulateRequest {
                file_path,
                content,
                includes,
                iterations: Some(1000),
                seed: None,
                sampling: None,
                scenario: scenario.map(Into::into),
                seek: seek.map(Into::into),
                vary: seek.map(|_| "price_per_dozen".into()),
                min: seek.map(|_| 14.0),
                max: seek.map(|_| 24.0),
                outputs: None,
            },
        ))))
    }

    /// `simulation_drivers_and_thresholds_match_scipy`: numpy gives the threshold
    /// probabilities and `scipy.stats.spearmanr` the rank correlations, on
    /// Forge's own input samples.
    #[test]
    fn simulate_gives_the_threshold_probabilities_and_drivers_by_path_and_inline() {
        for inline in [false, true] {
            let result = simulate(inline, None, None);
            let profit = &result["monte_carlo_results"]["outputs"]["year.profit"];
            assert_eq!(
                profit["thresholds"],
                serde_json::json!({"> 0": 0.349, "> 20000": 0.196, "< -50000": 0.205}),
                "inline {inline}"
            );
            // The year at the plan: the profit of bagel.yaml
            let plan = profit["at_plan"].as_f64().unwrap();
            assert!(close(plan, 786.320_361_7, 1e-6), "inline {inline}: {plan}");
            let drivers = profit["drivers"].as_array().unwrap();
            assert_eq!(drivers.len(), 4);
            for (driver, (name, rho)) in drivers.iter().zip([
                ("assumptions.opening_dozens_per_day", 0.845_145_641_146),
                ("assumptions.staff_hours_per_day", -0.417_853_641_854),
                ("@costs.labour.wage_per_hour", -0.322_749_874_750),
                ("@costs.ingredients.flour_per_kg", -0.098_687_702_688),
            ]) {
                assert_eq!(driver["input"], name);
                let got = driver["rank_correlation"].as_f64().unwrap();
                assert!(close(got, rho, 1e-11), "{name}: {got}");
            }
        }
    }

    /// `forge_calculate` with `plan`: the simulation's model with each
    /// uncertain input at its plan figure, every output at the value
    /// `forge_simulate` gives at plan (ADR-076); without it, refused.
    #[test]
    fn calculate_at_plan_gives_simulates_values_at_plan_by_path_and_inline() {
        use crate::mcp::types::CalculateRequest;
        for inline in [false, true] {
            let calculate = |plan: bool| {
                let (file_path, content, includes) = simulated(inline);
                ForgeMcpServer::new().calculate(Parameters(CalculateRequest {
                    file_path,
                    content,
                    includes,
                    dry_run: false,
                    scenario: None,
                    plan,
                }))
            };
            let result = json(&ok_text(calculate(true)));
            let simulated = simulate(inline, None, None);
            let outputs = simulated["monte_carlo_results"]["outputs"]
                .as_object()
                .unwrap();
            for name in ["year.profit", "year.worst_month_profit"] {
                let want = outputs[name]["at_plan"].as_f64().unwrap();
                let got = result["scalars"][name].as_f64().unwrap();
                assert!(
                    close(got, want, 1e-9),
                    "inline {inline}: {name} {got} {want}"
                );
            }
            let inputs = simulated["monte_carlo_results"]["inputs"]
                .as_object()
                .unwrap();
            for (name, input) in inputs {
                assert_eq!(
                    result["at_plan"][name], input["plan"],
                    "inline {inline}: {name}"
                );
            }
            // Nothing is written, so there is no calculated content to read back
            assert_eq!(result["dry_run"], true);
            assert!(result.get("calculated_content").is_none());
            let refused = calculate(false).unwrap_err();
            assert!(refused.contains("forge calculate --plan"), "{refused}");
        }
    }

    /// The rent hike moves every sample by exactly 12 months of 800, so the mean
    /// moves by 9,600 and so does the year at the plan.
    #[test]
    fn simulate_in_a_scenario_moves_the_year_by_the_rent_by_path_and_inline() {
        for inline in [false, true] {
            let base = simulate(inline, None, None);
            let hiked = simulate(inline, Some("rent_hike"), None);
            let at = |r: &serde_json::Value, field: &str| {
                r["monte_carlo_results"]["outputs"]["year.profit"][field]
                    .as_f64()
                    .unwrap()
            };
            assert!(close(at(&base, "mean") - at(&hiked, "mean"), 9600.0, 1e-6));
            assert!(close(
                at(&base, "at_plan") - at(&hiked, "at_plan"),
                9600.0,
                1e-6
            ));
        }
    }

    /// `price_for_an_eighty_percent_chance_of_profit`: just above the 800th
    /// lowest break-even price, 18.354117372865204 (the 801st is 18.359180292780614).
    #[test]
    fn seek_finds_the_price_for_an_eighty_percent_chance_by_path_and_inline() {
        for inline in [false, true] {
            let result = simulate(inline, None, Some("P(year.profit > 0) = 0.8"));
            assert_eq!(result["input"], "assumptions.price_per_dozen");
            let value = result["value"].as_f64().unwrap();
            assert!(
                value > 18.354_117_372_865_204 && value < 18.354_117_372_865_204 + 1e-4,
                "inline {inline}: {value}"
            );
            assert!(close(result["probability"].as_f64().unwrap(), 0.8, 1e-12));
        }
    }

    /// `decision_tree_prefers_the_popup`
    #[test]
    fn decision_tree_prefers_the_popup_by_path_and_inline() {
        for inline in [false, true] {
            let (file_path, content, includes) = bagel_source(inline, "second_shop.yaml", &[]);
            let result = json(&ok_text(ForgeMcpServer::new().decision_tree(Parameters(
                DecisionTreeRequest {
                    file_path,
                    content,
                    includes,
                },
            ))));
            assert!(close(
                result["root_expected_value"].as_f64().unwrap(),
                61_500.0,
                1e-9
            ));
            let root = &result["node_results"]["root"];
            assert_eq!(root["optimal_choice"], "popup_first");
            assert!(close(
                root["branch_values"]["open_now"].as_f64().unwrap(),
                58_500.0,
                1e-9
            ));
            let risk = &result["risk_profile"];
            assert!(close(
                risk["probability_positive"].as_f64().unwrap(),
                0.4,
                1e-12
            ));
            assert!(close(
                risk["worst_case"].as_f64().unwrap(),
                -117_000.0,
                1e-9
            ));
        }
    }

    /// `real_options_match_an_independent_binomial_tree`: Cox-Ross-Rubinstein,
    /// 200 steps, American; defer 64604.68351396034, abandon 279.7243076076951.
    #[test]
    fn real_options_match_the_binomial_tree_by_path_and_inline() {
        for inline in [false, true] {
            let (file_path, content, includes) = bagel_source(inline, "wait_option.yaml", &[]);
            let result = json(&ok_text(ForgeMcpServer::new().real_options(Parameters(
                RealOptionsRequest {
                    file_path,
                    content,
                    includes,
                },
            ))));
            let value = |name: &str| result["options"][name]["value"].as_f64().unwrap();
            assert!(close(
                value("Wait up to a year"),
                64_604.683_513_960_34,
                1e-6
            ));
            assert!(close(
                value("Sell the oven and fit-out if it fails"),
                279.724_307_607_695_1,
                1e-6
            ));
        }
    }

    /// `bootstrap_agrees_with_numpy`: 200,000 resamples of the same 30 days give
    /// mean 50.7, standard error 2.0162 and a 95% interval of [46.8, 54.667].
    #[test]
    fn bootstrap_agrees_with_numpy_by_path_and_inline() {
        for inline in [false, true] {
            let (file_path, content, includes) = bagel_source(inline, "first_month.yaml", &[]);
            let result = json(&ok_text(ForgeMcpServer::new().bootstrap(Parameters(
                BootstrapRequest {
                    file_path,
                    content,
                    includes,
                    iterations: None,
                    seed: None,
                    confidence_levels: None,
                },
            ))));
            assert!(close(
                result["original_estimate"].as_f64().unwrap(),
                50.7,
                1e-12
            ));
            let error = result["bootstrap_std_error"].as_f64().unwrap();
            assert!(close(error, 2.0162, 0.03), "inline {inline}: {error}");
            let intervals = result["confidence_intervals"].as_array().unwrap();
            let ci95 = intervals
                .iter()
                .find(|c| close(c["level"].as_f64().unwrap(), 0.95, 1e-12))
                .unwrap();
            assert!(close(ci95["lower"].as_f64().unwrap(), 46.8, 0.15));
            assert!(close(ci95["upper"].as_f64().unwrap(), 54.667, 0.15));
        }
    }

    /// `bayesian_two_parents_explain_away`: P(roadwork | slow) = 0.4 and
    /// P(roadwork | slow, rainy) = 3/11, from the joint distribution.
    #[test]
    fn bayesian_two_parents_explain_away_by_path_and_inline() {
        for inline in [false, true] {
            let p_yes = |evidence: &[&str]| {
                let (file_path, content, includes) = bagel_source(inline, "slow_week.yaml", &[]);
                let result = json(&ok_text(ForgeMcpServer::new().bayesian(Parameters(
                    BayesianRequest {
                        file_path,
                        content,
                        includes,
                        query_var: Some("roadwork".into()),
                        evidence: Some(evidence.iter().map(ToString::to_string).collect()),
                    },
                ))));
                let query = &result["queries"]["roadwork"];
                assert_eq!(query["states"], serde_json::json!(["yes", "no"]));
                query["probabilities"][0].as_f64().unwrap()
            };
            assert!(close(p_yes(&["sales=slow"]), 0.4, 1e-9), "inline {inline}");
            assert!(
                close(p_yes(&["sales=slow", "weather=rainy"]), 3.0 / 11.0, 1e-9),
                "inline {inline}"
            );
        }
    }

    /// examples/vineyard: 10,000 seasons in which the crop and the price are
    /// correlated at -0.7, as `the_season_is_what_python_gets_from_forges_own_draws`
    /// in `monte_carlo/correlation.rs` asserts them (the Python is quoted there).
    #[test]
    fn simulate_gives_the_vineyards_seasons_by_path_and_inline() {
        for inline in [false, true] {
            let (file_path, content, includes) = example_source(
                inline,
                "vineyard",
                "vineyard_risk.yaml",
                &[("costs", "costs.yaml")],
            );
            let result = json(&ok_text(ForgeMcpServer::new().simulate(Parameters(
                SimulateRequest {
                    file_path,
                    content,
                    includes,
                    iterations: None,
                    seed: None,
                    sampling: None,
                    scenario: None,
                    seek: None,
                    vary: None,
                    min: None,
                    max: None,
                    outputs: None,
                },
            ))));
            assert_eq!(result["monte_carlo_results"]["iterations"], 10_000);
            let profit = &result["monte_carlo_results"]["outputs"]["season.profit"];
            let at = |value: &serde_json::Value, want: f64, what: &str| {
                let got = value.as_f64().unwrap();
                assert!(close(got, want, 1e-6), "inline {inline}, {what}: {got}");
            };
            at(&profit["at_plan"], 72_140.0, "at plan");
            at(&profit["mean"], 42_889.015_893, "mean");
            at(&profit["median"], 39_680.667_472, "P50");
            at(&profit["percentiles"]["p5"], -58_926.619_114, "P5");
            at(&profit["percentiles"]["p95"], 153_625.617_060, "P95");
            at(&profit["thresholds"]["< 0"], 0.2555, "loss");
            at(
                &profit["expected_shortfall"][0]["mean"],
                -79_829.454_058,
                "worst 5%",
            );
            assert_eq!(profit["drivers"][0]["input"], "harvest.price_per_tonne");
            assert_eq!(profit["drivers"][2]["input"], "harvest.yield_factor");
        }
    }

    /// What every tool that reads a model says when it is given none
    const ASKS: &str = "Either file_path or content is required";

    fn request<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> Parameters<T> {
        Parameters(serde_json::from_value(value).expect("a request"))
    }

    #[test]
    fn each_uncertainty_tool_asks_for_a_model_when_it_gets_none() {
        use serde_json::json;
        let server = ForgeMcpServer::new();
        assert_eq!(server.simulate(request(json!({}))).unwrap_err(), ASKS);
        assert_eq!(server.decision_tree(request(json!({}))).unwrap_err(), ASKS);
        assert_eq!(server.real_options(request(json!({}))).unwrap_err(), ASKS);
        assert_eq!(server.bootstrap(request(json!({}))).unwrap_err(), ASKS);
        assert_eq!(server.bayesian(request(json!({}))).unwrap_err(), ASKS);
    }

    #[test]
    fn a_seek_without_what_to_vary_is_refused() {
        let server = ForgeMcpServer::new();
        let error = server
            .simulate(request(serde_json::json!({
                "file_path": "test-data/budget.yaml",
                "seek": "profit > 0 at 80%"
            })))
            .unwrap_err();
        assert_eq!(error, "seek needs vary, min and max");
    }
}
