//! Monte Carlo Simulation CLI Command
//!
//! Runs probabilistic analysis using MC.* distribution functions.

use crate::core::array_calculator::ArrayCalculator;
use crate::error::{ForgeError, ForgeResult};
use crate::monte_carlo::{MonteCarloConfig, MonteCarloEngine};
use crate::parser;
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Calculate the model once with the sampled `inputs` and read the outputs.
///
/// An input named `@ns.path` is a distribution in the included file `ns`.
///
/// # Errors
///
/// Returns the calculation error, or an error naming an output that has no
/// calculated value. Callers must not turn either into a 0 sample.
fn evaluate_iteration(
    model: &crate::types::ParsedModel,
    plan: &std::rc::Rc<crate::core::array_calculator::CalcPlan>,
    output_vars: &[String],
    inputs: &HashMap<String, f64>,
) -> Result<HashMap<String, f64>, String> {
    let mut iter_model = model.clone();
    // Replace MC.* distribution formulas with the sampled values
    for (var_name, &value) in inputs {
        iter_model.override_scalar(var_name, Some(value), None)?;
    }

    // Every iteration has the same formulas, so they are parsed and ordered once
    let calculated = ArrayCalculator::with_plan(iter_model, std::rc::Rc::clone(plan))
        .calculate_all()
        .map_err(|e| e.to_string())?;

    output_vars
        .iter()
        .map(|var_name| {
            calculated
                .scalars
                .get(var_name)
                .or_else(|| calculated.scalars.get(&format!("outputs.{var_name}")))
                .or_else(|| calculated.scalars.get(&format!("scalars.{var_name}")))
                .and_then(|s| s.value)
                .map(|value| (var_name.clone(), value))
                .ok_or_else(|| format!("output '{var_name}' has no calculated value"))
        })
        .collect()
}

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
    let yaml_content = fs::read_to_string(file).map_err(ForgeError::Io)?;
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

    let mut model = parser::parse_model(file)?;
    if let Some(scenario) = options.scenario {
        super::apply_scenario(&mut model, scenario)?;
    }
    let mut engine = MonteCarloEngine::new(config.clone()).map_err(ForgeError::Validation)?;
    engine
        .parse_distributions_from_model(&model)
        .map_err(ForgeError::Validation)?;

    let output_vars: Vec<String> = config.outputs.iter().map(|o| o.variable.clone()).collect();
    let plan = crate::core::array_calculator::CalcPlan::new();

    engine
        .run_with_evaluator(|inputs: &HashMap<String, f64>| {
            evaluate_iteration(&model, &plan, &output_vars, inputs)
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
    file: &Path,
    options: SimulateOptions<'_>,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🎲 Forge - Monte Carlo Simulation".bold().green());
    println!("   File: {}", file.display());
    if let Some(scenario) = options.scenario {
        println!("   Scenario: {}", scenario.bright_blue());
    }
    println!();
    if verbose {
        println!("{}", "🎲 Running simulation...".cyan());
    }

    let result = simulate_core(file, options)?;
    let config = &result.config;
    println!("   {}", "Configuration:".bold());
    println!(
        "      Iterations: {}",
        config.iterations.to_string().bright_blue()
    );
    println!("      Sampling:   {}", config.sampling.bright_blue());
    if let Some(seed) = config.seed {
        println!("      Seed:       {}", seed.to_string().bright_blue());
    }
    println!();

    print_simulation_results(&result);

    if let Some(output_path) = output_file {
        write_simulation_output(&result, &output_path)?;
    }

    println!("{}", "✅ Simulation complete".bold().green());

    Ok(())
}

/// Entries of a map in name order, so the printed results do not change between runs
fn sorted_by_name<V>(map: &std::collections::HashMap<String, V>) -> Vec<(&String, &V)> {
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    entries
}

/// Print simulation results to stdout
fn print_simulation_results(result: &crate::monte_carlo::SimulationResult) {
    println!("{}", "📊 Simulation Results:".bold().green());
    println!("   Iterations:     {}", result.iterations_completed);
    println!("   Execution time: {} ms", result.execution_time_ms);
    println!();

    // Show input distributions
    println!("   {}", "Input Distributions:".bold());
    for (var_name, samples) in sorted_by_name(&result.input_samples) {
        #[allow(clippy::cast_precision_loss)] // sample counts are always small enough for f64
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        let min = samples.iter().copied().fold(f64::INFINITY, f64::min);
        let max = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        println!(
            "      {} mean={:.2} min={:.2} max={:.2}",
            var_name.bright_blue(),
            mean,
            min,
            max
        );
    }
    println!();

    // Show output results
    if !result.outputs.is_empty() {
        println!("   {}", "Output Statistics:".bold());
        for (var_name, output) in &result.outputs {
            let stats = &output.statistics;
            println!("      {}:", var_name.bright_blue().bold());
            println!("         Mean:      {:.4}", stats.mean);
            println!("         Median:    {:.4}", stats.median);
            println!("         Std Dev:   {:.4}", stats.std_dev);
            println!("         Min:       {:.4}", stats.min);
            println!("         Max:       {:.4}", stats.max);

            // Percentiles
            println!("         Percentiles:");
            for (p, v) in &stats.percentiles {
                println!("            P{p}: {v:.4}");
            }

            // Thresholds, in the order configured
            for (threshold, prob) in &output.threshold_probabilities {
                println!(
                    "         P({} {}) = {:.2}%",
                    var_name,
                    threshold,
                    prob * 100.0
                );
            }

            // Which sampled inputs move it most
            if !output.drivers.is_empty() {
                println!("         Drivers (rank correlation with the output):");
                for driver in &output.drivers {
                    println!(
                        "            {:+.3}  {}",
                        driver.rank_correlation, driver.input
                    );
                }
            }
            println!();
        }
    }
}

/// Write simulation output to a file (YAML, JSON, or Excel)
fn write_simulation_output(
    result: &crate::monte_carlo::SimulationResult,
    output_path: &Path,
) -> ForgeResult<()> {
    let ext = output_path.extension().and_then(|e| e.to_str());
    match ext {
        Some("xlsx") => {
            crate::monte_carlo::excel_export::export_results(result, output_path)
                .map_err(ForgeError::Validation)?;
        },
        Some("json") => {
            let output_str = result
                .to_json()
                .map_err(|e| ForgeError::Validation(format!("JSON error: {e}")))?;
            fs::write(output_path, output_str).map_err(ForgeError::Io)?;
        },
        _ => {
            let output_str = result.to_yaml();
            fs::write(output_path, output_str).map_err(ForgeError::Io)?;
        },
    }

    println!(
        "{}",
        format!("💾 Results written to {}", output_path.display())
            .bold()
            .green()
    );
    Ok(())
}

/// Parse `monte_carlo` config from YAML content
fn parse_monte_carlo_config(yaml_content: &str) -> ForgeResult<MonteCarloConfig> {
    // Try to parse the monte_carlo section from the YAML
    let value: yaml_serde::Value = yaml_serde::from_str(yaml_content)
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
        let output_vars: Vec<String> = config.outputs.iter().map(|o| o.variable.clone()).collect();
        let plan = crate::core::array_calculator::CalcPlan::new();

        // Run with formula evaluation (the fix)
        let result = engine
            .run_with_evaluator(|inputs: &HashMap<String, f64>| {
                evaluate_iteration(&model, &plan, &output_vars, inputs)
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
}
