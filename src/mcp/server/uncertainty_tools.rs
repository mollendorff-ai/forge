//! The tools for uncertainty: Monte Carlo, decision trees, real options,
//! the bootstrap and Bayesian networks.

use super::{to_json, ForgeMcpServer};
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
                #[allow(clippy::cast_precision_loss)]
                let i = (p * (sorted.len() - 1) as f64)
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
            .and_then(|r| {
                r.to_json()
                    .map_err(|e| format!("Serialization failed: {e}"))
            })
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
    use crate::mcp::server::tests::{err_text, ok_text};
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
}
