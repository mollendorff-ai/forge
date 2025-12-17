//! Monte Carlo Simulation Engine
//!
//! Orchestrates the simulation:
//! 1. Parse distributions from model
//! 2. Generate samples using specified method
//! 3. Evaluate formulas for each iteration
//! 4. Compute output statistics

use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;

use super::config::MonteCarloConfig;
use super::distributions::{parse_distribution, Distribution};
use super::sampler::{Sampler, SamplingMethod};
use super::statistics::{evaluate_threshold, parse_threshold, Histogram, Statistics};
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
    /// Results for each tracked output variable
    pub outputs: HashMap<String, OutputResult>,
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
    /// Probability thresholds (threshold string -> probability)
    pub threshold_probabilities: HashMap<String, f64>,
}

/// Monte Carlo simulation engine
pub struct MonteCarloEngine {
    config: MonteCarloConfig,
    sampler: Sampler,
    distributions: HashMap<String, Distribution>,
}

impl MonteCarloEngine {
    /// Create a new engine with the given configuration
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

    /// Parse distributions from a model's scalar formulas
    pub fn parse_distributions_from_model(&mut self, model: &ParsedModel) -> Result<(), String> {
        for (name, scalar) in &model.scalars {
            if let Some(formula) = &scalar.formula {
                let formula = formula.trim();
                // Check if formula starts with =MC. or MC.
                let formula_content = formula.strip_prefix('=').unwrap_or(formula);

                if formula_content.starts_with("MC.") {
                    let dist = parse_distribution(formula_content)?;
                    self.add_distribution(name, dist);
                }
            }
        }
        Ok(())
    }

    /// Run the simulation
    pub fn run(&mut self) -> Result<SimulationResult, String> {
        let start = Instant::now();
        let n = self.config.iterations;

        // Generate samples for each distribution
        let mut input_samples: HashMap<String, Vec<f64>> = HashMap::new();

        for (var_name, dist) in &self.distributions {
            let samples = dist.sample_n(self.sampler.rng_mut(), n);
            input_samples.insert(var_name.clone(), samples);
        }

        // For now, output results are the same as input samples
        // (Full formula evaluation will be added when integrating with calculator)
        let mut outputs = HashMap::new();

        for output_config in &self.config.outputs {
            let var = &output_config.variable;

            // Get samples for this variable (either from inputs or computed)
            // Try exact match first, then with "scalars." prefix
            let samples = input_samples
                .get(var)
                .or_else(|| input_samples.get(&format!("scalars.{var}")))
                .cloned()
                .unwrap_or_else(|| vec![0.0; n]);

            // Calculate statistics
            let statistics = Statistics::from_samples(&samples);

            // Create histogram (50 bins default)
            let histogram = Histogram::from_samples(&samples, 50);

            // Evaluate thresholds
            let mut threshold_probabilities = HashMap::new();
            if let Some(threshold_str) = &output_config.threshold {
                if let Ok((op, value)) = parse_threshold(threshold_str) {
                    let prob = evaluate_threshold(&samples, &op, value);
                    threshold_probabilities.insert(threshold_str.clone(), prob);
                }
            }

            outputs.insert(
                var.clone(),
                OutputResult {
                    variable: var.clone(),
                    statistics,
                    samples,
                    histogram,
                    threshold_probabilities,
                },
            );
        }

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
    /// The evaluator takes input values for one iteration and returns output values
    pub fn run_with_evaluator<F>(&mut self, mut evaluator: F) -> Result<SimulationResult, String>
    where
        F: FnMut(&HashMap<String, f64>) -> HashMap<String, f64>,
    {
        let start = Instant::now();
        let n = self.config.iterations;

        // Generate samples for each distribution
        let mut input_samples: HashMap<String, Vec<f64>> = HashMap::new();
        for (var_name, dist) in &self.distributions {
            let samples = dist.sample_n(self.sampler.rng_mut(), n);
            input_samples.insert(var_name.clone(), samples);
        }

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
            let outputs = evaluator(&inputs);

            // Store output values
            for var in &output_vars {
                let value = outputs.get(var).copied().unwrap_or(0.0);
                output_samples.get_mut(var).unwrap().push(value);
            }
        }

        // Calculate statistics for outputs
        let mut outputs = HashMap::new();
        for output_config in &self.config.outputs {
            let var = &output_config.variable;
            let samples = output_samples.get(var).cloned().unwrap_or_default();

            let statistics = Statistics::from_samples(&samples);
            let histogram = Histogram::from_samples(&samples, 50);

            let mut threshold_probabilities = HashMap::new();
            if let Some(threshold_str) = &output_config.threshold {
                if let Ok((op, value)) = parse_threshold(threshold_str) {
                    let prob = evaluate_threshold(&samples, &op, value);
                    threshold_probabilities.insert(threshold_str.clone(), prob);
                }
            }

            outputs.insert(
                var.clone(),
                OutputResult {
                    variable: var.clone(),
                    statistics,
                    samples,
                    histogram,
                    threshold_probabilities,
                },
            );
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(SimulationResult {
            config: self.config.clone(),
            iterations_completed: n,
            execution_time_ms,
            outputs,
            input_samples,
        })
    }

    /// Get the sampler
    pub fn sampler(&self) -> &Sampler {
        &self.sampler
    }

    /// Get mutable sampler
    pub fn sampler_mut(&mut self) -> &mut Sampler {
        &mut self.sampler
    }
}

impl SimulationResult {
    /// Format results as YAML string
    pub fn to_yaml(&self) -> String {
        let mut output = String::new();

        output.push_str("monte_carlo_results:\n");
        output.push_str(&format!("  iterations: {}\n", self.iterations_completed));
        output.push_str(&format!(
            "  execution_time_ms: {}\n",
            self.execution_time_ms
        ));
        output.push_str(&format!("  sampling: {}\n", self.config.sampling));
        if let Some(seed) = self.config.seed {
            output.push_str(&format!("  seed: {seed}\n"));
        }

        output.push_str("\n  outputs:\n");
        for (var, result) in &self.outputs {
            output.push_str(&format!("    {var}:\n"));
            output.push_str(&format!("      mean: {:.4}\n", result.statistics.mean));
            output.push_str(&format!("      median: {:.4}\n", result.statistics.median));
            output.push_str(&format!(
                "      std_dev: {:.4}\n",
                result.statistics.std_dev
            ));
            output.push_str(&format!("      min: {:.4}\n", result.statistics.min));
            output.push_str(&format!("      max: {:.4}\n", result.statistics.max));

            output.push_str("      percentiles:\n");
            for (p, v) in &result.statistics.percentiles {
                output.push_str(&format!("        p{p}: {v:.4}\n"));
            }

            if !result.threshold_probabilities.is_empty() {
                output.push_str("      thresholds:\n");
                for (t, prob) in &result.threshold_probabilities {
                    output.push_str(&format!("        \"{t}\": {prob:.4}\n"));
                }
            }
        }

        output
    }

    /// Format results as JSON string
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

        let dist = Distribution::normal(100000.0, 15000.0).unwrap();
        engine.add_distribution("revenue", dist);

        assert!(engine.distributions.contains_key("revenue"));
    }

    #[test]
    fn test_run_simulation() {
        let config = test_config();
        let mut engine = MonteCarloEngine::new(config).unwrap();

        let dist = Distribution::normal(100000.0, 15000.0).unwrap();
        engine.add_distribution("revenue", dist);

        let result = engine.run().unwrap();

        assert_eq!(result.iterations_completed, 10000);
        assert!(result.input_samples.contains_key("revenue"));
        assert!(result.outputs.contains_key("revenue"));

        // Check statistics are reasonable
        let revenue_result = &result.outputs["revenue"];
        assert!((revenue_result.statistics.mean - 100000.0).abs() < 2000.0);
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
                outputs
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

        let dist = Distribution::normal(100000.0, 15000.0).unwrap();
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

        let dist = Distribution::normal(100000.0, 15000.0).unwrap();
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
