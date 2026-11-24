//! Monte Carlo Simulation Module
//!
//! Provides probabilistic analysis capabilities for FP&A:
//! - Distribution sampling (Normal, Triangular, Uniform, PERT, Lognormal)
//! - Latin Hypercube and Monte Carlo sampling methods
//! - Statistical output (percentiles, probability thresholds)
//! - Sensitivity analysis via correlation coefficients
//!
//! # Example
//!
//! ```yaml
//! monte_carlo:
//!   enabled: true
//!   iterations: 10000
//!   sampling: latin_hypercube
//!   seed: 12345
//!   outputs:
//!     - variable: valuation.npv
//!       percentiles: [10, 50, 90]
//!       threshold: "> 0"
//!
//! assumptions:
//!   revenue_growth: =MC.Normal(0.15, 0.05)
//!   initial_cost: =MC.Triangular(80000, 100000, 150000)
//! ```
//!
//! See ADR-016 for architecture decisions.

pub mod config;
pub mod correlation;
pub mod distributions;
pub mod engine;
#[cfg(feature = "excel")]
pub mod excel_export;
pub mod sampler;
pub mod sensitivity;
pub mod statistics;

// Re-exports
pub use config::MonteCarloConfig;
pub use correlation::CorrelationMatrix;
pub use distributions::{Distribution, DistributionType};
pub use engine::{Driver, MonteCarloEngine, OutputResult, SimulationResult};
pub use sampler::{Sampler, SamplingMethod};
pub use sensitivity::SensitivityAnalysis;
pub use statistics::Statistics;

/// Whether a formula is a distribution, as the simulation reads one: an
/// uncertain input
#[must_use]
pub fn is_distribution(formula: &str) -> bool {
    let formula = formula.trim();
    formula
        .strip_prefix('=')
        .unwrap_or(formula)
        .starts_with("MC.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::precision;

    mod integration {
        //! Integration tests for Monte Carlo module
        //!
        //! Tests the full simulation workflow from config to results.

        use super::*;
        use crate::monte_carlo::config::{MonteCarloConfig, OutputConfig};
        use crate::monte_carlo::distributions::Distribution;

        /// Test full simulation workflow
        #[test]
        fn test_full_simulation_workflow() {
            let config = MonteCarloConfig {
                enabled: true,
                iterations: 5000,
                sampling: "latin_hypercube".to_string(),
                seed: Some(42),
                outputs: vec![OutputConfig {
                    variable: "npv".to_string(),
                    percentiles: vec![10, 50, 90],
                    threshold: Some("> 0".to_string()),
                    thresholds: vec![],
                    expected_shortfall: vec![],
                    higher_is_worse: false,
                    row: None,
                    label: Some("Net Present Value".to_string()),
                }],
                correlations: vec![],
            };

            let mut engine = MonteCarloEngine::new(config).unwrap();

            // Add distributions for inputs
            engine.add_distribution(
                "revenue",
                Distribution::normal(1_000_000.0, 150_000.0).unwrap(),
            );
            engine.add_distribution(
                "costs",
                Distribution::triangular(400_000.0, 500_000.0, 600_000.0).unwrap(),
            );
            engine.add_distribution("discount_rate", Distribution::uniform(0.08, 0.12).unwrap());

            // Run with simple NPV calculation
            let result = engine
                .run_with_evaluator(&mut |inputs| {
                    let revenue = inputs.get("revenue").copied().unwrap_or(1_000_000.0);
                    let costs = inputs.get("costs").copied().unwrap_or(500_000.0);
                    let discount_rate = inputs.get("discount_rate").copied().unwrap_or(0.10);

                    // Simple 3-year NPV
                    let cf1 = revenue - costs;
                    let cf2 = f64::mul_add(revenue, 1.05, -costs);
                    let cf3 = f64::mul_add(revenue, 1.10, -costs);

                    let npv = cf1 / (1.0 + discount_rate)
                        + cf2 / (1.0 + discount_rate).powi(2)
                        + cf3 / (1.0 + discount_rate).powi(3)
                        - 1_000_000.0; // Initial investment

                    let mut outputs = std::collections::HashMap::new();
                    outputs.insert("npv".to_string(), npv);
                    Ok(outputs)
                })
                .unwrap();

            // Verify results
            assert_eq!(result.iterations_completed, 5000);

            let npv_result = result.outputs.get("npv").unwrap();

            // Mean NPV should be positive (profitable project)
            assert!(
                npv_result.statistics.mean > 0.0,
                "Mean NPV should be positive: {}",
                npv_result.statistics.mean
            );

            // P10 might be negative (some scenarios unprofitable)
            // P90 should be significantly positive
            let p10 = npv_result.statistics.percentile(10).unwrap();
            let p90 = npv_result.statistics.percentile(90).unwrap();
            assert!(p90 > p10, "P90 should be greater than P10");

            // Probability of positive NPV
            let prob = npv_result.threshold_probabilities.get("> 0").unwrap();
            assert!(*prob > 0.5, "Probability of positive NPV should be > 50%");

            // Check histogram exists
            assert_ne!(npv_result.histogram.counts.len(), 0);
        }

        /// Test Latin Hypercube vs Monte Carlo convergence
        #[test]
        fn test_lhs_vs_mc_convergence() {
            let n = 1000;

            // Monte Carlo
            let mc_config = MonteCarloConfig {
                enabled: true,
                iterations: n,
                sampling: "monte_carlo".to_string(),
                seed: Some(123),
                outputs: vec![OutputConfig {
                    variable: "x".to_string(),
                    percentiles: vec![50],
                    threshold: None,
                    thresholds: vec![],
                    expected_shortfall: vec![],
                    higher_is_worse: false,
                    row: None,
                    label: None,
                }],
                correlations: vec![],
            };

            let mut mc_engine = MonteCarloEngine::new(mc_config).unwrap();
            mc_engine.add_distribution("x", Distribution::uniform(0.0, 1.0).unwrap());
            let mc_result = mc_engine.run().unwrap();

            // Latin Hypercube
            let lhs_config = MonteCarloConfig {
                enabled: true,
                iterations: n,
                sampling: "latin_hypercube".to_string(),
                seed: Some(123),
                outputs: vec![OutputConfig {
                    variable: "x".to_string(),
                    percentiles: vec![50],
                    threshold: None,
                    thresholds: vec![],
                    expected_shortfall: vec![],
                    higher_is_worse: false,
                    row: None,
                    label: None,
                }],
                correlations: vec![],
            };

            let mut lhs_engine = MonteCarloEngine::new(lhs_config).unwrap();
            lhs_engine.add_distribution("x", Distribution::uniform(0.0, 1.0).unwrap());
            let lhs_result = lhs_engine.run().unwrap();

            // Both should have mean close to 0.5, but LHS should be closer
            let mc_mean = mc_result.outputs["x"].statistics.mean;
            let lhs_mean = lhs_result.outputs["x"].statistics.mean;

            let mc_error = (mc_mean - 0.5).abs();
            let lhs_error = (lhs_mean - 0.5).abs();

            // LHS should have lower or equal error (on average)
            // This test may occasionally fail due to randomness, but should pass most of the time
            println!("MC error: {mc_error}, LHS error: {lhs_error}");

            // Both should be within reasonable bounds
            assert!(mc_error < 0.05, "MC error too high: {mc_error}");
            assert!(lhs_error < 0.03, "LHS error too high: {lhs_error}");
        }

        /// Test distribution statistical accuracy
        #[test]
        fn test_distribution_accuracy() {
            let config = MonteCarloConfig::new()
                .enabled()
                .with_iterations(50000)
                .with_sampling("latin_hypercube")
                .with_seed(999);

            let mut engine = MonteCarloEngine::new(config).unwrap();

            // Normal(100, 10)
            engine.add_distribution("normal", Distribution::normal(100.0, 10.0).unwrap());

            // Uniform(0, 100)
            engine.add_distribution("uniform", Distribution::uniform(0.0, 100.0).unwrap());

            // Triangular(0, 50, 100)
            engine.add_distribution(
                "triangular",
                Distribution::triangular(0.0, 50.0, 100.0).unwrap(),
            );

            // PERT(0, 30, 100)
            engine.add_distribution("pert", Distribution::pert(0.0, 30.0, 100.0).unwrap());

            let result = engine.run().unwrap();

            // Normal: mean ≈ 100
            let normal_samples = &result.input_samples["normal"];
            let normal_mean: f64 =
                normal_samples.iter().sum::<f64>() / precision::count(normal_samples.len());
            assert!(
                (normal_mean - 100.0).abs() < 1.0,
                "Normal mean {normal_mean} not close to 100"
            );

            // Uniform: mean ≈ 50
            let uniform_samples = &result.input_samples["uniform"];
            let uniform_mean: f64 =
                uniform_samples.iter().sum::<f64>() / precision::count(uniform_samples.len());
            assert!(
                (uniform_mean - 50.0).abs() < 1.0,
                "Uniform mean {uniform_mean} not close to 50"
            );

            // Triangular: mean ≈ 50 ((0+50+100)/3)
            let tri_samples = &result.input_samples["triangular"];
            let tri_mean: f64 =
                tri_samples.iter().sum::<f64>() / precision::count(tri_samples.len());
            assert!(
                (tri_mean - 50.0).abs() < 1.0,
                "Triangular mean {tri_mean} not close to 50"
            );

            // PERT: mean ≈ 36.67 ((0+4*30+100)/6)
            let pert_samples = &result.input_samples["pert"];
            let pert_mean: f64 =
                pert_samples.iter().sum::<f64>() / precision::count(pert_samples.len());
            let expected_pert_mean = (4.0f64.mul_add(30.0, 0.0) + 100.0) / 6.0;
            assert!(
                (pert_mean - expected_pert_mean).abs() < 2.0,
                "PERT mean {pert_mean} not close to {expected_pert_mean}"
            );
        }

        /// Test percentile accuracy
        #[test]
        fn test_percentile_accuracy() {
            let config = MonteCarloConfig::new()
                .enabled()
                .with_iterations(10000)
                .with_seed(777);

            let mut engine = MonteCarloEngine::new(config).unwrap();

            // Uniform(0, 100) - percentiles should be approximately equal to percentile values
            engine.add_distribution("uniform", Distribution::uniform(0.0, 100.0).unwrap());

            let result = engine.run().unwrap();
            let samples = &result.input_samples["uniform"];
            let stats = Statistics::from_samples(samples);

            // P10 ≈ 10
            let p10 = stats.percentile(10).unwrap();
            assert!((p10 - 10.0).abs() < 2.0, "P10 {p10} not close to 10");

            // P50 ≈ 50
            let p50 = stats.percentile(50).unwrap();
            assert!((p50 - 50.0).abs() < 2.0, "P50 {p50} not close to 50");

            // P90 ≈ 90
            let p90 = stats.percentile(90).unwrap();
            assert!((p90 - 90.0).abs() < 2.0, "P90 {p90} not close to 90");
        }

        /// Test performance (10K iterations < 5 seconds)
        #[test]
        fn test_performance() {
            use std::time::Instant;

            let config = MonteCarloConfig::new()
                .enabled()
                .with_iterations(10000)
                .with_seed(111);

            let mut engine = MonteCarloEngine::new(config).unwrap();

            // Add 5 distributions
            engine.add_distribution("d1", Distribution::normal(100.0, 10.0).unwrap());
            engine.add_distribution("d2", Distribution::triangular(0.0, 50.0, 100.0).unwrap());
            engine.add_distribution("d3", Distribution::uniform(0.0, 100.0).unwrap());
            engine.add_distribution("d4", Distribution::pert(0.0, 30.0, 100.0).unwrap());
            engine.add_distribution("d5", Distribution::normal(1000.0, 100.0).unwrap());

            let start = Instant::now();
            let result = engine.run().unwrap();
            let elapsed = start.elapsed();

            assert_eq!(result.iterations_completed, 10000);
            let seconds = elapsed.as_secs_f64();
            assert!(
                elapsed.as_secs() < 5,
                "Performance test failed: {seconds} seconds"
            );

            println!("10K iterations with 5 distributions: {elapsed:?}");
        }

        /// Test YAML output format
        #[test]
        fn test_yaml_output() {
            let config = MonteCarloConfig {
                enabled: true,
                iterations: 100,
                sampling: "latin_hypercube".to_string(),
                seed: Some(42),
                outputs: vec![OutputConfig {
                    variable: "test".to_string(),
                    percentiles: vec![10, 50, 90],
                    threshold: Some("> 50".to_string()),
                    thresholds: vec![],
                    expected_shortfall: vec![],
                    higher_is_worse: false,
                    row: None,
                    label: None,
                }],
                correlations: vec![],
            };

            let mut engine = MonteCarloEngine::new(config).unwrap();
            engine.add_distribution("test", Distribution::uniform(0.0, 100.0).unwrap());

            let result = engine.run().unwrap();
            let yaml = result.to_yaml();

            // Verify YAML structure
            assert!(yaml.contains("monte_carlo_results:"));
            assert!(yaml.contains("iterations: 100"));
            assert!(yaml.contains("sampling: latin_hypercube"));
            assert!(yaml.contains("seed: 42"));
            assert!(yaml.contains("mean:"));
            assert!(yaml.contains("median:"));
            assert!(yaml.contains("percentiles:"));
            assert!(yaml.contains("p10:"));
            assert!(yaml.contains("p50:"));
            assert!(yaml.contains("p90:"));
            assert!(yaml.contains("\"> 50\":"));
        }

        /// Test JSON output format
        #[test]
        fn test_json_output() {
            let config = MonteCarloConfig {
                enabled: true,
                iterations: 100,
                sampling: "latin_hypercube".to_string(),
                seed: Some(42),
                outputs: vec![OutputConfig {
                    variable: "test".to_string(),
                    percentiles: vec![10, 50, 90],
                    threshold: Some("> 50".to_string()),
                    thresholds: vec![],
                    expected_shortfall: vec![],
                    higher_is_worse: false,
                    row: None,
                    label: None,
                }],
                correlations: vec![],
            };

            let mut engine = MonteCarloEngine::new(config).unwrap();
            engine.add_distribution("test", Distribution::uniform(0.0, 100.0).unwrap());

            let result = engine.run().unwrap();
            let json = result.to_json();

            // Verify JSON is valid and contains expected fields
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

            assert_eq!(
                parsed["monte_carlo_results"]["iterations"]
                    .as_u64()
                    .unwrap(),
                100
            );
            assert_eq!(
                parsed["monte_carlo_results"]["sampling"].as_str().unwrap(),
                "latin_hypercube"
            );
            assert_eq!(parsed["monte_carlo_results"]["seed"].as_u64().unwrap(), 42);
            assert!(parsed["monte_carlo_results"]["outputs"]["test"]["mean"].is_number());
        }
    }

    mod sampling {
        //! Tests that the engine honours `seed`, `sampling` and `correlations`

        use super::config::{CorrelationConfig, MonteCarloConfig};
        use super::distributions::Distribution;
        use super::engine::MonteCarloEngine;
        use crate::core::precision;
        use num_traits::ToPrimitive;
        use std::collections::HashMap;

        fn config(sampling: &str, seed: u64, iterations: usize) -> MonteCarloConfig {
            MonteCarloConfig {
                enabled: true,
                iterations,
                sampling: sampling.to_string(),
                seed: Some(seed),
                ..MonteCarloConfig::default()
            }
        }

        fn samples(
            config: MonteCarloConfig,
            dists: Vec<(&str, Distribution)>,
        ) -> HashMap<String, Vec<f64>> {
            let mut engine = MonteCarloEngine::new(config).unwrap();
            for (name, dist) in dists {
                engine.add_distribution(name, dist);
            }
            engine.run().unwrap().input_samples
        }

        fn five_variables() -> Vec<(&'static str, Distribution)> {
            vec![
                ("a.revenue", Distribution::normal(100.0, 10.0).unwrap()),
                (
                    "a.costs",
                    Distribution::triangular(50.0, 60.0, 90.0).unwrap(),
                ),
                ("b.growth", Distribution::uniform(-0.1, 0.3).unwrap()),
                ("b.duration", Distribution::pert(3.0, 5.0, 12.0).unwrap()),
                ("c.price", Distribution::lognormal(20.0, 4.0).unwrap()),
            ]
        }

        fn pearson(x: &[f64], y: &[f64]) -> f64 {
            let n = precision::count(x.len());
            let mean_x = x.iter().sum::<f64>() / n;
            let mean_y = y.iter().sum::<f64>() / n;
            let (mut cov, mut var_x, mut var_y) = (0.0_f64, 0.0_f64, 0.0_f64);
            for (a, b) in x.iter().zip(y) {
                cov = (a - mean_x).mul_add(b - mean_y, cov);
                var_x = (a - mean_x).mul_add(a - mean_x, var_x);
                var_y = (b - mean_y).mul_add(b - mean_y, var_y);
            }
            cov / (var_x * var_y).sqrt()
        }

        #[test]
        fn test_same_seed_gives_same_samples() {
            // Each HashMap gets its own random iteration order, so two engines built
            // the same way used to give different samples for the same seed
            for sampling in ["monte_carlo", "latin_hypercube"] {
                let first = samples(config(sampling, 42, 200), five_variables());
                for _ in 0..10 {
                    assert_eq!(
                        first,
                        samples(config(sampling, 42, 200), five_variables()),
                        "{sampling}"
                    );
                }
            }
        }

        #[test]
        fn test_different_seeds_give_different_samples() {
            let a = samples(config("monte_carlo", 1, 50), five_variables());
            let b = samples(config("monte_carlo", 2, 50), five_variables());
            assert_ne!(a, b);
        }

        #[test]
        fn test_latin_hypercube_puts_one_sample_in_each_stratum() {
            let n = 100;
            let result = samples(
                config("latin_hypercube", 7, n),
                vec![("u", Distribution::uniform(0.0, 1.0).unwrap())],
            );
            let mut counts = vec![0; n];
            for value in &result["u"] {
                counts[(value * precision::count(n))
                    .to_usize()
                    .expect("a draw in 0..1")] += 1;
            }
            assert!(counts.iter().all(|&count| count == 1), "{counts:?}");
        }

        #[test]
        fn test_correlations_are_applied() {
            for coefficient in [0.8, -0.5] {
                let mut config = config("monte_carlo", 11, 20_000);
                config.correlations = vec![CorrelationConfig {
                    variables: vec!["revenue".to_string(), "x.costs".to_string()],
                    coefficient,
                }];
                let result = samples(
                    config,
                    vec![
                        ("x.revenue", Distribution::normal(100.0, 10.0).unwrap()),
                        ("x.costs", Distribution::normal(50.0, 5.0).unwrap()),
                        ("x.other", Distribution::normal(0.0, 1.0).unwrap()),
                    ],
                );
                let r = pearson(&result["x.revenue"], &result["x.costs"]);
                assert!(
                    (r - coefficient).abs() < 0.02,
                    "wanted {coefficient}, got {r}"
                );
                // An uncorrelated variable stays uncorrelated
                assert!(pearson(&result["x.revenue"], &result["x.other"]).abs() < 0.03);
                // Each variable keeps its own distribution
                let mean = result["x.costs"].iter().sum::<f64>() / 20_000.0;
                assert!((mean - 50.0).abs() < 0.2, "{mean}");
            }
        }

        #[test]
        fn test_bad_correlation_variable_is_an_error() {
            for variables in [["revenue", "missing"], ["revenue", "x.revenue"]] {
                let mut config = config("monte_carlo", 1, 10);
                config.correlations = vec![CorrelationConfig {
                    variables: variables.iter().map(ToString::to_string).collect(),
                    coefficient: 0.5,
                }];
                let mut engine = MonteCarloEngine::new(config).unwrap();
                engine.add_distribution("x.revenue", Distribution::normal(1.0, 1.0).unwrap());
                assert!(engine.run().is_err(), "{variables:?}");
            }
        }

        #[test]
        fn test_inverse_cdf_matches_reference_values() {
            // Normal, PERT and lognormal references from R (qnorm, qbeta, qlnorm);
            // triangular in closed form: sqrt(6) and 10 - sqrt(14)
            let cases = [
                (
                    Distribution::normal(100.0, 15.0).unwrap(),
                    0.975,
                    129.399_459_768_101,
                ),
                (
                    Distribution::pert(10.0, 20.0, 40.0).unwrap(),
                    0.3,
                    18.200_465_492_438,
                ),
                (
                    Distribution::lognormal(100.0, 20.0).unwrap(),
                    0.9,
                    126.388_578_975_423,
                ),
                (
                    Distribution::triangular(0.0, 3.0, 10.0).unwrap(),
                    0.2,
                    6.0_f64.sqrt(),
                ),
                (
                    Distribution::triangular(0.0, 3.0, 10.0).unwrap(),
                    0.8,
                    10.0 - 14.0_f64.sqrt(),
                ),
                (Distribution::uniform(-1.0, 3.0).unwrap(), 0.25, 0.0),
            ];
            for (dist, p, expected) in cases {
                let value = dist.inverse_cdf(p);
                let kind = dist.dist_type();
                assert!((value - expected).abs() < 1e-6, "{kind} at {p}: {value}");
            }
            let discrete =
                Distribution::discrete(vec![1.0, 2.0, 3.0], vec![0.2, 0.5, 0.3]).unwrap();
            assert!((discrete.inverse_cdf(0.1) - 1.0).abs() < f64::EPSILON);
            assert!((discrete.inverse_cdf(0.6) - 2.0).abs() < f64::EPSILON);
            assert!((discrete.inverse_cdf(0.95) - 3.0).abs() < f64::EPSILON);
        }
    }
}
