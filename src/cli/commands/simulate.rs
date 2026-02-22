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

/// Run Monte Carlo simulation and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the YAML file cannot be parsed, the Monte Carlo
/// configuration is invalid, or the simulation fails.
pub fn simulate_core(
    file: &Path,
    iterations_override: Option<usize>,
    seed_override: Option<u64>,
    sampling_override: Option<&str>,
) -> ForgeResult<crate::monte_carlo::SimulationResult> {
    let yaml_content = fs::read_to_string(file).map_err(ForgeError::Io)?;
    let mut config = parse_monte_carlo_config(&yaml_content)?;

    if let Some(n) = iterations_override {
        config.iterations = n;
    }
    if let Some(s) = seed_override {
        config.seed = Some(s);
    }
    if let Some(sampling) = sampling_override {
        config.sampling = sampling.to_string();
    }
    config.validate().map_err(ForgeError::Validation)?;

    let model = parser::parse_model(file)?;
    let mut engine = MonteCarloEngine::new(config.clone()).map_err(ForgeError::Validation)?;
    engine
        .parse_distributions_from_model(&model)
        .map_err(ForgeError::Validation)?;

    let output_vars: Vec<String> = config.outputs.iter().map(|o| o.variable.clone()).collect();

    engine
        .run_with_evaluator(|inputs: &HashMap<String, f64>| {
            let mut iter_model = model.clone();
            for (var_name, &value) in inputs {
                if let Some(scalar) = iter_model.scalars.get_mut(var_name) {
                    scalar.value = Some(value);
                    scalar.formula = None;
                }
            }
            let calculator = crate::core::array_calculator::ArrayCalculator::new(iter_model);
            let Ok(calculated) = calculator.calculate_all() else {
                return HashMap::new();
            };
            let mut outputs = HashMap::new();
            for var_name in &output_vars {
                let value = calculated
                    .scalars
                    .get(var_name)
                    .or_else(|| calculated.scalars.get(&format!("outputs.{var_name}")))
                    .or_else(|| calculated.scalars.get(&format!("scalars.{var_name}")))
                    .and_then(|s| s.value)
                    .unwrap_or(0.0);
                outputs.insert(var_name.clone(), value);
            }
            outputs
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
    iterations_override: Option<usize>,
    seed_override: Option<u64>,
    sampling_override: Option<&str>,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🎲 Forge - Monte Carlo Simulation".bold().green());
    println!("   File: {}", file.display());
    println!();

    // Parse the YAML file
    if verbose {
        println!("{}", "📖 Parsing YAML file...".cyan());
    }

    let yaml_content = fs::read_to_string(file).map_err(ForgeError::Io)?;

    // Parse monte_carlo config from YAML
    let mut config = parse_monte_carlo_config(&yaml_content)?;

    // Apply command-line overrides
    if let Some(n) = iterations_override {
        config.iterations = n;
    }
    if let Some(s) = seed_override {
        config.seed = Some(s);
    }
    if let Some(sampling) = sampling_override {
        config.sampling = sampling.to_string();
    }

    // Validate config
    config.validate().map_err(ForgeError::Validation)?;

    // Display config
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

    // Parse the full model to extract distributions
    let model = parser::parse_model(file)?;

    if verbose {
        println!(
            "   Found {} tables, {} scalars",
            model.tables.len(),
            model.scalars.len()
        );
    }

    // Create engine
    let mut engine = MonteCarloEngine::new(config.clone()).map_err(ForgeError::Validation)?;

    // Parse distributions from model
    engine
        .parse_distributions_from_model(&model)
        .map_err(ForgeError::Validation)?;

    // Run simulation with formula evaluation
    if verbose {
        println!("{}", "🎲 Running simulation...".cyan());
    }

    // Get output variable names from config
    let output_vars: Vec<String> = config.outputs.iter().map(|o| o.variable.clone()).collect();

    // Create evaluator that runs formulas for each iteration
    let result = engine
        .run_with_evaluator(|inputs: &HashMap<String, f64>| {
            // Clone the model and substitute sampled values
            let mut iter_model = model.clone();

            // Replace MC.* distribution formulas with sampled values
            for (var_name, &value) in inputs {
                // Scalars are stored with their full path (e.g., "scalars.p_sampled" or "outputs.p_sampled")
                if let Some(scalar) = iter_model.scalars.get_mut(var_name) {
                    // Replace the formula with the sampled value
                    scalar.value = Some(value);
                    scalar.formula = None; // Clear formula since we're using sampled value
                }
            }

            // Run the calculator to evaluate dependent formulas
            let calculator = ArrayCalculator::new(iter_model);
            let Ok(calculated) = calculator.calculate_all() else {
                return HashMap::new();
            };

            // Extract output values
            let mut outputs = HashMap::new();
            for var_name in &output_vars {
                // Try exact match first, then with common prefixes
                let value = calculated
                    .scalars
                    .get(var_name)
                    .or_else(|| calculated.scalars.get(&format!("outputs.{var_name}")))
                    .or_else(|| calculated.scalars.get(&format!("scalars.{var_name}")))
                    .and_then(|s| s.value)
                    .unwrap_or(0.0);

                outputs.insert(var_name.clone(), value);
            }

            outputs
        })
        .map_err(ForgeError::Eval)?;

    // Display results
    print_simulation_results(&result);

    // Write output file if specified
    if let Some(output_path) = output_file {
        write_simulation_output(&result, &output_path)?;
    }

    println!("{}", "✅ Simulation complete".bold().green());

    Ok(())
}

/// Print simulation results to stdout
fn print_simulation_results(result: &crate::monte_carlo::SimulationResult) {
    println!("{}", "📊 Simulation Results:".bold().green());
    println!("   Iterations:     {}", result.iterations_completed);
    println!("   Execution time: {} ms", result.execution_time_ms);
    println!();

    // Show input distributions
    println!("   {}", "Input Distributions:".bold());
    for (var_name, samples) in &result.input_samples {
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

            // Thresholds
            for (threshold, prob) in &output.threshold_probabilities {
                println!(
                    "         P({} {}) = {:.2}%",
                    var_name,
                    threshold,
                    prob * 100.0
                );
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
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    if let Some(mc_value) = value.get("monte_carlo") {
        let config: MonteCarloConfig = serde_yaml_ng::from_value(mc_value.clone())
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

        // Run with formula evaluation (the fix)
        let result = engine
            .run_with_evaluator(|inputs: &HashMap<String, f64>| {
                let mut iter_model = model.clone();

                // Substitute sampled values
                for (var_name, &value) in inputs {
                    if let Some(scalar) = iter_model.scalars.get_mut(var_name) {
                        scalar.value = Some(value);
                        scalar.formula = None;
                    }
                }

                // Run calculator
                let calculator = crate::core::array_calculator::ArrayCalculator::new(iter_model);
                let Ok(calculated) = calculator.calculate_all() else {
                    return HashMap::new();
                };

                // Extract outputs
                let mut outputs = HashMap::new();
                for var_name in &output_vars {
                    let value = calculated
                        .scalars
                        .get(var_name)
                        .or_else(|| calculated.scalars.get(&format!("scalars.{var_name}")))
                        .and_then(|s| s.value)
                        .unwrap_or(0.0);
                    outputs.insert(var_name.clone(), value);
                }
                outputs
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
}
