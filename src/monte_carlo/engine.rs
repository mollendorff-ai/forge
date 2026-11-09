//! Monte Carlo Simulation Engine
//!
//! Orchestrates the simulation:
//! 1. Parse distributions from model
//! 2. Generate samples using specified method
//! 3. Evaluate formulas for each iteration
//! 4. Compute output statistics

use indexmap::IndexMap;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;

use super::config::MonteCarloConfig;
use super::correlation::CorrelationMatrix;
use super::distributions::{
    parse_distribution, standard_normal_cdf, standard_normal_quantile, Distribution,
};
use super::sampler::{Sampler, SamplingMethod};
use super::statistics::{
    calculate_percentiles, evaluate_threshold, parse_threshold, Histogram, Statistics,
};
use crate::types::ParsedModel;

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
        .filter_map(|t| {
            let (op, value) = parse_threshold(t).ok()?;
            Some((t.to_string(), evaluate_threshold(&samples, &op, value)))
        })
        .collect();

    let drivers = drivers(&samples, input_samples);
    OutputResult {
        variable: output_config.variable.clone(),
        statistics,
        samples,
        histogram,
        threshold_probabilities,
        drivers,
    }
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
}

impl MonteCarloEngine {
    /// Create a new engine with the given configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid (see [`MonteCarloConfig::validate`]).
    pub fn new(config: MonteCarloConfig) -> Result<Self, String> {
        config.validate()?;

        let method = SamplingMethod::from_str(&config.sampling)?;
        let sampler = Sampler::new(method, config.seed);

        Ok(Self {
            config,
            sampler,
            distributions: HashMap::new(),
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
                    let dist = parse_distribution(formula_content)?;
                    self.add_distribution(&format!("{prefix}{name}"), dist);
                }
            }
        }
        Ok(())
    }

    /// Run the simulation
    ///
    /// # Errors
    ///
    /// Returns an error if output variable samples cannot be resolved.
    pub fn run(&mut self) -> Result<SimulationResult, String> {
        let start = Instant::now();
        let n = self.config.iterations;

        let input_samples = self.sample_inputs(n)?;

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
            outputs.insert(
                var.clone(),
                summarize(output_config, samples, &input_samples),
            );
        }

        // cast_possible_truncation: simulation time in ms will never exceed u64::MAX
        #[allow(clippy::cast_possible_truncation)]
        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(SimulationResult {
            config: self.config.clone(),
            iterations_completed: n,
            execution_time_ms,
            outputs,
            input_samples,
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
    pub fn run_with_evaluator<F>(&mut self, mut evaluator: F) -> Result<SimulationResult, String>
    where
        F: FnMut(&HashMap<String, f64>) -> Result<HashMap<String, f64>, String>,
    {
        let start = Instant::now();
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

        let mut outputs = IndexMap::new();
        for output_config in &self.config.outputs {
            let var = &output_config.variable;
            let samples = output_samples.remove(var).unwrap_or_default();
            outputs.insert(
                var.clone(),
                summarize(output_config, samples, &input_samples),
            );
        }

        // cast_possible_truncation: simulation time in ms will never exceed u64::MAX
        #[allow(clippy::cast_possible_truncation)]
        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(SimulationResult {
            config: self.config.clone(),
            iterations_completed: n,
            execution_time_ms,
            outputs,
            input_samples,
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
        let columns: Vec<usize> = matrix
            .variables
            .iter()
            .map(|var| resolve_variable(names, var))
            .collect::<Result<_, _>>()?;

        let n = uniforms.first().map_or(0, Vec::len);
        let mut scores = vec![0.0; columns.len()];
        // Row `i` is spread across several columns, so an index is clearer than iterators
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            for (score, &column) in scores.iter_mut().zip(&columns) {
                *score = standard_normal_quantile(uniforms[column][i]);
            }
            let correlated = matrix.correlate(&scores)?;
            for (value, &column) in correlated.iter().zip(&columns) {
                uniforms[column][i] = standard_normal_cdf(*value);
            }
        }
        Ok(())
    }

    /// Get the sampler
    #[must_use]
    pub const fn sampler(&self) -> &Sampler {
        &self.sampler
    }

    /// Get mutable sampler
    pub const fn sampler_mut(&mut self) -> &mut Sampler {
        &mut self.sampler
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
    /// # Errors
    ///
    /// Returns an error if JSON serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
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
                    "median": result.statistics.median,
                    "std_dev": result.statistics.std_dev,
                    "min": result.statistics.min,
                    "max": result.statistics.max,
                    "percentiles": percentiles,
                    "thresholds": thresholds,
                    "drivers": drivers,
                }),
            );
        }

        let result_json = json!({
            "monte_carlo_results": {
                "iterations": self.iterations_completed,
                "execution_time_ms": self.execution_time_ms,
                "sampling": self.config.sampling,
                "seed": self.config.seed,
                "outputs": outputs_json,
            }
        });

        to_string_pretty(&result_json)
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
                label: None,
            }],
            correlations: vec![],
        };

        let mut engine = MonteCarloEngine::new(config).unwrap();

        engine.add_distribution("revenue", Distribution::normal(100.0, 10.0).unwrap());
        engine.add_distribution("costs", Distribution::normal(80.0, 5.0).unwrap());

        let result = engine
            .run_with_evaluator(|inputs| {
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
        let json = result.to_json().unwrap();

        assert!(json.contains("\"monte_carlo_results\""));
        assert!(json.contains("\"iterations\": 10000"));
        assert!(json.contains("\"mean\""));
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
}
