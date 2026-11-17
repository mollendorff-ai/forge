//! Monte Carlo Simulation CLI Command
//!
//! Runs probabilistic analysis using MC.* distribution functions.

use crate::cli::console::outln;
use crate::cli::Console;
use crate::core::array_calculator::ArrayCalculator;
use crate::error::{ForgeError, ForgeResult};
use crate::monte_carlo::{MonteCarloConfig, MonteCarloEngine};
use crate::parser;
use colored::Colorize;
use std::collections::HashMap;
use std::fmt::Write as _;
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
    output_vars: &[(String, Option<(String, usize)>)],
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
        .map(|(var_name, row)| {
            if let Some((column, row)) = row {
                let (table, col) = column.split_once('.').unwrap_or_default();
                return calculated
                    .tables
                    .get(table)
                    .and_then(|t| t.columns.get(col))
                    // An empty cell of a data column is 0, as a formula reads it
                    .and_then(|c| c.values.numbers().and_then(|v| v.get(*row).copied()))
                    .map(|value| (var_name.clone(), value))
                    .ok_or_else(|| format!("output '{var_name}' has no calculated number"));
            }
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

/// The outputs as simulated: a table column (`months.profit`) becomes one
/// output per row (`months.profit (row 1)`, ...), and each output learns
/// from the model whether its highest results are the worst (`better: lower`).
///
/// # Errors
///
/// Returns an error naming an output that is neither a scalar nor a column.
fn expand_outputs(
    outputs: &[crate::monte_carlo::config::OutputConfig],
    model: &crate::types::ParsedModel,
) -> ForgeResult<Vec<crate::monte_carlo::config::OutputConfig>> {
    let mut expanded = Vec::new();
    for output in outputs {
        let var = &output.variable;
        let column = var.split_once('.').and_then(|(table, col)| {
            let t = model.tables.get(table)?;
            let meta = t
                .columns
                .get(col)
                .map(|c| c.metadata.clone())
                .or_else(|| t.row_formulas.contains_key(col).then(Default::default))?;
            Some((t.row_count(), meta))
        });
        if let Some((rows, metadata)) = column {
            for row in 0..rows {
                let mut one = output.clone();
                one.variable = format!("{var} (row {})", row + 1);
                one.row = Some((var.clone(), row));
                one.higher_is_worse = metadata.better.as_deref() == Some("lower");
                expanded.push(one);
            }
            continue;
        }
        let scalar = model
            .scalars
            .get(var)
            .or_else(|| model.scalars.get(&format!("outputs.{var}")))
            .or_else(|| model.scalars.get(&format!("scalars.{var}")));
        let Some(scalar) = scalar else {
            return Err(ForgeError::Validation(format!(
                "monte_carlo output '{var}' is neither a scalar nor a table column of the model"
            )));
        };
        let mut one = output.clone();
        one.higher_is_worse = scalar.metadata.better.as_deref() == Some("lower");
        expanded.push(one);
    }
    Ok(expanded)
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
    /// Only these outputs of `monte_carlo.outputs` (all of them when empty).
    /// The draws do not depend on the outputs, so each one's numbers are the
    /// same as in a run of all of them.
    pub outputs: &'a [String],
}

/// The outputs of `outputs` that `names` choose, in the order configured: a
/// name is an output's variable, or its last part when only one output ends
/// with it (`profit` for `year.profit`). No names chooses all of them.
///
/// # Errors
///
/// Returns an error for a name that is not an output, or that ends more than
/// one, listing the outputs.
fn select_outputs(
    outputs: &[crate::monte_carlo::config::OutputConfig],
    names: &[String],
) -> ForgeResult<Vec<crate::monte_carlo::config::OutputConfig>> {
    if names.is_empty() {
        return Ok(outputs.to_vec());
    }
    let listed = || {
        outputs
            .iter()
            .map(|o| o.variable.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut chosen = vec![false; outputs.len()];
    for name in names {
        let index = if let Some(i) = outputs.iter().position(|o| &o.variable == name) {
            i
        } else {
            let suffix = format!(".{name}");
            let ends: Vec<usize> = (0..outputs.len())
                .filter(|&i| outputs[i].variable.ends_with(&suffix))
                .collect();
            match ends.as_slice() {
                [i] => *i,
                [] => {
                    return Err(ForgeError::Validation(format!(
                        "--output-var {name}: not an output of monte_carlo (outputs: {})",
                        listed()
                    )))
                },
                _ => {
                    let names: Vec<&str> =
                        ends.iter().map(|&i| outputs[i].variable.as_str()).collect();
                    return Err(ForgeError::Validation(format!(
                        "--output-var {name}: more than one output ends with it ({}); write \
                         the whole name",
                        names.join(", ")
                    )));
                },
            }
        };
        chosen[index] = true;
    }
    Ok(outputs
        .iter()
        .zip(chosen)
        .filter(|(_, keep)| *keep)
        .map(|(o, _)| o.clone())
        .collect())
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
        .run_with_evaluator(|inputs: &HashMap<String, f64>| {
            evaluate_iteration(model, plan, &output_vars, inputs)
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

/// Entries of a map in name order, so the printed results do not change between runs
fn sorted_by_name<V>(map: &std::collections::HashMap<String, V>) -> Vec<(&String, &V)> {
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    entries
}

/// Write the simulation results, as `forge simulate` prints them
fn print_simulation_results(c: &mut Console<'_>, result: &crate::monte_carlo::SimulationResult) {
    outln!(c, "{}", "📊 Simulation Results:".bold().green());
    outln!(c, "   Iterations:     {}", result.iterations_completed);
    outln!(c, "   Execution time: {} ms", result.execution_time_ms);
    outln!(c);

    print_input_distributions(c, result);

    // Show output results
    if !result.outputs.is_empty() {
        outln!(c, "   {}", "Output Statistics:".bold());
        let rows_of = |name: &str| {
            result
                .config
                .outputs
                .iter()
                .find(|o| o.variable == name)
                .and_then(|o| o.row.clone())
        };
        let mut printed_columns: Vec<String> = Vec::new();
        for (var_name, output) in &result.outputs {
            if let Some((column, _)) = rows_of(var_name) {
                if !printed_columns.contains(&column) {
                    print_column_rows(c, result, &column);
                    printed_columns.push(column);
                }
                continue;
            }
            print_output(c, var_name, output);
        }
    }
}

/// The sampled inputs: each one's mean, plan value, min and max
fn print_input_distributions(c: &mut Console<'_>, result: &crate::monte_carlo::SimulationResult) {
    outln!(c, "   {}", "Input Distributions:".bold());
    for (var_name, samples) in sorted_by_name(&result.input_samples) {
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        let min = samples.iter().copied().fold(f64::INFINITY, f64::min);
        let max = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let plan = result
            .plan_inputs
            .get(var_name)
            .map_or_else(String::new, |p| {
                format!(" plan={} ({})", super::display_number(p.value), p.from)
            });
        outln!(
            c,
            "      {} mean={}{plan} min={} max={}",
            var_name.bright_blue(),
            super::display_number(mean),
            super::display_number(min),
            super::display_number(max)
        );
    }
    outln!(c);
}

/// One output's statistics, thresholds, expected shortfall and drivers
fn print_output(c: &mut Console<'_>, var_name: &str, output: &crate::monte_carlo::OutputResult) {
    let stats = &output.statistics;
    outln!(c, "      {}:", var_name.bright_blue().bold());
    outln!(
        c,
        "         Mean:      {}",
        super::display_number(stats.mean)
    );
    if let Some(at_plan) = output.at_plan {
        outln!(
            c,
            "         At plan:   {}  (each uncertain input at its plan value)",
            super::display_number(at_plan)
        );
        if let Some(note) = mean_versus_plan(stats.mean, at_plan, stats.std_dev) {
            outln!(c, "         {note}");
        }
    }
    outln!(
        c,
        "         Median:    {}",
        super::display_number(stats.median)
    );
    outln!(
        c,
        "         Std Dev:   {}",
        super::display_number(stats.std_dev)
    );
    outln!(
        c,
        "         Min:       {}",
        super::display_number(stats.min)
    );
    outln!(
        c,
        "         Max:       {}",
        super::display_number(stats.max)
    );

    // Percentiles
    outln!(c, "         Percentiles:");
    for (p, v) in &stats.percentiles {
        outln!(c, "            P{p}: {}", super::display_number(*v));
    }

    // Thresholds, in the order configured
    for (threshold, prob) in &output.threshold_probabilities {
        outln!(
            c,
            "         P({} {}) = {:.2}%",
            var_name,
            threshold,
            prob * 100.0
        );
    }

    for (p, mean) in &output.expected_shortfall {
        outln!(
            c,
            "         Mean of the worst {p}% (expected shortfall): {}",
            super::display_number(*mean)
        );
    }

    // Which sampled inputs move it most
    if !output.drivers.is_empty() {
        outln!(c, "         Drivers (rank correlation with the output):");
        for driver in &output.drivers {
            outln!(
                c,
                "            {:+.3}  {}",
                driver.rank_correlation,
                driver.input
            );
        }
    }
    outln!(c);
}

/// A note when an output's mean is far from its value at plan (more than a
/// tenth of a standard deviation), which a reader would otherwise take for a
/// contradiction between `forge calculate` and `forge simulate`.
fn mean_versus_plan(mean: f64, at_plan: f64, std_dev: f64) -> Option<String> {
    let gap = mean - at_plan;
    // Not past a tenth of a standard deviation, or not comparable (NaN)
    if gap.abs().partial_cmp(&(0.1 * std_dev)) != Some(std::cmp::Ordering::Greater) {
        return None;
    }
    Some(format!(
        "The mean is {} {} the value at plan: the uncertain inputs' means are not their plan \
         values (see Input Distributions), or the model is not linear in them.",
        super::display_number(gap.abs()),
        if gap < 0.0 { "below" } else { "above" }
    ))
}

/// A table column output, one line per row: mean, P10, P50, P90, and each
/// threshold's probability.
fn print_column_rows(
    c: &mut Console<'_>,
    result: &crate::monte_carlo::SimulationResult,
    column: &str,
) {
    let rows: Vec<&crate::monte_carlo::OutputResult> = result
        .config
        .outputs
        .iter()
        .filter(|o| o.row.as_ref().is_some_and(|(c, _)| c == column))
        .filter_map(|o| result.outputs.get(&o.variable))
        .collect();
    let Some(first) = rows.first() else {
        return;
    };
    outln!(
        c,
        "      {} {}:",
        column.bright_blue().bold(),
        "by row".bold()
    );
    let mut header = format!(
        "         {:>4} {:>14} {:>14} {:>14} {:>14} {:>14}",
        "row", "mean", "at plan", "P10", "P50", "P90"
    );
    for threshold in first.threshold_probabilities.keys() {
        let _ = write!(header, " {:>10}", format!("P({threshold})"));
    }
    outln!(c, "{header}");
    for (row, output) in rows.iter().enumerate() {
        let p = |q: u8| {
            output
                .statistics
                .percentiles
                .get(&q)
                .copied()
                .unwrap_or(f64::NAN)
        };
        let mut line = format!(
            "         {:>4} {:>14} {:>14} {:>14} {:>14} {:>14}",
            row + 1,
            super::display_number(output.statistics.mean),
            output
                .at_plan
                .map_or_else(|| "-".to_string(), super::display_number),
            super::display_number(p(10)),
            super::display_number(p(50)),
            super::display_number(p(90))
        );
        for prob in output.threshold_probabilities.values() {
            let _ = write!(line, " {:>9.2}%", prob * 100.0);
        }
        outln!(c, "{line}");
    }
    outln!(c);
}

/// Write simulation output to a file (YAML, JSON, or Excel)
fn write_simulation_output(
    c: &mut Console<'_>,
    result: &crate::monte_carlo::SimulationResult,
    output_path: &Path,
) -> ForgeResult<()> {
    let ext = output_path.extension().and_then(|e| e.to_str());
    match ext {
        Some("xlsx") => {
            #[cfg(feature = "excel")]
            crate::monte_carlo::excel_export::export_results(result, output_path)
                .map_err(ForgeError::Validation)?;
            #[cfg(not(feature = "excel"))]
            return Err(ForgeError::Validation(
                "Writing Excel needs Forge's `excel` feature, which this build does not have"
                    .to_string(),
            ));
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

    outln!(
        c,
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
    use crate::cli::console::capture;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// T1-SIM-MEAN-VS-PLAN: the note appears past a tenth of a standard
    /// deviation, and says which way the mean lies.
    #[test]
    fn the_mean_versus_plan_note() {
        assert_eq!(mean_versus_plan(100.0, 100.5, 10.0), None);
        assert_eq!(mean_versus_plan(5.0, 5.0, 0.0), None);
        assert_eq!(mean_versus_plan(f64::NAN, 5.0, 1.0), None);
        let below = mean_versus_plan(-15_925.94, 786.32, 38_058.05).unwrap();
        assert!(
            below.starts_with("The mean is 16,712.26 below the value at plan"),
            "{below}"
        );
        let above = mean_versus_plan(12.0, 10.0, 1.0).unwrap();
        assert!(
            above.starts_with("The mean is 2.00 above the value at plan"),
            "{above}"
        );
    }

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
            let n = s.len() as f64;
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
                assert!(ran.success(), "{line:?}: {}", ran.both());
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
}
