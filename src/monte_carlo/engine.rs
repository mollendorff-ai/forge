//! Monte Carlo Simulation Engine
//!
//! Orchestrates the simulation:
//! 1. Parse distributions from model
//! 2. Generate samples using specified method
//! 3. Evaluate formulas for each iteration
//! 4. Compute output statistics

use crate::core::precision;
use indexmap::IndexMap;
use num_traits::ToPrimitive;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;

/// When a run starts, to report `execution_time_ms`. A browser build
/// (wasm32-unknown-unknown) has no clock behind `Instant`, which panics
/// there; it reports 0 ms (ADR-055).
fn stopwatch() -> Option<Instant> {
    (!cfg!(all(target_arch = "wasm32", target_os = "unknown"))).then(Instant::now)
}

/// Milliseconds since [`stopwatch`], or 0 without a clock (and `u64::MAX`
/// after 584 million years).
fn elapsed_ms(start: Option<Instant>) -> u64 {
    start.map_or(0, |start| {
        u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
    })
}

use super::config::MonteCarloConfig;
use super::correlation::CorrelationMatrix;
use super::distributions::{
    parse_distribution, standard_normal_cdf, standard_normal_quantile, Distribution,
};
use super::sampler::{Sampler, SamplingMethod};
use super::statistics::{
    calculate_percentiles, evaluate_threshold, parse_threshold, Histogram, Statistics,
};
use crate::types::{row_input_name, ParsedModel};

/// The value an uncertain input is planned at, which the model is calculated
/// with once to give each output's value at plan.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanInput {
    /// The planned value
    pub value: f64,
    /// Where it comes from: `value` (written beside the `MC.*` formula), or
    /// the distribution's `mode`, `mean`, `midpoint` or `likeliest` value
    pub from: &'static str,
}

/// The plan figure of each uncertain input of `model`, in name order.
///
/// The model's own and its included files' (`@namespace.name`): the figures
/// `forge simulate` calculates each output's value at plan with
/// ([`MonteCarloEngine::plan_inputs`]).
///
/// # Errors
///
/// Returns an error if an `MC.*` formula does not parse.
///
/// # Panics
///
/// Never: the default settings are valid.
pub fn plan_of(model: &ParsedModel) -> Result<IndexMap<String, PlanInput>, String> {
    let mut engine = MonteCarloEngine::new(MonteCarloConfig::default())
        .expect("the default Monte Carlo settings are valid");
    engine.parse_distributions_from_model(model)?;
    Ok(engine.plan_inputs())
}

/// Result of a Monte Carlo simulation
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Configuration used
    pub config: MonteCarloConfig,
    /// Number of iterations completed
    pub iterations_completed: usize,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Results for each tracked output variable, in the order configured
    pub outputs: IndexMap<String, OutputResult>,
    /// All sampled values for inputs (variable -> samples)
    pub input_samples: HashMap<String, Vec<f64>>,
    /// Each uncertain input's plan value, in name order
    pub plan_inputs: IndexMap<String, PlanInput>,
}

/// Result for a single output variable
#[derive(Debug, Clone)]
pub struct OutputResult {
    /// Variable name
    pub variable: String,
    /// Statistics for this output
    pub statistics: Statistics,
    /// All simulated values
    pub samples: Vec<f64>,
    /// Histogram data
    pub histogram: Histogram,
    /// Probability thresholds (threshold string -> probability), in the order configured
    pub threshold_probabilities: IndexMap<String, f64>,
    /// Inputs ranked by how strongly they move this output
    pub drivers: Vec<Driver>,
    /// Mean of the worst p% of runs, for each p asked for, in that order
    pub expected_shortfall: Vec<(u8, f64)>,
    /// The output calculated once with every uncertain input at its plan
    /// value; `None` when that calculation fails
    pub at_plan: Option<f64>,
}

/// How strongly one sampled input moves an output.
#[derive(Debug, Clone, PartialEq)]
pub struct Driver {
    /// The `MC.*` input
    pub input: String,
    /// Spearman rank correlation between the input's samples and the output's
    /// (-1 to 1; the sign says which way the output moves)
    pub rank_correlation: f64,
}

/// Statistics, thresholds and drivers of one output's samples.
fn summarize(
    output_config: &super::config::OutputConfig,
    samples: Vec<f64>,
    input_samples: &HashMap<String, Vec<f64>>,
) -> OutputResult {
    let mut statistics = Statistics::from_samples(&samples);
    // The percentiles asked for, on top of the standard set
    statistics
        .percentiles
        .extend(calculate_percentiles(&samples, &output_config.percentiles));
    let histogram = Histogram::from_samples(&samples, 50);

    // Thresholds were checked by `MonteCarloConfig::validate`
    let threshold_probabilities = output_config
        .all_thresholds()
        .into_iter()
        .map(|t| {
            let (op, value) = parse_threshold(t).expect("validate parsed every threshold");
            (t.to_string(), evaluate_threshold(&samples, &op, value))
        })
        .collect();

    let drivers = drivers(&samples, input_samples);
    let expected_shortfall = output_config
        .expected_shortfall
        .iter()
        .map(|&p| {
            (
                p,
                expected_shortfall(&samples, p, output_config.higher_is_worse),
            )
        })
        .collect();
    OutputResult {
        variable: output_config.variable.clone(),
        statistics,
        samples,
        histogram,
        threshold_probabilities,
        drivers,
        expected_shortfall,
        at_plan: None,
    }
}

/// The mean of the worst `percent`% of `samples`: the lowest, or the highest
/// when `higher_is_worse`. The worst ceil(n × percent / 100) samples are
/// averaged, so there is always at least one. `samples` is never empty:
/// `validate` asks for at least one iteration.
fn expected_shortfall(samples: &[f64], percent: u8, higher_is_worse: bool) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    if higher_is_worse {
        sorted.reverse();
    }
    let count = (precision::count(sorted.len()) * f64::from(percent) / 100.0)
        .ceil()
        .to_usize()
        .unwrap_or(1);
    let worst = &sorted[..count.clamp(1, sorted.len())];
    super::statistics::mean(worst)
}

/// Every input whose samples vary, ranked by the size of its rank
/// correlation with the output (name order breaks ties). An input that does
/// not vary, or an output that does not, moves nothing and is left out.
fn drivers(output: &[f64], input_samples: &HashMap<String, Vec<f64>>) -> Vec<Driver> {
    let varies = |v: &[f64]| v.iter().any(|x| (x - v[0]).abs() > 0.0);
    if output.is_empty() || !varies(output) {
        return Vec::new();
    }
    let mut drivers: Vec<Driver> = input_samples
        .iter()
        .filter(|(_, samples)| samples.len() == output.len() && varies(samples))
        .map(|(input, samples)| Driver {
            input: input.clone(),
            rank_correlation: super::sensitivity::spearman_correlation(samples, output),
        })
        .collect();
    drivers.sort_by(|a, b| {
        b.rank_correlation
            .abs()
            .total_cmp(&a.rank_correlation.abs())
            .then_with(|| a.input.cmp(&b.input))
    });
    drivers
}

/// Monte Carlo simulation engine
pub struct MonteCarloEngine {
    config: MonteCarloConfig,
    sampler: Sampler,
    distributions: HashMap<String, Distribution>,
    /// The `value:` written beside an `MC.*` formula, by input
    planned: HashMap<String, f64>,
}

/// What calculates the outputs of one iteration from its inputs.
pub type Evaluator<'a> =
    dyn FnMut(&HashMap<String, f64>) -> Result<HashMap<String, f64>, String> + 'a;

impl MonteCarloEngine {
    /// Create a new engine with the given configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid (see [`MonteCarloConfig::validate`]).
    ///
    /// # Panics
    ///
    /// Cannot panic: `validate` accepts only sampling methods `from_str` reads.
    pub fn new(config: MonteCarloConfig) -> Result<Self, String> {
        config.validate()?;

        let method = SamplingMethod::from_str(&config.sampling)
            .expect("validate accepts only monte_carlo and latin_hypercube");
        let sampler = Sampler::new(method, config.seed);

        Ok(Self {
            config,
            sampler,
            distributions: HashMap::new(),
            planned: HashMap::new(),
        })
    }

    /// Add a distribution for a variable
    pub fn add_distribution(&mut self, variable: &str, distribution: Distribution) {
        self.distributions
            .insert(variable.to_string(), distribution);
    }

    /// Parse distributions from a model's scalar formulas, and from those of
    /// the files it includes (named `@namespace.path`).
    ///
    /// # Errors
    ///
    /// Returns an error if any `MC.*` formula fails to parse.
    pub fn parse_distributions_from_model(&mut self, model: &ParsedModel) -> Result<(), String> {
        self.add_distributions_of(model, "")?;
        for (namespace, resolved) in &model.resolved_includes {
            self.add_distributions_of(&resolved.model, &format!("@{namespace}."))
                .map_err(|e| format!("{e} (in the file included as '{namespace}')"))?;
        }
        Ok(())
    }

    fn add_distributions_of(&mut self, model: &ParsedModel, prefix: &str) -> Result<(), String> {
        for (name, scalar) in &model.scalars {
            if let Some(formula) = &scalar.formula {
                let formula = formula.trim();
                // Check if formula starts with =MC. or MC.
                let formula_content = formula.strip_prefix('=').unwrap_or(formula);

                if formula_content.starts_with("MC.") {
                    let dist = parse_distribution(formula_content)
                        .map_err(|e| format!("{prefix}{name}: {e}"))?;
                    let variable = format!("{prefix}{name}");
                    if let Some(value) = scalar.value.filter(|v| v.is_finite()) {
                        self.planned.insert(variable.clone(), value);
                    }
                    self.add_distribution(&variable, dist);
                }
            }
        }
        // A table column whose formula is a distribution draws each row on
        // its own: one input per row, named as a row of an output is
        for (table_name, table) in &model.tables {
            let rows = table.row_count();
            for (column, formula) in &table.row_formulas {
                let formula = formula.trim();
                let content = formula.strip_prefix('=').unwrap_or(formula);
                if !content.starts_with("MC.") {
                    continue;
                }
                let table = format!("{prefix}{table_name}");
                let dist =
                    parse_distribution(content).map_err(|e| format!("{table}.{column}: {e}"))?;
                for row in 0..rows {
                    self.add_distribution(&row_input_name(&table, column, row), dist.clone());
                }
            }
        }
        Ok(())
    }

    /// Each uncertain input's plan value, in name order: the `value:` beside
    /// its formula, or else its distribution's likeliest value.
    #[must_use]
    pub fn plan_inputs(&self) -> IndexMap<String, PlanInput> {
        let mut names: Vec<&String> = self.distributions.keys().collect();
        names.sort();
        names
            .into_iter()
            .map(|name| {
                let plan = self.planned.get(name).map_or_else(
                    || {
                        let (value, from) = self.distributions[name].most_likely();
                        PlanInput { value, from }
                    },
                    |&value| PlanInput {
                        value,
                        from: "value",
                    },
                );
                (name.clone(), plan)
            })
            .collect()
    }

    /// Run the simulation
    ///
    /// # Errors
    ///
    /// Returns an error if output variable samples cannot be resolved.
    pub fn run(&mut self) -> Result<SimulationResult, String> {
        let start = stopwatch();
        let n = self.config.iterations;

        let input_samples = self.sample_inputs(n)?;
        let plan_inputs = self.plan_inputs();

        let mut outputs = IndexMap::new();
        for output_config in &self.config.outputs {
            let var = &output_config.variable;
            // Without an evaluator an output can only be an input itself
            let samples = input_samples
                .get(var)
                .or_else(|| input_samples.get(&format!("scalars.{var}")))
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "output '{var}' is not an MC.* input; use run_with_evaluator to \
                         calculate it"
                    )
                })?;
            let mut result = summarize(output_config, samples, &input_samples);
            result.at_plan = plan_inputs
                .get(var)
                .or_else(|| plan_inputs.get(&format!("scalars.{var}")))
                .map(|p| p.value);
            outputs.insert(var.clone(), result);
        }

        let execution_time_ms = elapsed_ms(start);

        Ok(SimulationResult {
            config: self.config.clone(),
            iterations_completed: n,
            execution_time_ms,
            outputs,
            input_samples,
            plan_inputs,
        })
    }

    /// Run simulation with a custom evaluator function
    ///
    /// The evaluator takes input values for one iteration and returns output values.
    ///
    /// # Errors
    ///
    /// Returns an error if sampling fails, if the evaluator fails for any
    /// iteration, or if it returns no value for a configured output. A failed
    /// iteration is never recorded as 0.
    ///
    /// # Panics
    ///
    /// Panics if an output variable configured in config is missing from `output_samples`.
    pub fn run_with_evaluator(
        &mut self,
        evaluator: &mut Evaluator<'_>,
    ) -> Result<SimulationResult, String> {
        let start = stopwatch();
        let n = self.config.iterations;

        let input_samples = self.sample_inputs(n)?;

        // Initialize output sample storage
        let output_vars: Vec<String> = self
            .config
            .outputs
            .iter()
            .map(|o| o.variable.clone())
            .collect();
        let mut output_samples: HashMap<String, Vec<f64>> = output_vars
            .iter()
            .map(|v| (v.clone(), Vec::with_capacity(n)))
            .collect();

        // Run iterations
        for i in 0..n {
            // Collect input values for this iteration
            let mut inputs: HashMap<String, f64> = HashMap::new();
            for (var, samples) in &input_samples {
                inputs.insert(var.clone(), samples[i]);
            }

            // Evaluate
            let outputs = evaluator(&inputs).map_err(|e| format!("Iteration {}: {e}", i + 1))?;

            // Store output values
            for var in &output_vars {
                let value = outputs
                    .get(var)
                    .copied()
                    .ok_or_else(|| format!("Iteration {}: no value for output '{var}'", i + 1))?;
                output_samples.get_mut(var).unwrap().push(value);
            }
        }

        // The model once more, with every uncertain input at its plan value.
        // The draws are already made, so this changes no other number; a
        // model that cannot be calculated there has no value at plan.
        let plan_inputs = self.plan_inputs();
        let planned: HashMap<String, f64> = plan_inputs
            .iter()
            .map(|(name, plan)| (name.clone(), plan.value))
            .collect();
        let at_plan = evaluator(&planned).ok();

        let mut outputs = IndexMap::new();
        for output_config in &self.config.outputs {
            let var = &output_config.variable;
            let samples = output_samples.remove(var).unwrap_or_default();
            let mut result = summarize(output_config, samples, &input_samples);
            result.at_plan = at_plan.as_ref().and_then(|m| m.get(var).copied());
            outputs.insert(var.clone(), result);
        }

        let execution_time_ms = elapsed_ms(start);

        Ok(SimulationResult {
            config: self.config.clone(),
            iterations_completed: n,
            execution_time_ms,
            outputs,
            input_samples,
            plan_inputs,
        })
    }

    /// Draw `n` samples for every distribution.
    ///
    /// Variables are handled in name order, so the same seed gives the same
    /// samples on every run. Uniform draws come from the configured sampling
    /// method (plain random or Latin hypercube) and are mapped through each
    /// distribution's inverse CDF. Configured correlations are applied to the
    /// normal scores of those draws first (a Gaussian copula), which keeps
    /// every variable's own distribution.
    fn sample_inputs(&mut self, n: usize) -> Result<HashMap<String, Vec<f64>>, String> {
        let mut names: Vec<String> = self.distributions.keys().cloned().collect();
        names.sort();

        let mut uniforms = self.sampler.generate_uniform_samples_nd(n, names.len());
        if !self.config.correlations.is_empty() {
            self.apply_correlations(&names, &mut uniforms)?;
        }

        Ok(names
            .into_iter()
            .zip(uniforms)
            .map(|(name, column)| {
                let dist = &self.distributions[&name];
                let samples = column.into_iter().map(|p| dist.inverse_cdf(p)).collect();
                (name, samples)
            })
            .collect())
    }

    /// Correlate the uniform draws of the variables named in `correlations`.
    fn apply_correlations(
        &self,
        names: &[String],
        uniforms: &mut [Vec<f64>],
    ) -> Result<(), String> {
        let pairs = self
            .config
            .correlations
            .iter()
            .map(|corr| {
                let first = &names[resolve_variable(names, &corr.variables[0])?];
                let second = &names[resolve_variable(names, &corr.variables[1])?];
                if first == second {
                    return Err(format!("correlation: '{first}' is paired with itself"));
                }
                Ok((first.clone(), second.clone(), corr.coefficient))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let matrix = CorrelationMatrix::new(&pairs)?;

        // One cursor down the column of each of the matrix's variables, in
        // the matrix's order; each step takes one draw from every column
        let mut by_name: HashMap<&str, std::slice::IterMut<'_, f64>> = names
            .iter()
            .map(String::as_str)
            .zip(uniforms.iter_mut().map(|column| column.iter_mut()))
            .collect();
        let mut cursors: Vec<std::slice::IterMut<'_, f64>> = matrix
            .variables
            .iter()
            .map(|var| {
                by_name
                    .remove(var.as_str())
                    .expect("the matrix's variables are names of distributions, each once")
            })
            .collect();

        let mut scores = vec![0.0; cursors.len()];
        // The columns are as long as each other, so all end at the same row
        while let Some(draws) = cursors
            .iter_mut()
            .map(Iterator::next)
            .collect::<Option<Vec<&mut f64>>>()
        {
            for (score, draw) in scores.iter_mut().zip(&draws) {
                *score = standard_normal_quantile(**draw);
            }
            let correlated = matrix
                .correlate(&scores)
                .expect("one score for each variable of the matrix, which has its factor");
            for (draw, value) in draws.into_iter().zip(correlated) {
                *draw = standard_normal_cdf(value);
            }
        }
        Ok(())
    }
}

/// Index of the distribution a correlation refers to: an exact name, or the
/// only name whose last part matches (`revenue_growth` for
/// `assumptions.revenue_growth`).
fn resolve_variable(names: &[String], var: &str) -> Result<usize, String> {
    if let Some(index) = names.iter().position(|name| name == var) {
        return Ok(index);
    }
    let matches: Vec<usize> = names
        .iter()
        .enumerate()
        .filter(|(_, name)| name.rsplit_once('.').is_some_and(|(_, last)| last == var))
        .map(|(index, _)| index)
        .collect();
    match matches.as_slice() {
        [index] => Ok(*index),
        [] => Err(format!(
            "correlation variable '{var}' has no MC.* distribution. Distributions: {}",
            names.join(", ")
        )),
        _ => Err(format!(
            "correlation variable '{var}' is ambiguous: {}. Use the full name.",
            matches
                .iter()
                .map(|&index| names[index].as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

impl SimulationResult {
    /// Format results as YAML string
    #[must_use]
    pub fn to_yaml(&self) -> String {
        use std::fmt::Write;

        let mut output = String::new();

        output.push_str("monte_carlo_results:\n");
        let _ = writeln!(output, "  iterations: {}", self.iterations_completed);
        let _ = writeln!(output, "  execution_time_ms: {}", self.execution_time_ms);
        let _ = writeln!(output, "  sampling: {}", self.config.sampling);
        if let Some(seed) = self.config.seed {
            let _ = writeln!(output, "  seed: {seed}");
        }

        output.push_str("\n  outputs:\n");
        for (var, result) in &self.outputs {
            let _ = writeln!(output, "    {var}:");
            let _ = writeln!(output, "      mean: {:.4}", result.statistics.mean);
            if let Some(at_plan) = result.at_plan {
                let _ = writeln!(output, "      at_plan: {at_plan:.4}");
            }
            let _ = writeln!(output, "      median: {:.4}", result.statistics.median);
            let _ = writeln!(output, "      std_dev: {:.4}", result.statistics.std_dev);
            let _ = writeln!(output, "      min: {:.4}", result.statistics.min);
            let _ = writeln!(output, "      max: {:.4}", result.statistics.max);

            output.push_str("      percentiles:\n");
            for (p, v) in &result.statistics.percentiles {
                let _ = writeln!(output, "        p{p}: {v:.4}");
            }

            if !result.threshold_probabilities.is_empty() {
                output.push_str("      thresholds:\n");
                for (t, prob) in &result.threshold_probabilities {
                    let _ = writeln!(output, "        \"{t}\": {prob:.4}");
                }
            }
            if !result.expected_shortfall.is_empty() {
                output.push_str("      expected_shortfall:\n");
                for (p, v) in &result.expected_shortfall {
                    let _ = writeln!(output, "        worst_{p}_percent: {v:.4}");
                }
            }
            if !result.drivers.is_empty() {
                output.push_str("      drivers:\n");
                for d in &result.drivers {
                    let _ = writeln!(output, "        \"{}\": {:.4}", d.input, d.rank_correlation);
                }
            }
        }

        output
    }

    /// Format results as JSON string
    ///
    /// # Panics
    ///
    /// Never in practice: serializing a `serde_json::Value` cannot fail.
    #[must_use]
    pub fn to_json(&self) -> String {
        use serde_json::{json, to_string_pretty};

        let mut outputs_json = serde_json::Map::new();
        for (var, result) in &self.outputs {
            let percentiles: serde_json::Map<String, serde_json::Value> = result
                .statistics
                .percentiles
                .iter()
                .map(|(p, v)| (format!("p{p}"), json!(v)))
                .collect();

            let thresholds: serde_json::Map<String, serde_json::Value> = result
                .threshold_probabilities
                .iter()
                .map(|(t, p)| (t.clone(), json!(p)))
                .collect();
            let drivers: Vec<serde_json::Value> = result
                .drivers
                .iter()
                .map(|d| json!({"input": d.input, "rank_correlation": d.rank_correlation}))
                .collect();

            outputs_json.insert(
                var.clone(),
                json!({
                    "mean": result.statistics.mean,
                    "at_plan": result.at_plan,
                    "median": result.statistics.median,
                    "std_dev": result.statistics.std_dev,
                    "min": result.statistics.min,
                    "max": result.statistics.max,
                    "percentiles": percentiles,
                    "thresholds": thresholds,
                    "drivers": drivers,
                    "expected_shortfall": result
                        .expected_shortfall
                        .iter()
                        .map(|(p, v)| json!({"percent": p, "mean": v}))
                        .collect::<Vec<_>>(),
                }),
            );
        }

        let inputs_json: serde_json::Map<String, serde_json::Value> = self
            .plan_inputs
            .iter()
            .map(|(name, plan)| {
                let mean = self
                    .input_samples
                    .get(name)
                    .map(|s| super::statistics::mean(s));
                (
                    name.clone(),
                    json!({"mean": mean, "plan": plan.value, "plan_from": plan.from}),
                )
            })
            .collect();

        let result_json = json!({
            "monte_carlo_results": {
                "iterations": self.iterations_completed,
                "execution_time_ms": self.execution_time_ms,
                "sampling": self.config.sampling,
                "seed": self.config.seed,
                "inputs": inputs_json,
                "outputs": outputs_json,
                // `outputs` is keyed by name, sorted as text; this is the
                // order of `monte_carlo.outputs`, a column's rows in order
                "output_order": self.outputs.keys().collect::<Vec<_>>(),
            }
        });

        to_string_pretty(&crate::core::precision::Rounded(&result_json))
            .expect("a JSON value always serializes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monte_carlo::config::OutputConfig;

    fn test_config() -> MonteCarloConfig {
        MonteCarloConfig {
            enabled: true,
            iterations: 10000,
            sampling: "latin_hypercube".to_string(),
            seed: Some(12345),
            outputs: vec![OutputConfig {
                variable: "revenue".to_string(),
                percentiles: vec![10, 50, 90],
                threshold: Some("> 100000".to_string()),
                thresholds: vec![],
                expected_shortfall: vec![],
                higher_is_worse: false,
                row: None,
                label: None,
            }],
            correlations: vec![],
        }
    }

    #[test]
    fn test_engine_creation() {
        let config = test_config();
        let engine = MonteCarloEngine::new(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_add_distribution() {
        let config = test_config();
        let mut engine = MonteCarloEngine::new(config).unwrap();

        let dist = Distribution::normal(100_000.0, 15_000.0).unwrap();
        engine.add_distribution("revenue", dist);

        assert!(engine.distributions.contains_key("revenue"));
    }

    #[test]
    fn test_run_simulation() {
        let config = test_config();
        let mut engine = MonteCarloEngine::new(config).unwrap();

        let dist = Distribution::normal(100_000.0, 15_000.0).unwrap();
        engine.add_distribution("revenue", dist);

        let result = engine.run().unwrap();

        assert_eq!(result.iterations_completed, 10000);
        assert!(result.input_samples.contains_key("revenue"));
        assert!(result.outputs.contains_key("revenue"));

        // Check statistics are reasonable
        let revenue_result = &result.outputs["revenue"];
        assert!((revenue_result.statistics.mean - 100_000.0).abs() < 2_000.0);
        assert!(revenue_result.statistics.percentiles.contains_key(&50));
    }

    #[test]
    fn test_run_with_evaluator() {
        let config = MonteCarloConfig {
            enabled: true,
            iterations: 1000,
            sampling: "latin_hypercube".to_string(),
            seed: Some(42),
            outputs: vec![OutputConfig {
                variable: "profit".to_string(),
                percentiles: vec![10, 50, 90],
                threshold: Some("> 0".to_string()),
                thresholds: vec![],
                expected_shortfall: vec![],
                higher_is_worse: false,
                row: None,
                label: None,
            }],
            correlations: vec![],
        };

        let mut engine = MonteCarloEngine::new(config).unwrap();

        engine.add_distribution("revenue", Distribution::normal(100.0, 10.0).unwrap());
        engine.add_distribution("costs", Distribution::normal(80.0, 5.0).unwrap());

        let result = engine
            .run_with_evaluator(&mut |inputs| {
                let revenue = inputs.get("revenue").copied().unwrap_or(0.0);
                let costs = inputs.get("costs").copied().unwrap_or(0.0);
                let mut outputs = HashMap::new();
                outputs.insert("profit".to_string(), revenue - costs);
                Ok(outputs)
            })
            .unwrap();

        let profit_result = &result.outputs["profit"];
        // Expected profit mean ≈ 100 - 80 = 20
        assert!((profit_result.statistics.mean - 20.0).abs() < 3.0);

        // Check threshold probability (profit > 0 should be high)
        let prob = profit_result.threshold_probabilities.get("> 0").unwrap();
        assert!(*prob > 0.9);
    }

    /// T1-SIM-MEAN-VS-PLAN: with no `value:` beside a formula, an input is
    /// planned at its distribution's likeliest value, and an output without an
    /// evaluator (an input itself) is at plan where its input is.
    #[test]
    fn plan_values_fall_back_to_the_likeliest_value() {
        let mut engine = MonteCarloEngine::new(test_config()).unwrap();
        engine.add_distribution("revenue", Distribution::normal(100.0, 10.0).unwrap());
        engine.add_distribution("hours", Distribution::triangular(1.0, 2.0, 6.0).unwrap());
        let plan = engine.plan_inputs();
        assert_eq!(
            plan.keys().collect::<Vec<_>>(),
            ["hours", "revenue"],
            "in name order"
        );
        assert_eq!(
            plan["hours"],
            PlanInput {
                value: 2.0,
                from: "mode"
            }
        );
        assert_eq!(
            plan["revenue"],
            PlanInput {
                value: 100.0,
                from: "mean"
            }
        );

        let result = engine.run().unwrap();
        assert_eq!(result.outputs["revenue"].at_plan, Some(100.0));
        let yaml = result.to_yaml();
        assert!(yaml.contains("      at_plan: 100.0000\n"), "{yaml}");
        let json: serde_json::Value = serde_json::from_str(&result.to_json()).unwrap();
        let mc = &json["monte_carlo_results"];
        assert_eq!(mc["outputs"]["revenue"]["at_plan"], 100.0);
        assert_eq!(mc["inputs"]["hours"]["plan"], 2.0);
        assert_eq!(mc["inputs"]["hours"]["plan_from"], "mode");
        let mean = mc["inputs"]["hours"]["mean"].as_f64().unwrap();
        assert!((mean - 3.0).abs() < 0.01, "{mean}");
    }

    /// The plan is calculated once after the draws; a model that cannot be
    /// calculated there has no value at plan, and its other numbers stand.
    #[test]
    fn a_plan_that_cannot_be_calculated_has_no_value_at_plan() {
        let mut config = test_config();
        config.outputs[0].variable = "profit".to_string();
        let mut engine = MonteCarloEngine::new(config).unwrap();
        engine.add_distribution("revenue", Distribution::normal(100.0, 10.0).unwrap());
        let result = engine
            .run_with_evaluator(&mut |inputs| {
                let revenue = inputs["revenue"];
                // Exactly the plan value: no draw lands there, so only the
                // calculation at plan fails
                if revenue.to_bits() == 100.0_f64.to_bits() {
                    return Err("division by zero".to_string());
                }
                Ok(HashMap::from([("profit".to_string(), revenue - 80.0)]))
            })
            .unwrap();
        let profit = &result.outputs["profit"];
        assert_eq!(profit.at_plan, None);
        assert!((profit.statistics.mean - 20.0).abs() < 1.0);
        assert!(!result.to_yaml().contains("at_plan"));
    }

    #[test]
    fn test_output_yaml() {
        let config = test_config();
        let mut engine = MonteCarloEngine::new(config).unwrap();

        let dist = Distribution::normal(100_000.0, 15_000.0).unwrap();
        engine.add_distribution("revenue", dist);

        let result = engine.run().unwrap();
        let yaml = result.to_yaml();

        assert!(yaml.contains("monte_carlo_results:"));
        assert!(yaml.contains("iterations: 10000"));
        assert!(yaml.contains("mean:"));
        assert!(yaml.contains("percentiles:"));
    }

    #[test]
    fn test_output_json() {
        let config = test_config();
        let mut engine = MonteCarloEngine::new(config).unwrap();

        let dist = Distribution::normal(100_000.0, 15_000.0).unwrap();
        engine.add_distribution("revenue", dist);

        let result = engine.run().unwrap();
        let json = result.to_json();

        assert!(json.contains("\"monte_carlo_results\""));
        assert!(json.contains("\"iterations\": 10000"));
        assert!(json.contains("\"mean\""));
    }

    /// A JSON object has no order, and `serde_json` writes its keys sorted as
    /// text (row 1, row 10, row 2): `output_order` lists them as configured.
    #[test]
    fn the_json_lists_the_outputs_in_the_order_configured() {
        let mut config = test_config();
        config.iterations = 10;
        let names = ["profit (row 2)", "profit (row 10)", "cost"];
        config.outputs = names.iter().map(|n| output(n)).collect();
        let mut engine = MonteCarloEngine::new(config).unwrap();
        engine.add_distribution("revenue", Distribution::normal(100.0, 10.0).unwrap());
        let result = engine
            .run_with_evaluator(&mut |_| {
                Ok(names.iter().map(|n| ((*n).to_string(), 1.0)).collect())
            })
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(&result.to_json()).unwrap();
        let results = &json["monte_carlo_results"];
        assert_eq!(results["output_order"], serde_json::json!(names));
        let keys: Vec<&String> = results["outputs"].as_object().unwrap().keys().collect();
        assert_eq!(keys, ["cost", "profit (row 10)", "profit (row 2)"]);
    }

    #[test]
    fn test_seed_reproducibility() {
        let config = test_config();

        let mut engine1 = MonteCarloEngine::new(config.clone()).unwrap();
        engine1.add_distribution("revenue", Distribution::normal(100.0, 10.0).unwrap());
        let result1 = engine1.run().unwrap();

        let mut engine2 = MonteCarloEngine::new(config).unwrap();
        engine2.add_distribution("revenue", Distribution::normal(100.0, 10.0).unwrap());
        let result2 = engine2.run().unwrap();

        // Same seed should produce identical results
        let samples1 = &result1.input_samples["revenue"];
        let samples2 = &result2.input_samples["revenue"];
        assert_eq!(samples1, samples2);
    }

    fn output(variable: &str) -> OutputConfig {
        OutputConfig {
            variable: variable.to_string(),
            percentiles: vec![],
            threshold: None,
            thresholds: vec![],
            expected_shortfall: vec![],
            higher_is_worse: false,
            row: None,
            label: None,
        }
    }

    fn config(outputs: Vec<OutputConfig>) -> MonteCarloConfig {
        MonteCarloConfig {
            enabled: true,
            iterations: 4,
            sampling: "monte_carlo".to_string(),
            seed: None,
            outputs,
            correlations: vec![],
        }
    }

    #[test]
    fn an_invalid_config_is_refused() {
        let mut bad = config(vec![]);
        bad.iterations = 0;
        assert_eq!(
            MonteCarloEngine::new(bad).err(),
            Some("iterations must be > 0".to_string())
        );
    }

    /// A distribution in an included file that does not parse names the file
    #[test]
    fn a_bad_distribution_in_an_include_names_the_include() {
        let mut included = ParsedModel::new();
        included.add_scalar(
            "growth".to_string(),
            crate::types::Variable::new(
                "growth".to_string(),
                None,
                Some("=MC.Uniform(2, 1)".to_string()),
            ),
        );
        let mut model = ParsedModel::new();
        model.resolved_includes.insert(
            "ext".to_string(),
            crate::types::ResolvedInclude {
                include: crate::types::Include::new("ext.yaml".to_string(), "ext".to_string()),
                resolved_path: std::path::PathBuf::from("ext.yaml"),
                model: included,
            },
        );
        let mut engine = MonteCarloEngine::new(config(vec![])).unwrap();
        assert_eq!(
            engine.parse_distributions_from_model(&model).err(),
            Some(
                "@ext.growth: Uniform distribution requires min < max (in the file included as 'ext')"
                    .to_string()
            )
        );
    }

    /// Without an evaluator an output is an input, found by its own name or
    /// by its name under `scalars.`; any other is refused.
    #[test]
    fn without_an_evaluator_an_output_must_be_an_input() {
        let mut engine = MonteCarloEngine::new(config(vec![output("x")])).unwrap();
        engine.add_distribution("scalars.x", Distribution::uniform(1.0, 2.0).unwrap());
        let result = engine.run().unwrap();
        let x = &result.outputs["x"];
        assert_eq!(
            (x.samples.len(), x.at_plan),
            (4, Some(1.5)),
            "the midpoint is the plan of a uniform"
        );

        let mut engine = MonteCarloEngine::new(config(vec![output("y")])).unwrap();
        engine.add_distribution("x", Distribution::uniform(1.0, 2.0).unwrap());
        assert_eq!(
            engine.run().err(),
            Some(
                "output 'y' is not an MC.* input; use run_with_evaluator to calculate it"
                    .to_string()
            )
        );
    }

    #[test]
    fn an_evaluator_that_leaves_out_an_output_stops_the_run() {
        let mut engine = MonteCarloEngine::new(config(vec![output("profit")])).unwrap();
        engine.add_distribution("x", Distribution::uniform(1.0, 2.0).unwrap());
        let result =
            engine.run_with_evaluator(&mut |_| Ok(HashMap::from([("loss".to_string(), 1.0)])));
        assert_eq!(
            result.err(),
            Some("Iteration 1: no value for output 'profit'".to_string())
        );
    }

    /// A correlation names a distribution by its whole name or by the last
    /// part of only one; the errors name the distributions there are.
    #[test]
    fn a_correlation_variable_must_name_one_distribution() {
        let names = ["a.growth", "b.growth", "c.price"].map(String::from);
        assert_eq!(resolve_variable(&names, "b.growth"), Ok(1));
        assert_eq!(resolve_variable(&names, "price"), Ok(2));
        assert_eq!(
            resolve_variable(&names, "growth"),
            Err(
                "correlation variable 'growth' is ambiguous: a.growth, b.growth. Use the full name."
                    .to_string()
            )
        );
        let mut corr = config(vec![]);
        corr.correlations = vec![crate::monte_carlo::config::CorrelationConfig {
            variables: vec!["cost".to_string(), "price".to_string()],
            coefficient: 0.5,
        }];
        let mut engine = MonteCarloEngine::new(corr).unwrap();
        engine.add_distribution("c.price", Distribution::uniform(1.0, 2.0).unwrap());
        assert_eq!(
            engine.run().err(),
            Some(
                "correlation variable 'cost' has no MC.* distribution. Distributions: c.price"
                    .to_string()
            )
        );
    }

    /// The correlated draws of a seeded run, to the bit: four inputs, three of
    /// them correlated (`a`, `c` and `d`, not side by side among the names)
    /// and `b` left as drawn. Each column's bits are folded into one number;
    /// the folds were taken from the run before the loop over the draws was
    /// rewritten, so a rewrite that moves any draw by one bit fails.
    #[test]
    fn a_seeded_correlated_run_gives_the_same_draws_to_the_bit() {
        let mut seeded = config(vec![output("a")]);
        seeded.iterations = 500;
        seeded.seed = Some(2026);
        seeded.sampling = "latin_hypercube".to_string();
        seeded.correlations = [("a", "c", 0.7), ("c", "d", -0.4), ("a", "d", 0.2)]
            .map(
                |(first, second, coefficient)| crate::monte_carlo::config::CorrelationConfig {
                    variables: vec![first.to_string(), second.to_string()],
                    coefficient,
                },
            )
            .to_vec();
        let mut engine = MonteCarloEngine::new(seeded).unwrap();
        engine.add_distribution("a", Distribution::normal(100.0, 15.0).unwrap());
        engine.add_distribution("b", Distribution::uniform(0.0, 1.0).unwrap());
        engine.add_distribution("c", Distribution::triangular(1.0, 2.0, 4.0).unwrap());
        engine.add_distribution("d", Distribution::lognormal(50.0, 10.0).unwrap());
        let result = engine.run().unwrap();
        let fold = |name: &str| {
            result.input_samples[name]
                .iter()
                .fold(0_u64, |hash, x| hash.rotate_left(7) ^ x.to_bits())
        };
        assert_eq!(
            ["a", "b", "c", "d"].map(fold),
            [
                0x0c9b_33c7_4009_cfee,
                0xc4a0_f629_5c08_9e0b,
                0x864a_22a9_b03a_b6aa,
                0xe851_4dba_d25f_2758,
            ]
        );
        assert_eq!(
            ["a", "b", "c", "d"].map(|name| result.input_samples[name][0].to_bits()),
            [
                0x4055_ac89_f6de_8a9e,
                0x3fe0_8556_ef84_613c,
                0x3ffb_9888_a2c9_defe,
                0x404c_a13d_0c73_3451,
            ]
        );
        // And the draws are correlated as configured, which the bits alone do not say
        let rank = |first: &str, second: &str| {
            super::super::sensitivity::spearman_correlation(
                &result.input_samples[first],
                &result.input_samples[second],
            )
        };
        assert!((rank("a", "c") - 0.7).abs() < 0.05, "{}", rank("a", "c"));
        assert!((rank("c", "d") + 0.4).abs() < 0.05, "{}", rank("c", "d"));
        assert!(rank("a", "b").abs() < 0.1, "{}", rank("a", "b"));
    }

    /// Inputs that move the output as strongly, one each way, are in name order
    #[test]
    fn drivers_of_equal_strength_are_in_name_order() {
        let inputs = HashMap::from([
            ("b".to_string(), vec![3.0, 2.0, 1.0]),
            ("a".to_string(), vec![1.0, 2.0, 3.0]),
        ]);
        let ranked = drivers(&[10.0, 20.0, 30.0], &inputs);
        assert_eq!(
            ranked,
            vec![
                Driver {
                    input: "a".to_string(),
                    rank_correlation: 1.0
                },
                Driver {
                    input: "b".to_string(),
                    rank_correlation: -1.0
                },
            ]
        );
    }

    /// An output that does not vary has no drivers; without a seed or
    /// thresholds those lines are left out, and the expected shortfall is
    /// shown in YAML and JSON.
    #[test]
    fn a_result_shows_only_what_it_has() {
        let mut out = output("profit");
        out.expected_shortfall = vec![25];
        let mut engine = MonteCarloEngine::new(config(vec![out])).unwrap();
        engine.add_distribution("x", Distribution::uniform(1.0, 2.0).unwrap());
        let mut result = engine
            .run_with_evaluator(&mut |_| Ok(HashMap::from([("profit".to_string(), 7.0)])))
            .unwrap();
        result.execution_time_ms = 0;
        assert_eq!(
            result.to_yaml(),
            "monte_carlo_results:\n  iterations: 4\n  execution_time_ms: 0\n  sampling: monte_carlo\n\n  outputs:\n    profit:\n      mean: 7.0000\n      at_plan: 7.0000\n      median: 7.0000\n      std_dev: 0.0000\n      min: 7.0000\n      max: 7.0000\n      percentiles:\n        p5: 7.0000\n        p10: 7.0000\n        p25: 7.0000\n        p50: 7.0000\n        p75: 7.0000\n        p90: 7.0000\n        p95: 7.0000\n      expected_shortfall:\n        worst_25_percent: 7.0000\n"
        );
        let json: serde_json::Value = serde_json::from_str(&result.to_json()).unwrap();
        let profit = &json["monte_carlo_results"]["outputs"]["profit"];
        assert_eq!(
            (
                &profit["expected_shortfall"],
                &profit["drivers"],
                &json["monte_carlo_results"]["seed"]
            ),
            (
                &serde_json::json!([{"percent": 25, "mean": 7.0}]),
                &serde_json::json!([]),
                &serde_json::Value::Null
            )
        );
    }

    mod uncertain_rows {
        //! A table column whose formula is a distribution is one input per row.

        use super::*;
        use crate::types::{Column, ColumnValue, Table};

        fn model(formula: &str) -> ParsedModel {
            let mut table = Table::new("t".to_string());
            table.add_column(Column::new(
                "n".to_string(),
                ColumnValue::Number(vec![1.0, 2.0, 3.0]),
            ));
            table.add_row_formula("shock".to_string(), formula.to_string());
            table.add_row_formula("twice".to_string(), "=n * 2".to_string());
            let mut model = ParsedModel::new();
            model.add_table(table);
            model
        }

        #[test]
        fn each_row_is_an_input_at_its_plan() {
            let mut engine = MonteCarloEngine::new(MonteCarloConfig::default()).unwrap();
            engine
                .parse_distributions_from_model(&model("=MC.Triangular(1, 2, 4)"))
                .unwrap();
            let plan = engine.plan_inputs();
            let names: Vec<&str> = plan.keys().map(String::as_str).collect();
            // The column of plain formulas draws nothing
            assert_eq!(
                names,
                ["t.shock (row 1)", "t.shock (row 2)", "t.shock (row 3)"]
            );
            for input in plan.values() {
                assert!((input.value - 2.0).abs() <= 0.0);
                assert_eq!(input.from, "mode");
            }
        }

        #[test]
        fn a_column_whose_distribution_does_not_parse_is_named() {
            let mut engine = MonteCarloEngine::new(MonteCarloConfig::default()).unwrap();
            let err = engine
                .parse_distributions_from_model(&model("=MC.Normal(0, -1)"))
                .unwrap_err();
            assert!(err.starts_with("t.shock: "), "{err}");
        }
    }
}
