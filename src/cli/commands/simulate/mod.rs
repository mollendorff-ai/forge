//! Monte Carlo Simulation CLI Command
//!
//! Runs probabilistic analysis using MC.* distribution functions.

use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::monte_carlo::{MonteCarloConfig, MonteCarloEngine};
use crate::parser;
use colored::Colorize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod iteration;
mod outputs;
mod print;
use iteration::evaluate_iteration;
use outputs::{expand_outputs, select_outputs};
use print::{print_simulation_results, write_simulation_output};

/// What `forge simulate` runs, beyond the model file.
#[derive(Debug, Clone, Copy, Default)]
pub struct SimulateOptions<'a> {
    /// Number of iterations, instead of the file's
    pub iterations: Option<usize>,
    /// Random seed, instead of the file's
    pub seed: Option<u64>,
    /// `monte_carlo` or `latin_hypercube`, instead of the file's
    pub sampling: Option<&'a str>,
    /// A scenario of the model whose overrides apply first. An override of an
    /// `MC.*` input fixes that input at the scenario's value.
    pub scenario: Option<&'a str>,
    /// Only these outputs of `monte_carlo.outputs` (all of them when empty).
    /// The draws do not depend on the outputs, so each one's numbers are the
    /// same as in a run of all of them.
    pub outputs: &'a [String],
}

/// Run Monte Carlo simulation and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the YAML file cannot be parsed, the Monte Carlo
/// configuration is invalid, the scenario does not exist, or the simulation fails.
pub fn simulate_core(
    file: &Path,
    options: SimulateOptions<'_>,
) -> ForgeResult<crate::monte_carlo::SimulationResult> {
    let (config, model) = prepare(file, options)?;
    run_model(
        &config,
        &model,
        &crate::core::array_calculator::CalcPlan::new(),
        false,
    )
}

/// The Monte Carlo settings (with the options applied and the outputs
/// expanded) and the model (with the scenario applied).
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed, the settings are
/// invalid, the scenario does not exist, or an output names nothing.
pub(super) fn prepare(
    file: &Path,
    options: SimulateOptions<'_>,
) -> ForgeResult<(MonteCarloConfig, crate::types::ParsedModel)> {
    let yaml_content = parser::read_model_text(file)?;
    let mut config = parse_monte_carlo_config(&yaml_content)?;

    if let Some(n) = options.iterations {
        config.iterations = n;
    }
    if let Some(s) = options.seed {
        config.seed = Some(s);
    }
    if let Some(sampling) = options.sampling {
        config.sampling = sampling.to_string();
    }
    config.validate().map_err(ForgeError::Validation)?;
    config.outputs = select_outputs(&config.outputs, options.outputs)?;

    let mut model = parser::parse_model(file)?;
    if let Some(scenario) = options.scenario {
        super::apply_scenario(&mut model, scenario)?;
    }
    config.outputs = expand_outputs(&config.outputs, &model)?;
    Ok((config, model))
}

/// Simulate `model`: the same configuration and seed give the same samples,
/// so two runs that differ in one input see the same draws.
///
/// # Errors
///
/// Returns an error if a distribution cannot be parsed or an iteration fails.
pub(super) fn run_model(
    config: &MonteCarloConfig,
    model: &crate::types::ParsedModel,
    plan: &std::rc::Rc<crate::core::array_calculator::CalcPlan>,
    only_outputs: bool,
) -> ForgeResult<crate::monte_carlo::SimulationResult> {
    let mut engine = MonteCarloEngine::new(config.clone()).map_err(ForgeError::Validation)?;
    engine
        .parse_distributions_from_model(model)
        .map_err(ForgeError::Validation)?;

    let output_vars: Vec<(String, Option<(String, usize)>)> = config
        .outputs
        .iter()
        .map(|o| (o.variable.clone(), o.row.clone()))
        .collect();

    engine
        .run_with_evaluator(&mut |inputs: &HashMap<String, f64>| {
            evaluate_iteration(model, plan, &output_vars, inputs, only_outputs)
        })
        .map_err(ForgeError::Eval)
}

/// Execute the simulate command - Monte Carlo simulation
///
/// # Errors
///
/// Returns an error if the YAML file cannot be parsed, the Monte Carlo
/// configuration is invalid, or the simulation fails.
pub fn simulate(
    c: &mut Console<'_>,
    file: &Path,
    options: SimulateOptions<'_>,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "🎲 Forge - Monte Carlo Simulation".bold().green());
    outln!(c, "   File: {}", file.display());
    if let Some(scenario) = options.scenario {
        outln!(c, "   Scenario: {}", scenario.bright_blue());
    }
    outln!(c);
    if verbose {
        outln!(c, "{}", "🎲 Running simulation...".cyan());
    }

    let result = simulate_core(file, options)?;
    let config = &result.config;
    outln!(c, "   {}", "Configuration:".bold());
    outln!(
        c,
        "      Iterations: {}",
        config.iterations.to_string().bright_blue()
    );
    outln!(c, "      Sampling:   {}", config.sampling.bright_blue());
    if let Some(seed) = config.seed {
        outln!(c, "      Seed:       {}", seed.to_string().bright_blue());
    }
    outln!(c);

    print_simulation_results(c, &result);

    if let Some(output_path) = output_file {
        write_simulation_output(c, &result, &output_path)?;
    }

    outln!(c, "{}", "✅ Simulation complete".bold().green());

    Ok(())
}

/// Parse `monte_carlo` config from YAML content
fn parse_monte_carlo_config(yaml_content: &str) -> ForgeResult<MonteCarloConfig> {
    // Try to parse the monte_carlo section from the YAML
    let value: yaml_serde::Value = parser::read_yaml(yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    if let Some(mc_value) = value.get("monte_carlo") {
        let config: MonteCarloConfig = yaml_serde::from_value(mc_value.clone())
            .map_err(|e| ForgeError::Validation(format!("monte_carlo config error: {e}")))?;
        Ok(config)
    } else {
        // No monte_carlo section - use defaults
        Ok(MonteCarloConfig::default().enabled())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::console::capture;
    use crate::core::precision;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_monte_carlo_config() {
        let yaml = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 5000
  sampling: latin_hypercube
  seed: 42
  outputs:
    - variable: revenue
      percentiles: [10, 50, 90]

scalars:
  revenue:
    value: 100000
    formula: "=MC.Normal(100000, 15000)"
"#;
        let config = parse_monte_carlo_config(yaml).unwrap();
        assert!(config.enabled);
        assert_eq!(config.iterations, 5000);
        assert_eq!(config.seed, Some(42));
        assert_eq!(config.outputs.len(), 1);
    }

    #[test]
    fn test_parse_monte_carlo_config_defaults() {
        let yaml = r#"
_forge_version: "5.0.0"
scalars:
  revenue:
    value: 100000
"#;
        let config = parse_monte_carlo_config(yaml).unwrap();
        assert!(config.enabled); // Default enabled when we explicitly call it
        assert_eq!(config.iterations, 10000); // Default
    }

    #[test]
    fn test_simulate_with_mc_distributions() {
        let yaml = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 1000
  sampling: latin_hypercube
  seed: 12345
  outputs:
    - variable: revenue
      percentiles: [10, 50, 90]
      threshold: "> 90000"

scalars:
  revenue:
    value: 100000
    formula: "=MC.Normal(100000, 15000)"
"#;
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{yaml}").unwrap();

        // This test verifies the config parsing and basic flow
        // The actual simulate function requires a valid model
        let config = parse_monte_carlo_config(yaml).unwrap();
        assert!(config.enabled);
        assert_eq!(config.iterations, 1000);
        assert_eq!(config.outputs[0].variable, "revenue");
        assert_eq!(config.outputs[0].threshold, Some("> 90000".to_string()));
    }

    /// Regression test for FORGE-MC-001: MC dependent formula evaluation
    /// Before fix: dependent formulas returned 0.0 instead of calculated values
    #[test]
    fn test_mc_dependent_formula_evaluation() {
        use crate::monte_carlo::MonteCarloEngine;
        use crate::parser;

        // Create test file with dependent formula
        let yaml = r#"
_forge_version: "5.0.0"
monte_carlo:
  enabled: true
  iterations: 100
  sampling: latin_hypercube
  seed: 42
  outputs:
    - variable: result
      percentiles: [50]
scalars:
  p_sampled:
    value: null
    formula: "=MC.Triangular(0.4, 0.6, 0.8)"
  result:
    value: null
    formula: "=scalars.p_sampled * 100"
"#;
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{yaml}").unwrap();

        // Parse config and model
        let config = parse_monte_carlo_config(yaml).unwrap();
        let model = parser::parse_model(file.path()).unwrap();

        // Create engine and parse distributions
        let mut engine = MonteCarloEngine::new(config.clone()).unwrap();
        engine.parse_distributions_from_model(&model).unwrap();

        // Get output variable names
        let output_vars: Vec<(String, Option<(String, usize)>)> = config
            .outputs
            .iter()
            .map(|o| (o.variable.clone(), None))
            .collect();
        let plan = crate::core::array_calculator::CalcPlan::new();

        // Run with formula evaluation (the fix)
        let result = engine
            .run_with_evaluator(&mut |inputs: &HashMap<String, f64>| {
                evaluate_iteration(&model, &plan, &output_vars, inputs, false)
            })
            .unwrap();

        // FORGE-MC-001 fix: result should be approximately p_sampled * 100 ≈ 60
        // Before fix: result was 0.0
        let result_stats = &result.outputs["result"];
        assert!(
            result_stats.statistics.mean > 50.0,
            "Mean should be ~60 (p_sampled * 100), got {}",
            result_stats.statistics.mean
        );
        assert!(
            result_stats.statistics.mean < 70.0,
            "Mean should be ~60 (p_sampled * 100), got {}",
            result_stats.statistics.mean
        );
    }

    fn write(dir: &std::path::Path, name: &str, text: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    const SCENARIO_MODEL: &str = r#"_forge_version: "5.1.0"
a:
  x:
    formula: "=MC.Uniform(0, 10)"
  total:
    formula: "=x * 2"
t:
  n: [1, 2]
monte_carlo:
  iterations: 100
  seed: 3
  outputs:
    - variable: a.total
    - variable: t.n
scenarios:
  known_x:
    x: 4
"#;

    /// The command with a scenario, a seed, `--verbose` and `--output`: it
    /// says each, a column of data is an output row by row, and the file is
    /// written.
    #[test]
    fn the_command_says_its_scenario_seed_and_steps_and_writes_its_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "m.yaml", SCENARIO_MODEL);
        let options = SimulateOptions {
            seed: Some(9),
            scenario: Some("known_x"),
            ..SimulateOptions::default()
        };
        let json = dir.path().join("o.json");
        let (result, printed) = capture(|c| simulate(c, &path, options, Some(json.clone()), true));
        result.unwrap();
        for line in [
            "   Scenario: known_x\n",
            "🎲 Running simulation...\n",
            "      Seed:       9\n",
            "      a.total:\n         Mean:      8.00\n",
            "      t.n by row:\n",
            "            2           2.00           2.00",
            "✅ Simulation complete\n",
        ] {
            assert!(printed.out.contains(line), "{line}\n{printed:?}");
        }
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
        assert_eq!(written["monte_carlo_results"]["seed"], 9);
        assert_eq!(written["monte_carlo_results"]["iterations"], 100);
        // Without a scenario, a seed or --verbose, and with no file to write
        let unseeded = write(
            dir.path(),
            "u.yaml",
            &SCENARIO_MODEL.replace("  seed: 3\n", ""),
        );
        let (result, printed) =
            capture(|c| simulate(c, &unseeded, SimulateOptions::default(), None, false));
        result.unwrap();
        for absent in ["Scenario:", "Running simulation", "Seed:"] {
            assert!(!printed.out.contains(absent), "{absent}\n{printed:?}");
        }
        let nowhere = dir.path().join("nodir").join("o.json");
        let (result, _) = capture(|c| simulate(c, &path, options, Some(nowhere), false));
        assert_eq!(
            result.unwrap_err().to_string(),
            "IO error: No such file or directory (os error 2)"
        );
    }

    /// What cannot be read says why, and a configuration the engine refuses
    /// is refused again where the engine is started.
    #[test]
    fn a_model_or_configuration_that_cannot_be_read_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let error = |name: &str, text: &str| {
            let path = write(dir.path(), name, text);
            simulate_core(&path, SimulateOptions::default())
                .unwrap_err()
                .to_string()
        };
        assert_eq!(
            error("bad.yaml", "a: [1\n"),
            "Validation error: YAML parse error: did not find expected ',' or ']' at line 2 column 1, while parsing a flow sequence at line 1 column 4"
        );
        assert_eq!(
            error(
                "cfg.yaml",
                "_forge_version: \"5.1.0\"\nmonte_carlo:\n  iterations: lots\n"
            ),
            "Validation error: monte_carlo config error: invalid type: string \"lots\", expected usize"
        );
        assert!(error("nm.yaml", "monte_carlo:\n  iterations: 5\n")
            .starts_with("Validation error: Missing required field: _forge_version"));
        let gone = simulate_core(&dir.path().join("none.yaml"), SimulateOptions::default());
        assert_eq!(
            gone.unwrap_err().to_string(),
            "IO error: No such file or directory (os error 2)"
        );
        let path = write(dir.path(), "m.yaml", SCENARIO_MODEL);
        let (mut config, model) = prepare(&path, SimulateOptions::default()).unwrap();
        config.iterations = 0;
        let plan = crate::core::array_calculator::CalcPlan::new();
        let refused = run_model(&config, &model, &plan, false).unwrap_err();
        assert!(
            refused.to_string().starts_with("Validation error:"),
            "{refused}"
        );
    }

    /// A game's result is an output (ADR-068): in each run the human audits
    /// with gain / (gain + penalty), by hand and as nashpy 0.0.43 finds,
    /// for that run's penalty.
    #[test]
    fn a_games_result_is_simulated_run_by_run() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "m.yaml",
            r#"_forge_version: "5.5.0"
i:
  penalty:
    formula: "=MC.Uniform(2, 8)"
games:
  g:
    players: [ai, human]
    strategies: {ai: [comply, cheat], human: [audit, trust]}
    payoffs:
      comply: {audit: [0, -1], trust: [0, 0]}
      cheat: {audit: [=-i.penalty, -1], trust: [2, -3]}
monte_carlo:
  iterations: 100
  seed: 7
  outputs:
    - variable: g.equilibrium_1.human.audit
"#,
        );
        let result = simulate_core(&path, SimulateOptions::default()).unwrap();
        let audit = &result.outputs["g.equilibrium_1.human.audit"].samples;
        let penalty = &result.input_samples["i.penalty"];
        assert_eq!(audit.len(), 100);
        for (a, p) in audit.iter().zip(penalty) {
            assert!((a - 2.0 / (2.0 + p)).abs() < 1e-12, "{a} at {p}");
        }
    }

    /// A scenario that overrides an MC.* input fixes it: it is no longer
    /// sampled, and drives nothing.
    #[test]
    fn scenario_fixes_an_uncertain_input() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "m.yaml",
            r#"_forge_version: "5.1.0"
a:
  x:
    formula: "=MC.Uniform(0, 10)"
  y:
    formula: "=MC.Uniform(0, 1)"
  total:
    formula: "=x + y"
monte_carlo:
  iterations: 200
  seed: 3
  outputs:
    - variable: a.total
      thresholds: ["> 5", "<= 1"]
scenarios:
  known_x:
    x: 4
"#,
        );
        let base = simulate_core(&path, SimulateOptions::default()).unwrap();
        let drivers: Vec<&str> = base.outputs["a.total"]
            .drivers
            .iter()
            .map(|d| d.input.as_str())
            .collect();
        assert_eq!(drivers, ["a.x", "a.y"], "x varies ten times more");
        let thresholds: Vec<&String> = base.outputs["a.total"]
            .threshold_probabilities
            .keys()
            .collect();
        assert_eq!(thresholds, ["> 5", "<= 1"], "in the order written");

        let fixed = simulate_core(
            &path,
            SimulateOptions {
                scenario: Some("known_x"),
                ..SimulateOptions::default()
            },
        )
        .unwrap();
        assert!(!fixed.input_samples.contains_key("a.x"));
        let total = &fixed.outputs["a.total"];
        assert!(total.samples.iter().all(|t| (4.0..=5.0).contains(t)));
        assert!(total.threshold_probabilities["> 5"].abs() < f64::EPSILON);
        assert_eq!(total.drivers.len(), 1);
        assert_eq!(total.drivers[0].input, "a.y");
        assert!((total.drivers[0].rank_correlation - 1.0).abs() < 1e-12);

        let err = simulate_core(
            &path,
            SimulateOptions {
                scenario: Some("nope"),
                ..SimulateOptions::default()
            },
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("known_x"), "{err}");
    }

    /// An MC.* input can be in an included file: it is sampled as `@ns.name`,
    /// and the formulas of that file that read it follow.
    #[test]
    fn uncertain_input_in_an_included_file() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "rates.yaml",
            "_forge_version: \"5.1.0\"\nfx:\n  usd:\n    formula: \"=MC.Uniform(1.3, 1.4)\"\n  usd_with_fee:\n    formula: \"=usd * 1.01\"\n",
        );
        let path = write(
            dir.path(),
            "m.yaml",
            "_forge_version: \"5.1.0\"\n_includes:\n  - file: rates.yaml\n    as: rates\nbuy:\n  cad:\n    formula: \"=100 * @rates.fx.usd_with_fee\"\nmonte_carlo:\n  iterations: 100\n  seed: 1\n  outputs:\n    - variable: buy.cad\n",
        );
        let result = simulate_core(&path, SimulateOptions::default()).unwrap();
        let usd = &result.input_samples["@rates.fx.usd"];
        let cad = &result.outputs["buy.cad"].samples;
        for (u, c) in usd.iter().zip(cad) {
            assert!((c - 100.0 * u * 1.01).abs() < 1e-9, "{u} {c}");
        }
        assert_eq!(result.outputs["buy.cad"].drivers[0].input, "@rates.fx.usd");
    }

    #[test]
    fn a_bad_threshold_in_the_list_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "m.yaml",
            "_forge_version: \"5.1.0\"\na:\n  x:\n    formula: \"=MC.Uniform(0, 1)\"\nmonte_carlo:\n  iterations: 10\n  outputs:\n    - variable: a.x\n      thresholds: [\"> 0\", \"about 3\"]\n",
        );
        let err = simulate_core(&path, SimulateOptions::default())
            .unwrap_err()
            .to_string();
        assert!(err.contains("a.x"), "{err}");
    }

    /// A correlation can pair an input of an included file with one of the
    /// model (Gaussian copula: Spearman 6/pi asin(0.6/2) = 0.58 for 0.6).
    #[test]
    fn correlation_across_an_include() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "costs.yaml",
            "_forge_version: \"5.1.0\"\nlabour:\n  wage:\n    formula: \"=MC.Uniform(17, 21)\"\n",
        );
        let path = write(
            dir.path(),
            "m.yaml",
            "_forge_version: \"5.1.0\"\n_includes:\n  - file: costs.yaml\n    as: costs\nshop:\n  hours:\n    formula: \"=MC.Normal(30, 3)\"\n  pay:\n    formula: \"=hours * @costs.labour.wage\"\nmonte_carlo:\n  iterations: 4000\n  seed: 11\n  correlations:\n    - variables: [wage, hours]\n      coefficient: 0.6\n  outputs:\n    - variable: shop.pay\n",
        );
        let result = simulate_core(&path, SimulateOptions::default()).unwrap();
        let rho = crate::monte_carlo::sensitivity::spearman_correlation(
            &result.input_samples["@costs.labour.wage"],
            &result.input_samples["shop.hours"],
        );
        let expected = 6.0 / std::f64::consts::PI * (0.3_f64).asin();
        assert!((rho - expected).abs() < 0.03, "{rho} vs {expected}");
    }

    /// The worst runs of a cost are its highest (`better: lower`).
    #[test]
    fn expected_shortfall_follows_better() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "m.yaml",
            "_forge_version: \"5.1.0\"\na:\n  x:\n    formula: \"=MC.Uniform(0, 100)\"\n  cost:\n    formula: \"=x\"\n    better: lower\n  gain:\n    formula: \"=x\"\nmonte_carlo:\n  iterations: 1000\n  seed: 2\n  outputs:\n    - variable: a.cost\n      expected_shortfall: [10]\n    - variable: a.gain\n      expected_shortfall: [10]\n",
        );
        let result = simulate_core(&path, SimulateOptions::default()).unwrap();
        let cost = result.outputs["a.cost"].expected_shortfall[0].1;
        let gain = result.outputs["a.gain"].expected_shortfall[0].1;
        // Latin hypercube: one draw in each 1% band, so the worst 10% average about 95 and 5
        assert!((cost - 95.0).abs() < 0.5, "{cost}");
        assert!((gain - 5.0).abs() < 0.5, "{gain}");
    }

    #[test]
    fn an_output_that_names_nothing_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "m.yaml",
            "_forge_version: \"5.1.0\"\na:\n  x:\n    formula: \"=MC.Uniform(0, 1)\"\nmonte_carlo:\n  iterations: 100\n  outputs:\n    - variable: a.nope\n",
        );
        let err = simulate_core(&path, SimulateOptions::default())
            .unwrap_err()
            .to_string();
        assert!(err.contains("neither a scalar nor a table column"), "{err}");
        // A column its table does not have is the same
        let path = write(
            dir.path(),
            "t.yaml",
            "_forge_version: \"5.1.0\"\nt:\n  n: [1, 2]\nmonte_carlo:\n  iterations: 100\n  outputs:\n    - variable: t.nope\n",
        );
        let err = simulate_core(&path, SimulateOptions::default())
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("'t.nope' is neither a scalar nor a table column"),
            "{err}"
        );
    }

    #[test]
    fn seek_says_when_the_goal_is_out_of_range_or_the_input_is_a_formula() {
        use super::super::{probability_seek_core, SeekGoal};
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "m.yaml",
            "_forge_version: \"5.1.0\"\na:\n  price: 10\n  units:\n    formula: \"=MC.Uniform(80, 120)\"\n  profit:\n    formula: \"=units * price - 1000\"\n  double:\n    formula: \"=price * 2\"\nmonte_carlo:\n  iterations: 500\n  seed: 4\n  outputs:\n    - variable: a.profit\n",
        );
        let goal = SeekGoal::parse("P(a.profit > 0) = 0.5").unwrap();
        // profit > 0 when price > 1000 / units: half the runs at 1000/100 = 10
        let found =
            probability_seek_core(&path, SimulateOptions::default(), &goal, "price", 5.0, 20.0)
                .unwrap();
        assert!((found.value - 10.0).abs() < 0.05, "{}", found.value);
        let err = probability_seek_core(
            &path,
            SimulateOptions::default(),
            &goal,
            "price",
            20.0,
            30.0,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("not crossed"), "{err}");
        let err =
            probability_seek_core(&path, SimulateOptions::default(), &goal, "double", 1.0, 2.0)
                .unwrap_err()
                .to_string();
        assert!(err.contains("has a formula"), "{err}");
    }
    // From tests/simulate_outputs.rs and tests/simulate_plan.rs (roadmap
    // T1-TESTS-INLINE). `--output-var` shows only the outputs named
    // (T1-SIM-SHOW-ONE): the draws do not depend on the outputs, so an
    // output's numbers are the same with the flag as without it. `forge
    // simulate` shows each uncertain input's plan value beside its mean, and
    // each output's value at plan beside its mean (T1-SIM-MEAN-VS-PLAN); the
    // figures are checked against closed forms, not against Forge:
    // - a triangular's mean is (min + mode + max) / 3, a uniform's
    //   (min + max) / 2 and a PERT's (min + 4 mode + max) / 6;
    // - profit is linear in staff hours, the wage and the flour price, drawn
    //   independently, so the expected profit is the plan's profit plus each
    //   input's (mean - plan) times its coefficient: labour (E[hours] E[wage]
    //   - 32 x 18.50) x 30 days x 1.15 load x 12 months, flour the plan's year
    //   of dozens x 12 / 9 bagels a kg x (E[flour] - 1.15). Opening demand is
    //   a symmetric PERT (mean 55 = plan), so it adds nothing.

    const BAGEL_SIMULATE: &str = "examples/bagel/bagel_simulate.yaml";

    fn bagel_with(
        outputs: &[String],
        iterations: Option<usize>,
    ) -> crate::monte_carlo::SimulationResult {
        simulate_core(
            Path::new(BAGEL_SIMULATE),
            SimulateOptions {
                iterations,
                outputs,
                ..SimulateOptions::default()
            },
        )
        .expect("simulates")
    }

    /// What `forge simulate` prints for these options, without colour codes
    /// (an in-process run may see a terminal, which a piped binary did not)
    fn printed(options: SimulateOptions<'_>) -> ForgeResult<String> {
        let (result, printed) =
            capture(|c| simulate(c, Path::new(BAGEL_SIMULATE), options, None, false));
        result.map(|()| printed.out)
    }

    // The same draws give the same numbers, to the last bit
    #[test]
    fn one_output_has_the_numbers_it_has_among_all() {
        let all = bagel_with(&[], Some(500));
        let one = bagel_with(&["year.profit".to_string()], Some(500));
        assert_eq!(one.outputs.keys().collect::<Vec<_>>(), ["year.profit"]);
        let (a, b) = (&all.outputs["year.profit"], &one.outputs["year.profit"]);
        assert_eq!(a.samples, b.samples);
        assert_eq!(a.statistics.mean.to_bits(), b.statistics.mean.to_bits());
        assert_eq!(a.statistics.percentiles, b.statistics.percentiles);
        assert_eq!(a.threshold_probabilities, b.threshold_probabilities);
        assert_eq!(a.expected_shortfall, b.expected_shortfall);
        let drivers = |r: &crate::monte_carlo::OutputResult| {
            r.drivers
                .iter()
                .map(|d| (d.input.clone(), d.rank_correlation))
                .collect::<Vec<_>>()
        };
        assert_eq!(drivers(a), drivers(b));
    }

    #[test]
    fn a_name_without_its_group_and_a_column_choose_their_outputs() {
        let worst = bagel_with(&["worst_month_profit".to_string()], Some(500));
        assert_eq!(
            worst.outputs.keys().collect::<Vec<_>>(),
            ["year.worst_month_profit"]
        );
        // A table column is all its rows, in the order configured
        let months = bagel_with(
            &["months.profit".to_string(), "year.profit".to_string()],
            Some(500),
        );
        let names: Vec<&String> = months.outputs.keys().collect();
        assert_eq!(names.len(), 13, "{names:?}");
        assert_eq!(names[0], "year.profit");
        assert_eq!(names[1], "months.profit (row 1)");
        assert_eq!(names[12], "months.profit (row 12)");
    }

    #[test]
    fn an_unknown_or_ambiguous_name_is_refused_with_the_outputs() {
        let err = |names: &[&str]| {
            let names: Vec<String> = names.iter().map(ToString::to_string).collect();
            simulate_core(
                Path::new(BAGEL_SIMULATE),
                SimulateOptions {
                    iterations: Some(10),
                    outputs: &names,
                    ..SimulateOptions::default()
                },
            )
            .expect_err("refused")
            .to_string()
        };
        assert_eq!(
            err(&["revenue"]),
            "Validation error: --output-var revenue: not an output of monte_carlo (outputs: \
             year.profit, year.worst_month_profit, months.profit)"
        );
        assert_eq!(
            err(&["profit"]),
            "Validation error: --output-var profit: more than one output ends with it \
             (year.profit, months.profit); write the whole name"
        );
    }

    #[test]
    fn the_flag_prints_only_that_output() {
        let outputs = ["year.profit".to_string()];
        let text = printed(SimulateOptions {
            iterations: Some(200),
            outputs: &outputs,
            ..SimulateOptions::default()
        })
        .expect("simulates");
        assert!(text.contains("year.profit:"), "{text}");
        assert!(text.contains("P(year.profit > 0) = "), "{text}");
        assert!(!text.contains("year.worst_month_profit:"), "{text}");
        assert!(!text.contains("months.profit by row"), "{text}");
    }

    #[test]
    fn an_unknown_name_is_an_error_that_lists_the_outputs() {
        // main prints any error as "Error: ..." and exits 1
        let outputs = ["nope".to_string()];
        let message = printed(SimulateOptions {
            outputs: &outputs,
            ..SimulateOptions::default()
        })
        .expect_err("refused")
        .to_string();
        assert!(
            message.contains(
                "--output-var nope: not an output of monte_carlo (outputs: year.profit, \
                 year.worst_month_profit, months.profit)"
            ),
            "{message}"
        );
    }

    #[test]
    fn the_value_at_plan_is_the_plans_profit() {
        let result = bagel_with(&["year.profit".to_string()], None);
        let at_plan = result.outputs["year.profit"].at_plan.expect("calculated");
        let plan =
            crate::cli::calculate_core(Path::new("examples/bagel/bagel.yaml"), true, None).unwrap();
        let planned = plan.scalars["year.profit"].unwrap();
        assert!(
            (at_plan - planned).abs() < 1e-9,
            "{at_plan} against {planned}"
        );
        assert!((at_plan - 786.320_361_7).abs() < 1e-6, "{at_plan}");
    }

    #[test]
    fn the_inputs_plan_values_and_means_are_the_closed_forms() {
        let result = bagel_with(&["year.profit".to_string()], None);
        let mean = |name: &str| {
            let s = &result.input_samples[name];
            let n = precision::count(s.len());
            s.iter().sum::<f64>() / n
        };
        let cases = [
            // (input, plan, closed-form mean)
            (
                "assumptions.staff_hours_per_day",
                32.0,
                (28.0 + 32.0 + 38.0) / 3.0,
            ),
            (
                "@costs.ingredients.flour_per_kg",
                1.15,
                (1.05 + 1.15 + 1.70) / 3.0,
            ),
            (
                "@costs.labour.wage_per_hour",
                18.50,
                f64::midpoint(17.50, 20.50),
            ),
            (
                "assumptions.opening_dozens_per_day",
                55.0,
                4.0f64.mul_add(55.0, 40.0 + 70.0) / 6.0,
            ),
        ];
        for (name, plan, closed_form) in cases {
            let p = &result.plan_inputs[name];
            assert!((p.value - plan).abs() < 1e-12, "{name}: plan {}", p.value);
            assert_eq!(p.from, "value", "{name}");
            // Latin hypercube with 5,000 draws: the mean is within 0.1%
            let m = mean(name);
            assert!(
                (m - closed_form).abs() <= closed_form * 1e-3,
                "{name}: sampled mean {m}, closed form {closed_form}"
            );
        }
    }

    #[test]
    fn the_gap_between_mean_and_plan_is_the_inputs_means() {
        let result = bagel_with(&["year.profit".to_string()], None);
        let year = &result.outputs["year.profit"];
        let at_plan = year.at_plan.expect("calculated");
        let hours: f64 = (28.0 + 32.0 + 38.0) / 3.0;
        let wage = f64::midpoint(17.50, 20.50);
        let labour = hours.mul_add(wage, -32.0 * 18.50) * 30.0 * 1.15 * 12.0;
        // The plan's year of dozens, from the independent Python in bagel_example.rs
        let dozens = 24_176.715_596_114_2;
        let flour = dozens * 12.0 / 9.0 * ((1.05 + 1.15 + 1.70) / 3.0 - 1.15);
        let expected_gap = -(labour + flour); // -16,703.34
        assert!((expected_gap + 16_703.34).abs() < 0.01, "{expected_gap}");
        let gap = year.statistics.mean - at_plan;
        // Sampling error of the mean: under 0.1% of a standard deviation of 38,058
        assert!(
            (gap - expected_gap).abs() < 40.0,
            "simulated gap {gap}, closed form {expected_gap}"
        );
    }

    #[test]
    fn simulate_prints_the_plan_beside_the_mean() {
        let outputs = ["year.profit".to_string()];
        let text = printed(SimulateOptions {
            outputs: &outputs,
            ..SimulateOptions::default()
        })
        .expect("simulates");
        for line in [
            "assumptions.staff_hours_per_day mean=32.67 plan=32.00 (value)",
            "@costs.ingredients.flour_per_kg mean=1.30 plan=1.15 (value)",
            "@costs.labour.wage_per_hour mean=19.00 plan=18.50 (value)",
            "At plan:   786.32  (each uncertain input at its plan value)",
            "the uncertain inputs' means are not their plan values",
        ] {
            assert!(text.contains(line), "missing {line:?} in:\n{text}");
        }
    }

    mod enabled {
        //! `monte_carlo.enabled` is accepted but `forge simulate` never reads it: a
        //! model whose section says `enabled: false`, or leaves it out, still runs.
        //! docs/MCP.md says so; this keeps the doc honest.

        use crate::cli::forge;

        fn model(enabled_line: &str) -> String {
            format!(
                r#"_forge_version: "5.1.0"
monte_carlo:
{enabled_line}  iterations: 100
  seed: 42
  outputs:
    - variable: revenue
      percentiles: [50]
revenue:
  value: 100000
  formula: "=MC.Normal(100000, 15000)"
"#
            )
        }

        #[test]
        fn simulate_runs_without_enabled_and_with_enabled_false() {
            for line in ["", "  enabled: false\n", "  enabled: true\n"] {
                let dir = tempfile::tempdir().expect("temp dir");
                let file = dir.path().join("mc.yaml");
                std::fs::write(&file, model(line)).expect("write");
                let ran = forge(&["simulate", file.to_str().expect("path")]);
                assert!(ran.success(), "{line:?}: {ran:?}");
                assert!(
                    ran.out.contains("Simulation complete"),
                    "{line:?}: {}",
                    ran.out
                );
            }
        }

        #[test]
        fn mcp_md_does_not_say_enabled_is_required() {
            let doc = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/MCP.md"))
                .expect("docs/MCP.md");
            assert!(
                !doc.contains("with `enabled: true`"),
                "docs/MCP.md says a Monte Carlo section needs enabled: true; forge simulate never reads it"
            );
        }
    }

    mod printed_numbers {
        //! `forge simulate` prints its statistics as `forge report` prints numbers:
        //! two decimals and thousands separators (the end-to-end pass of 2026-11-12).

        use crate::cli::forge;

        #[test]
        fn simulate_prints_statistics_with_separators_and_two_decimals() {
            let text = forge(&["simulate", "examples/bagel/bagel_simulate.yaml"]).printed();
            for line in [
                "Mean:      -15,925.94",
                "Median:    -15,689.18",
                "P5: -78,477.56",
                "(expected shortfall): -91,760.43",
                "assumptions.opening_dozens_per_day mean=55.00 plan=55.00 (value) min=40.79 max=69.73",
            ] {
                assert!(text.contains(line), "{line}\n{text}");
            }
        }
    }

    mod microgrid_example {
        //! `examples/microgrid/microgrid_risk.yaml`: futures for Ilha Corva. The
        //! README quotes the file's 10,000 runs; this runs 1,000 of them, which
        //! keep the README's claims: the project is worth more than nothing in
        //! every future, the loan's cover never falls below 1.2, and the diesel
        //! price drives the value most. Three of the futures are calculated
        //! again by the plain model with that future's inputs, so the simulation
        //! is shown to run the model and nothing else.

        use crate::cli::{simulate_core, SimulateOptions};
        use crate::core::ArrayCalculator;
        use std::path::Path;

        const MODEL: &str = "examples/microgrid/microgrid_risk.yaml";

        #[test]
        fn the_microgrid_pays_in_every_future_and_the_loan_is_always_covered() {
            let options = SimulateOptions {
                iterations: Some(1000),
                ..SimulateOptions::default()
            };
            let result = simulate_core(Path::new(MODEL), options).unwrap();
            assert_eq!(result.iterations_completed, 1000);
            let npv = &result.outputs["verdict.project_npv"];
            let p = &npv.threshold_probabilities;
            assert!((p["> 0"] - 1.0).abs() <= 0.0, "{p:?}");
            assert!(npv.statistics.min > 0.0, "{}", npv.statistics.min);
            let dscr = &result.outputs["verdict.min_dscr"].threshold_probabilities;
            assert!(dscr["< 1.2"].abs() <= 0.0, "{dscr:?}");
            assert!(dscr["< 1.0"].abs() <= 0.0, "{dscr:?}");
            // The diesel price drives the value most
            assert_eq!(npv.drivers[0].input, "fuel.diesel_price");

            for run in [0, 499, 999] {
                let mut model = crate::parser::parse_model(Path::new(MODEL)).unwrap();
                for (input, samples) in &result.input_samples {
                    model
                        .override_scalar(input, Some(samples[run]), None)
                        .unwrap();
                }
                let calculated = ArrayCalculator::new(model).calculate_all().unwrap();
                let value = calculated.scalars["verdict.project_npv"].value.unwrap();
                assert!(
                    (value - npv.samples[run]).abs() <= 1e-6,
                    "run {run}: {value}"
                );
            }
        }
    }

    mod realestate_example {
        //! `examples/realestate/realestate_risk.yaml`: futures for Linden Court.
        //! The README quotes the file's 10,000 runs; this runs 1,000. Three of the
        //! uncertain inputs lean against the buyer (vacancy, the exit cap rate and
        //! the running costs have their mean on the bad side of the plan), so the
        //! mean value falls below the value at plan, which is realestate.yaml's
        //! (prediction/scenarios.rs, `realestate_example`). Rent growth drives the
        //! value most, as in the tornado. Three of the futures are calculated again
        //! by the plain model with that future's inputs.

        use crate::cli::{simulate_core, SimulateOptions};
        use crate::core::ArrayCalculator;
        use std::path::Path;

        const MODEL: &str = "examples/realestate/realestate_risk.yaml";

        #[test]
        fn the_futures_lean_below_the_plan_and_rent_growth_drives_them() {
            let options = SimulateOptions {
                iterations: Some(1000),
                ..SimulateOptions::default()
            };
            let result = simulate_core(Path::new(MODEL), options).unwrap();
            assert_eq!(result.iterations_completed, 1000);
            let npv = &result.outputs["valuation.npv"];
            let at_plan = npv.at_plan.unwrap();
            assert!(
                (at_plan - 155_715.314_059_857_28).abs() <= 1e-6,
                "{at_plan}"
            );
            assert!(npv.statistics.mean < at_plan, "{}", npv.statistics.mean);
            assert_eq!(npv.drivers[0].input, "market.rent_growth");
            // The lender's cover depends on this year's rent and costs, not on growth
            let dscr = &result.outputs["returns.min_dscr"];
            let mut first_two = [
                dscr.drivers[0].input.as_str(),
                dscr.drivers[1].input.as_str(),
            ];
            first_two.sort_unstable();
            assert_eq!(
                first_two,
                ["building.opex_per_unit_year", "building.vacancy"]
            );

            for run in [0, 499, 999] {
                let mut model = crate::parser::parse_model(Path::new(MODEL)).unwrap();
                for (input, samples) in &result.input_samples {
                    model
                        .override_scalar(input, Some(samples[run]), None)
                        .unwrap();
                }
                let calculated = ArrayCalculator::new(model).calculate_all().unwrap();
                let value = calculated.scalars["valuation.npv"].value.unwrap();
                assert!(
                    (value - npv.samples[run]).abs() <= 1e-6,
                    "run {run}: {value}"
                );
            }
        }
    }

    mod realestate_flexibility {
        //! `examples/realestate/realestate_flex.yaml`: Linden Court bought for cash,
        //! in a market that moves year by year, with the right to sell early.
        //!
        //! One market path, written from the model's description and not from its
        //! formulas:
        //! ```python
        //! def run(shocks, cap=0.0575, reversion=0.3, sell_above=1.35, price=4.6e6):
        //!     gaps, g = [], 0.0
        //!     for y in range(1, 12):  # the gap from the trend keeps 1 - reversion
        //!         g = g * (1 - reversion) + (shocks.get(y, 0.0) if y > 1 else 0.0)
        //!         gaps.append(g)
        //!     def noi(y):
        //!         idx = 1.03 ** (y - 1) * (1 + gaps[y - 1])
        //!         income = 24 * 12 * 1510 * idx - 24 * 12 * 1450 * idx * 0.07
        //!         return income * 0.96 - 24 * 5200 * 1.025 ** (y - 1)
        //!     cash = lambda y: noi(y) - 24 * 350 * 1.025 ** (y - 1)
        //!     sale = lambda y: noi(y + 1) / cap * 0.98
        //!     npv = lambda t: sum(cash(y) / 1.08 ** y for y in range(1, t + 1)) \
        //!         + sale(t) / 1.08 ** t - price * 1.015
        //!     t = next((y for y in range(3, 10) if sale(y) >= sell_above * price), 10)
        //!     return npv(10), npv(t), t
        //! run({})  # the plan: 155715.31405985728 both ways, sold in year 10
        //! run({2: -0.10, 3: -0.05, 6: 0.15, 7: 0.10})
        //!          # a bust, then a boom: 359141.3958009919 held, 840042.2399097551
        //!          # sold in year 6
        //! run({2: 0.12, 3: 0.10}, reversion=0.0)
        //!          # an early boom that lasts: 1597783.8654967323 held,
        //!          # 1330940.9913548753 sold in year 3
        //! ```
        //! The shocks are one column, `years.shock`, drawn for each row on its own.
        //! The simulation is this function over 10,000 paths. With cycles the
        //! right to sell early is worth more than nothing on average; in a random
        //! walk it is worth less, since selling gives up growth as likely to last
        //! as to reverse.

        use crate::cli::{simulate_core, SimulateOptions};
        use crate::core::ArrayCalculator;
        use std::path::Path;

        const MODEL: &str = "examples/realestate/realestate_flex.yaml";

        fn path(shocks: &[(usize, f64)], reversion: f64) -> (f64, f64, f64) {
            let mut model = crate::parser::parse_model(Path::new(MODEL)).unwrap();
            // Year 1's shock is drawn but not used, so it is set to 0 here
            for year in 1..=11 {
                let shock = shocks
                    .iter()
                    .find(|(y, _)| *y == year)
                    .map_or(0.0, |(_, s)| *s);
                model
                    .set_input(&format!("years.shock (row {year})"), shock)
                    .unwrap();
            }
            model
                .override_scalar("market.exit_cap_rate", Some(0.0575), None)
                .unwrap();
            model
                .override_scalar("market.reversion", Some(reversion), None)
                .unwrap();
            let calculated = ArrayCalculator::new(model).calculate_all().unwrap();
            let get = |name: &str| calculated.scalars[name].value.unwrap();
            (
                get("valuation.npv_hold"),
                get("valuation.npv_flex"),
                get("decision.sale_year"),
            )
        }

        fn close(a: f64, b: f64) -> bool {
            (a - b).abs() <= 1e-6
        }

        #[test]
        fn each_path_matches_the_independent_calculation() {
            let cases = [
                (
                    vec![],
                    0.3,
                    155_715.314_059_857_28,
                    155_715.314_059_857_28,
                    10.0,
                ),
                (
                    vec![(2, -0.10), (3, -0.05), (6, 0.15), (7, 0.10)],
                    0.3,
                    359_141.395_800_991_9,
                    840_042.239_909_755_1,
                    6.0,
                ),
                (
                    vec![(2, 0.12), (3, 0.10)],
                    0.0,
                    1_597_783.865_496_732_3,
                    1_330_940.991_354_875_3,
                    3.0,
                ),
            ];
            for (shocks, reversion, hold, flex, year) in cases {
                let (got_hold, got_flex, got_year) = path(&shocks, reversion);
                assert!(close(got_hold, hold), "{shocks:?}: hold {got_hold}");
                assert!(close(got_flex, flex), "{shocks:?}: flex {got_flex}");
                assert!(close(got_year, year), "{shocks:?}: year {got_year}");
            }
        }

        fn mean_flexibility(scenario: Option<&str>) -> f64 {
            let options = SimulateOptions {
                iterations: Some(1000),
                scenario,
                ..SimulateOptions::default()
            };
            let result = simulate_core(Path::new(MODEL), options).unwrap();
            let flexibility = &result.outputs["valuation.flexibility"];
            // At plan the market follows its trend, the rule never fires, and
            // the right to sell early is worth nothing
            assert!(flexibility.at_plan.unwrap().abs() <= 1e-6);
            // Three futures, calculated again by the plain model
            let npv = &result.outputs["valuation.npv_flex"];
            for run in [0, 499, 999] {
                let mut model = crate::parser::parse_model(Path::new(MODEL)).unwrap();
                if scenario == Some("random_walk") {
                    model
                        .override_scalar("market.reversion", Some(0.0), None)
                        .unwrap();
                }
                for (input, samples) in &result.input_samples {
                    model.set_input(input, samples[run]).unwrap();
                }
                let calculated = ArrayCalculator::new(model).calculate_all().unwrap();
                let value = calculated.scalars["valuation.npv_flex"].value.unwrap();
                assert!(close(value, npv.samples[run]), "run {run}: {value}");
            }
            flexibility.statistics.mean
        }

        #[test]
        fn selling_early_pays_in_a_market_with_cycles_and_not_in_a_random_walk() {
            let cycles = mean_flexibility(None);
            assert!(cycles > 50_000.0, "{cycles}");
            let random_walk = mean_flexibility(Some("random_walk"));
            assert!(random_walk < -20_000.0, "{random_walk}");
        }
    }

    mod uncertain_rows {
        //! A table column whose formula is a distribution draws each row on its
        //! own. Three rows of MC.Normal(0, 0.1): their sum has a standard
        //! deviation of sqrt(3) * 0.1 = 0.1732, where one draw for the whole
        //! column would give 3 * 0.1 = 0.3, and two rows are not correlated.

        use super::super::{simulate_core, SimulateOptions};
        use super::write;

        const MODEL: &str = r#"_forge_version: "5.2.0"
t:
  n: [1, 2, 3]
  shock: "=MC.Normal(0, 0.1)"
  level: "=10 + shock"
s:
  total: {formula: "=SUM(t.shock)"}
monte_carlo:
  iterations: 2000
  seed: 5
  outputs:
    - variable: s.total
    - variable: t.level
"#;

        #[test]
        fn each_row_is_drawn_on_its_own() {
            let dir = tempfile::tempdir().unwrap();
            let path = write(dir.path(), "m.yaml", MODEL);
            let result = simulate_core(&path, SimulateOptions::default()).unwrap();
            let mut names: Vec<&String> = result.input_samples.keys().collect();
            names.sort();
            assert_eq!(
                names,
                ["t.shock (row 1)", "t.shock (row 2)", "t.shock (row 3)"]
            );

            let total = &result.outputs["s.total"];
            let sd = total.statistics.std_dev;
            assert!((sd - 0.173_205).abs() < 0.01, "{sd}");
            assert!(total.at_plan.unwrap().abs() <= 1e-12);
            // Each future's total is the sum of its three draws
            let rows: Vec<&Vec<f64>> = (1..=3)
                .map(|r| &result.input_samples[&format!("t.shock (row {r})")])
                .collect();
            for (i, total) in total.samples.iter().enumerate() {
                let sum: f64 = rows.iter().map(|r| r[i]).sum();
                assert!((total - sum).abs() <= 1e-12, "run {i}");
            }
            // A column that reads the draws gets that row's draw
            let level = &result.outputs["t.level (row 2)"].samples;
            for (l, s) in level.iter().zip(rows[1]) {
                assert!((l - 10.0 - s).abs() <= 1e-12);
            }
            // Rows 1 and 2 are independent
            let mean = |v: &[f64]| v.iter().sum::<f64>() / 2000.0;
            let (a, b) = (rows[0], rows[1]);
            let (ma, mb) = (mean(a), mean(b));
            let cov: f64 = a
                .iter()
                .zip(b)
                .map(|(x, y)| (x - ma) * (y - mb))
                .sum::<f64>()
                / 2000.0;
            let corr = cov / (0.1 * 0.1);
            assert!(corr.abs() < 0.1, "{corr}");
        }

        #[test]
        fn calculate_names_the_column_it_cannot_draw() {
            let dir = tempfile::tempdir().unwrap();
            let path = write(dir.path(), "m.yaml", MODEL);
            let err = crate::cli::calculate_core(&path, true, None)
                .unwrap_err()
                .to_string();
            assert!(err.contains("t.shock use MC.* distributions"), "{err}");
        }
    }

    mod acquisition_example {
        //! `examples/acquisition/deal_risk.yaml`: futures for Project Kittiwake.
        //! The README quotes the file's 10,000 runs; this runs 1,000. Every
        //! uncertain synergy leans against Valdane (PERT(20, 40, 50) has mean
        //! 38.33, triangular(0, 60, 80) 46.67 and triangular(90, 100, 170) 120),
        //! so the mean value falls below the value at plan, which is deal.yaml's
        //! (prediction/scenarios.rs, `acquisition_example`). Quillon's own growth
        //! drives the value most. The earn-out pays when Quillon grows at least
        //! 9% a year, so in 1 - Phi((0.09 - 0.07) / 0.025) = 0.2119 of futures,
        //! and it takes the worst of them away. Three of the futures are
        //! calculated again by the plain model with that future's inputs.

        use crate::cli::{simulate_core, SimulateOptions};
        use crate::core::ArrayCalculator;
        use std::path::Path;

        const MODEL: &str = "examples/acquisition/deal_risk.yaml";

        fn run(scenario: Option<&str>) -> crate::monte_carlo::SimulationResult {
            let options = SimulateOptions {
                iterations: Some(1000),
                scenario,
                ..SimulateOptions::default()
            };
            simulate_core(Path::new(MODEL), options).unwrap()
        }

        #[test]
        fn the_synergies_lean_short_and_quillons_growth_drives_the_value() {
            let result = run(None);
            assert_eq!(result.iterations_completed, 1000);
            let value = &result.outputs["valuation.value_created"];
            let at_plan = value.at_plan.unwrap();
            assert!(
                (at_plan - 224.507_933_418_290_12).abs() <= 1e-9,
                "{at_plan}"
            );
            assert!(
                value.statistics.mean < at_plan - 50.0,
                "{}",
                value.statistics.mean
            );
            assert_eq!(value.drivers[0].input, "case.quillon_growth");
            let synergies = &result.outputs["valuation.synergies"];
            assert!(synergies.statistics.mean < synergies.at_plan.unwrap() - 50.0);
            // Switched off, the earn-out is never paid
            let paid = &result.outputs["earnout.paid"];
            assert!(paid.statistics.max.abs() <= 0.0, "{}", paid.statistics.max);

            for future in [0, 499, 999] {
                let mut model = crate::parser::parse_model(Path::new(MODEL)).unwrap();
                for (input, samples) in &result.input_samples {
                    model
                        .override_scalar(input, Some(samples[future]), None)
                        .unwrap();
                }
                let calculated = ArrayCalculator::new(model).calculate_all().unwrap();
                let got = calculated.scalars["valuation.value_created"].value.unwrap();
                assert!(
                    (got - value.samples[future]).abs() <= 1e-9,
                    "future {future}: {got}"
                );
            }
        }

        #[test]
        fn the_earnout_is_paid_a_fifth_of_the_time_and_cuts_the_losses() {
            let plain = run(None);
            let earnout = run(Some("earnout"));
            let paid = &earnout.outputs["earnout.paid"];
            let share = paid.threshold_probabilities["> 0"];
            assert!((share - 0.2119).abs() <= 0.02, "{share}");
            assert!((paid.statistics.max - 240.0).abs() <= 0.0);
            let shortfall = |r: &crate::monte_carlo::SimulationResult| {
                r.outputs["valuation.value_created"].expected_shortfall[0].1
            };
            assert!(shortfall(&earnout) > shortfall(&plain) + 50.0);
            let positive = |r: &crate::monte_carlo::SimulationResult| {
                r.outputs["valuation.value_created"].threshold_probabilities["> 0"]
            };
            assert!(positive(&earnout) > positive(&plain));
        }
    }
}
