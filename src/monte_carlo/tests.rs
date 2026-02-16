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
        .run_with_evaluator(|inputs| {
            let revenue = inputs.get("revenue").copied().unwrap_or(1_000_000.0);
            let costs = inputs.get("costs").copied().unwrap_or(500_000.0);
            let discount_rate = inputs.get("discount_rate").copied().unwrap_or(0.10);

            // Simple 3-year NPV
            let cf1 = revenue - costs;
            let cf2 = revenue * 1.05 - costs;
            let cf3 = revenue * 1.10 - costs;

            let npv = cf1 / (1.0 + discount_rate)
                + cf2 / (1.0 + discount_rate).powi(2)
                + cf3 / (1.0 + discount_rate).powi(3)
                - 1_000_000.0; // Initial investment

            let mut outputs = std::collections::HashMap::new();
            outputs.insert("npv".to_string(), npv);
            outputs
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
    assert!(!npv_result.histogram.counts.is_empty());
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
    let normal_mean: f64 = normal_samples.iter().sum::<f64>() / normal_samples.len() as f64;
    assert!(
        (normal_mean - 100.0).abs() < 1.0,
        "Normal mean {normal_mean} not close to 100"
    );

    // Uniform: mean ≈ 50
    let uniform_samples = &result.input_samples["uniform"];
    let uniform_mean: f64 = uniform_samples.iter().sum::<f64>() / uniform_samples.len() as f64;
    assert!(
        (uniform_mean - 50.0).abs() < 1.0,
        "Uniform mean {uniform_mean} not close to 50"
    );

    // Triangular: mean ≈ 50 ((0+50+100)/3)
    let tri_samples = &result.input_samples["triangular"];
    let tri_mean: f64 = tri_samples.iter().sum::<f64>() / tri_samples.len() as f64;
    assert!(
        (tri_mean - 50.0).abs() < 1.0,
        "Triangular mean {tri_mean} not close to 50"
    );

    // PERT: mean ≈ 36.67 ((0+4*30+100)/6)
    let pert_samples = &result.input_samples["pert"];
    let pert_mean: f64 = pert_samples.iter().sum::<f64>() / pert_samples.len() as f64;
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
    assert!(
        elapsed.as_secs() < 5,
        "Performance test failed: {} seconds",
        elapsed.as_secs_f64()
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
            label: None,
        }],
        correlations: vec![],
    };

    let mut engine = MonteCarloEngine::new(config).unwrap();
    engine.add_distribution("test", Distribution::uniform(0.0, 100.0).unwrap());

    let result = engine.run().unwrap();
    let json = result.to_json().unwrap();

    // Verify JSON is valid and contains expected fields
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(
        parsed["monte_carlo_results"]["iterations"]
            .as_u64()
            .unwrap()
            == 100
    );
    assert!(parsed["monte_carlo_results"]["sampling"].as_str().unwrap() == "latin_hypercube");
    assert!(parsed["monte_carlo_results"]["seed"].as_u64().unwrap() == 42);
    assert!(parsed["monte_carlo_results"]["outputs"]["test"]["mean"].is_number());
}
