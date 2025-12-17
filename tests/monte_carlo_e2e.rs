//! End-to-end tests for Monte Carlo simulation
//!
//! These tests validate the full Monte Carlo workflow including:
//! - All 6 distribution types
//! - Latin Hypercube vs Monte Carlo sampling
//! - Percentile calculations
//! - Probability thresholds
//! - JSON and YAML output formats
//! - Seed reproducibility
//! - Statistical properties validation

#![cfg(not(feature = "demo"))]

use royalbit_forge::monte_carlo::{Distribution, MonteCarloConfig, MonteCarloEngine};
use royalbit_forge::parser;
use serde_yaml_ng::Value;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Create a temporary YAML file with the given content
fn create_test_yaml(content: &str) -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let yaml_path = temp_dir.path().join("test_model.yaml");
    fs::write(&yaml_path, content).unwrap();
    (temp_dir, yaml_path)
}

/// Parse monte_carlo config from YAML content
fn parse_monte_carlo_config(yaml_content: &str) -> Option<MonteCarloConfig> {
    let yaml: Value = serde_yaml_ng::from_str(yaml_content).ok()?;

    if let Some(mc_value) = yaml.get("monte_carlo") {
        serde_yaml_ng::from_value(mc_value.clone()).ok()
    } else {
        None
    }
}

/// Assert that a value is within a percentage of an expected value
fn assert_within_percent(actual: f64, expected: f64, percent: f64, msg: &str) {
    let diff = (actual - expected).abs();
    let tolerance = expected.abs() * percent / 100.0;
    assert!(
        diff <= tolerance,
        "{}: expected {} ± {}%, got {} (diff: {}, tolerance: {})",
        msg,
        expected,
        percent,
        actual,
        diff,
        tolerance
    );
}

/// Assert that a value is within an absolute tolerance
fn assert_within_abs(actual: f64, expected: f64, tolerance: f64, msg: &str) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "{}: expected {} ± {}, got {} (diff: {})",
        msg,
        expected,
        tolerance,
        actual,
        diff
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// DISTRIBUTION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_normal_distribution_basic() {
    let yaml_content = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 10000
  sampling: latin_hypercube
  seed: 12345
  outputs:
    - variable: revenue
      percentiles: [10, 50, 90]

scalars:
  revenue:
    formula: "=MC.Normal(100000, 15000)"
"#;

    let (_temp_dir, yaml_path) = create_test_yaml(yaml_content);
    let model = parser::parse_model(&yaml_path).unwrap();
    let config = parse_monte_carlo_config(yaml_content).unwrap();

    // Create and run simulation
    let mut engine = MonteCarloEngine::new(config).unwrap();
    engine.parse_distributions_from_model(&model).unwrap();

    let result = engine.run().unwrap();

    // Verify output exists
    assert!(result.outputs.contains_key("revenue"));
    let revenue_output = &result.outputs["revenue"];

    // Validate statistical properties
    // Mean should be within 5% of 100,000
    assert_within_percent(
        revenue_output.statistics.mean,
        100000.0,
        5.0,
        "Normal distribution mean",
    );

    // Std dev should be within 10% of 15,000
    assert_within_percent(
        revenue_output.statistics.std_dev,
        15000.0,
        10.0,
        "Normal distribution std dev",
    );

    // Check percentiles exist
    assert!(revenue_output.statistics.percentile(10).is_some());
    assert!(revenue_output.statistics.percentile(50).is_some());
    assert!(revenue_output.statistics.percentile(90).is_some());

    // P50 (median) should be close to mean for normal distribution
    let p50 = revenue_output.statistics.percentile(50).unwrap();
    assert_within_abs(p50, 100000.0, 5000.0, "Normal distribution median");
}

#[test]
fn test_triangular_distribution_basic() {
    let yaml_content = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 10000
  sampling: latin_hypercube
  seed: 42
  outputs:
    - variable: cost
      percentiles: [25, 50, 75]

scalars:
  cost:
    formula: "=MC.Triangular(80000, 100000, 150000)"
"#;

    let (_temp_dir, yaml_path) = create_test_yaml(yaml_content);
    let model = parser::parse_model(&yaml_path).unwrap();
    let config = parse_monte_carlo_config(yaml_content).unwrap();

    let mut engine = MonteCarloEngine::new(config).unwrap();
    engine.parse_distributions_from_model(&model).unwrap();

    let result = engine.run().unwrap();

    let cost_output = &result.outputs["cost"];

    // Triangular theoretical mean = (min + mode + max) / 3 = (80000 + 100000 + 150000) / 3 = 110000
    assert_within_percent(
        cost_output.statistics.mean,
        110000.0,
        5.0,
        "Triangular distribution mean",
    );

    // All values should be within bounds
    assert!(
        cost_output.statistics.min >= 80000.0,
        "Min should be >= 80000"
    );
    assert!(
        cost_output.statistics.max <= 150000.0,
        "Max should be <= 150000"
    );

    // Check percentiles
    let p50 = cost_output.statistics.percentile(50).unwrap();
    assert!(
        (80000.0..=150000.0).contains(&p50),
        "Median should be within distribution bounds"
    );
}

#[test]
fn test_uniform_distribution_basic() {
    let yaml_content = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 10000
  sampling: monte_carlo
  seed: 999
  outputs:
    - variable: discount
      percentiles: [10, 50, 90]

scalars:
  discount:
    formula: "=MC.Uniform(0.1, 0.3)"
"#;

    let (_temp_dir, yaml_path) = create_test_yaml(yaml_content);
    let model = parser::parse_model(&yaml_path).unwrap();
    let config = parse_monte_carlo_config(yaml_content).unwrap();

    let mut engine = MonteCarloEngine::new(config).unwrap();
    engine.parse_distributions_from_model(&model).unwrap();

    let result = engine.run().unwrap();

    let discount_output = &result.outputs["discount"];

    // Uniform theoretical mean = (min + max) / 2 = (0.1 + 0.3) / 2 = 0.2
    assert_within_percent(
        discount_output.statistics.mean,
        0.2,
        5.0,
        "Uniform distribution mean",
    );

    // All values should be within bounds
    assert!(
        discount_output.statistics.min >= 0.1,
        "Min should be >= 0.1"
    );
    assert!(discount_output.statistics.max < 0.3, "Max should be < 0.3");

    // For uniform, P50 should be very close to mean
    let p50 = discount_output.statistics.percentile(50).unwrap();
    assert_within_abs(p50, 0.2, 0.02, "Uniform distribution median");
}

#[test]
fn test_pert_distribution_basic() {
    let yaml_content = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 10000
  sampling: latin_hypercube
  seed: 777
  outputs:
    - variable: duration
      percentiles: [10, 50, 90]

scalars:
  duration:
    formula: "=MC.PERT(10, 15, 30)"
"#;

    let (_temp_dir, yaml_path) = create_test_yaml(yaml_content);
    let model = parser::parse_model(&yaml_path).unwrap();
    let config = parse_monte_carlo_config(yaml_content).unwrap();

    let mut engine = MonteCarloEngine::new(config).unwrap();
    engine.parse_distributions_from_model(&model).unwrap();

    let result = engine.run().unwrap();

    let duration_output = &result.outputs["duration"];

    // PERT theoretical mean = (min + 4*mode + max) / 6 = (10 + 4*15 + 30) / 6 = 100/6 ≈ 16.67
    assert_within_percent(
        duration_output.statistics.mean,
        16.67,
        5.0,
        "PERT distribution mean",
    );

    // All values should be within bounds
    assert!(
        duration_output.statistics.min >= 10.0,
        "Min should be >= 10"
    );
    assert!(
        duration_output.statistics.max <= 30.0,
        "Max should be <= 30"
    );
}

#[test]
fn test_lognormal_distribution_basic() {
    let yaml_content = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 10000
  sampling: latin_hypercube
  seed: 555
  outputs:
    - variable: price
      percentiles: [10, 50, 90]

scalars:
  price:
    formula: "=MC.Lognormal(50, 10)"
"#;

    let (_temp_dir, yaml_path) = create_test_yaml(yaml_content);
    let model = parser::parse_model(&yaml_path).unwrap();
    let config = parse_monte_carlo_config(yaml_content).unwrap();

    let mut engine = MonteCarloEngine::new(config).unwrap();
    engine.parse_distributions_from_model(&model).unwrap();

    let result = engine.run().unwrap();

    let price_output = &result.outputs["price"];

    // Lognormal mean should be close to specified mean
    assert_within_percent(
        price_output.statistics.mean,
        50.0,
        10.0,
        "Lognormal distribution mean",
    );

    // All values should be non-negative (lognormal property)
    assert!(price_output.statistics.min >= 0.0, "Min should be >= 0");
    assert!(price_output.samples.iter().all(|&x| x >= 0.0));
}

#[test]
fn test_discrete_distribution_basic() {
    // For Discrete distribution, we need to use the engine directly
    // since the parser doesn't support MC.Discrete yet
    let config = MonteCarloConfig::new()
        .enabled()
        .with_iterations(10000)
        .with_sampling("latin_hypercube")
        .with_seed(333);

    let mut engine = MonteCarloEngine::new(config).unwrap();

    // Discrete: 50% chance of 100, 30% chance of 200, 20% chance of 300
    let dist = Distribution::discrete(vec![100.0, 200.0, 300.0], vec![0.5, 0.3, 0.2]).unwrap();

    engine.add_distribution("outcome", dist);

    // Add output config manually
    engine = MonteCarloEngine::new(MonteCarloConfig {
        enabled: true,
        iterations: 10000,
        sampling: "latin_hypercube".to_string(),
        seed: Some(333),
        outputs: vec![royalbit_forge::monte_carlo::config::OutputConfig {
            variable: "outcome".to_string(),
            percentiles: vec![10, 50, 90],
            threshold: None,
            label: None,
        }],
        correlations: vec![],
    })
    .unwrap();

    let dist = Distribution::discrete(vec![100.0, 200.0, 300.0], vec![0.5, 0.3, 0.2]).unwrap();

    engine.add_distribution("outcome", dist);

    let result = engine.run().unwrap();
    let outcome_output = &result.outputs["outcome"];

    // Expected mean = 100*0.5 + 200*0.3 + 300*0.2 = 50 + 60 + 60 = 170
    assert_within_percent(
        outcome_output.statistics.mean,
        170.0,
        5.0,
        "Discrete distribution mean",
    );

    // Values should only be 100, 200, or 300
    for &sample in &outcome_output.samples {
        assert!(
            (sample - 100.0).abs() < 0.1
                || (sample - 200.0).abs() < 0.1
                || (sample - 300.0).abs() < 0.1,
            "Sample {} should be one of [100, 200, 300]",
            sample
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SAMPLING METHOD TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_latin_hypercube_vs_monte_carlo_convergence() {
    let yaml_lhs = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 1000
  sampling: latin_hypercube
  seed: 12345
  outputs:
    - variable: test_value

test_value:
  formula: "=MC.Normal(100, 15)"
"#;

    let yaml_mc = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 1000
  sampling: monte_carlo
  seed: 12345
  outputs:
    - variable: test_value

test_value:
  formula: "=MC.Normal(100, 15)"
"#;

    // Run LHS
    let (_temp_dir1, yaml_path1) = create_test_yaml(yaml_lhs);
    let model1 = parser::parse_model(&yaml_path1).unwrap();
    let config1 = parse_monte_carlo_config(yaml_lhs).unwrap();
    let mut engine1 = MonteCarloEngine::new(config1).unwrap();
    engine1.parse_distributions_from_model(&model1).unwrap();
    let result1 = engine1.run().unwrap();

    // Run Monte Carlo
    let (_temp_dir2, yaml_path2) = create_test_yaml(yaml_mc);
    let model2 = parser::parse_model(&yaml_path2).unwrap();
    let config2 = parse_monte_carlo_config(yaml_mc).unwrap();
    let mut engine2 = MonteCarloEngine::new(config2).unwrap();
    engine2.parse_distributions_from_model(&model2).unwrap();
    let result2 = engine2.run().unwrap();

    // Both should produce reasonable results
    let lhs_mean = result1.outputs["test_value"].statistics.mean;
    let mc_mean = result2.outputs["test_value"].statistics.mean;

    assert_within_percent(lhs_mean, 100.0, 5.0, "LHS mean");
    assert_within_percent(mc_mean, 100.0, 5.0, "MC mean");

    // LHS typically has better convergence (lower variance in estimates)
    // but with different seeds this isn't guaranteed, so we just verify both work
}

#[test]
fn test_sampling_method_validation() {
    let yaml_content = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 100
  sampling: invalid_method
  outputs:
    - variable: value

value:
  formula: "=MC.Normal(100, 15)"
"#;

    let config = parse_monte_carlo_config(yaml_content).unwrap();

    // Should fail validation
    let result = MonteCarloEngine::new(config);
    assert!(result.is_err(), "Invalid sampling method should fail");
}

// ═══════════════════════════════════════════════════════════════════════════
// PERCENTILE TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_percentile_calculations() {
    let yaml_content = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 10000
  sampling: latin_hypercube
  seed: 1111
  outputs:
    - variable: metric
      percentiles: [5, 10, 25, 50, 75, 90, 95]

scalars:
  metric:
    formula: "=MC.Normal(1000, 100)"
"#;

    let (_temp_dir, yaml_path) = create_test_yaml(yaml_content);
    let model = parser::parse_model(&yaml_path).unwrap();
    let config = parse_monte_carlo_config(yaml_content).unwrap();

    let mut engine = MonteCarloEngine::new(config).unwrap();
    engine.parse_distributions_from_model(&model).unwrap();

    let result = engine.run().unwrap();
    let metric_output = &result.outputs["metric"];

    // Verify all percentiles exist
    for p in [5, 10, 25, 50, 75, 90, 95] {
        assert!(
            metric_output.statistics.percentile(p).is_some(),
            "Percentile P{} should exist",
            p
        );
    }

    // Percentiles should be monotonically increasing
    let p5 = metric_output.statistics.percentile(5).unwrap();
    let p25 = metric_output.statistics.percentile(25).unwrap();
    let p50 = metric_output.statistics.percentile(50).unwrap();
    let p75 = metric_output.statistics.percentile(75).unwrap();
    let p95 = metric_output.statistics.percentile(95).unwrap();

    assert!(p5 < p25, "P5 < P25");
    assert!(p25 < p50, "P25 < P50");
    assert!(p50 < p75, "P50 < P75");
    assert!(p75 < p95, "P75 < P95");

    // P50 should be close to mean for normal distribution
    assert_within_abs(p50, 1000.0, 50.0, "P50 close to mean");

    // For normal distribution, P5 and P95 should be roughly symmetric around mean
    let lower_spread = 1000.0 - p5;
    let upper_spread = p95 - 1000.0;
    assert_within_percent(
        lower_spread,
        upper_spread,
        20.0,
        "Normal distribution symmetry",
    );
}

#[test]
fn test_custom_percentiles() {
    let yaml_content = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 5000
  sampling: latin_hypercube
  seed: 2222
  outputs:
    - variable: value
      percentiles: [1, 5, 95, 99]

value:
  formula: "=MC.Uniform(0, 100)"
"#;

    let (_temp_dir, yaml_path) = create_test_yaml(yaml_content);
    let model = parser::parse_model(&yaml_path).unwrap();
    let config = parse_monte_carlo_config(yaml_content).unwrap();

    let mut engine = MonteCarloEngine::new(config).unwrap();
    engine.parse_distributions_from_model(&model).unwrap();

    let result = engine.run().unwrap();
    let value_output = &result.outputs["value"];

    // For uniform [0, 100]:
    // P5 should be close to 5
    // P10 should be close to 10
    // P90 should be close to 90
    // P95 should be close to 95
    let p5 = value_output.statistics.percentile(5).unwrap();
    let p10 = value_output.statistics.percentile(10).unwrap();
    let p90 = value_output.statistics.percentile(90).unwrap();
    let p95 = value_output.statistics.percentile(95).unwrap();

    assert_within_abs(p5, 5.0, 3.0, "P5 for uniform distribution");
    assert_within_abs(p10, 10.0, 3.0, "P10 for uniform distribution");
    assert_within_abs(p90, 90.0, 3.0, "P90 for uniform distribution");
    assert_within_abs(p95, 95.0, 3.0, "P95 for uniform distribution");
}

// ═══════════════════════════════════════════════════════════════════════════
// THRESHOLD PROBABILITY TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_threshold_greater_than() {
    let yaml_content = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 10000
  sampling: latin_hypercube
  seed: 3333
  outputs:
    - variable: profit
      percentiles: [50]
      threshold: "> 0"

scalars:
  profit:
    formula: "=MC.Normal(50000, 20000)"
"#;

    let (_temp_dir, yaml_path) = create_test_yaml(yaml_content);
    let model = parser::parse_model(&yaml_path).unwrap();
    let config = parse_monte_carlo_config(yaml_content).unwrap();

    let mut engine = MonteCarloEngine::new(config).unwrap();
    engine.parse_distributions_from_model(&model).unwrap();

    let result = engine.run().unwrap();
    let profit_output = &result.outputs["profit"];

    // Should have threshold probability
    assert!(
        profit_output.threshold_probabilities.contains_key("> 0"),
        "Should have threshold '> 0'"
    );

    let prob_positive = profit_output.threshold_probabilities["> 0"];

    // For Normal(50000, 20000), P(X > 0) should be very high (close to 1.0)
    // Mean is 2.5 standard deviations above 0
    assert!(
        prob_positive > 0.98,
        "Probability of profit > 0 should be > 98%, got {}",
        prob_positive
    );
}

#[test]
fn test_threshold_less_than() {
    let yaml_content = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 10000
  sampling: latin_hypercube
  seed: 4444
  outputs:
    - variable: cost
      percentiles: [50]
      threshold: "< 150000"

scalars:
  cost:
    formula: "=MC.Normal(100000, 15000)"
"#;

    let (_temp_dir, yaml_path) = create_test_yaml(yaml_content);
    let model = parser::parse_model(&yaml_path).unwrap();
    let config = parse_monte_carlo_config(yaml_content).unwrap();

    let mut engine = MonteCarloEngine::new(config).unwrap();
    engine.parse_distributions_from_model(&model).unwrap();

    let result = engine.run().unwrap();
    let cost_output = &result.outputs["cost"];

    let prob_under_150k = cost_output.threshold_probabilities["< 150000"];

    // For Normal(100000, 15000), P(X < 150000) should be very high
    // 150000 is (150000-100000)/15000 = 3.33 standard deviations above mean
    assert!(
        prob_under_150k > 0.99,
        "Probability of cost < 150000 should be > 99%, got {}",
        prob_under_150k
    );
}

#[test]
fn test_multiple_thresholds() {
    // Test multiple thresholds using run_with_evaluator
    let config = MonteCarloConfig {
        enabled: true,
        iterations: 5000,
        sampling: "latin_hypercube".to_string(),
        seed: Some(5555),
        outputs: vec![
            royalbit_forge::monte_carlo::config::OutputConfig {
                variable: "value".to_string(),
                percentiles: vec![50],
                threshold: Some("> 100".to_string()),
                label: None,
            },
            royalbit_forge::monte_carlo::config::OutputConfig {
                variable: "value".to_string(),
                percentiles: vec![50],
                threshold: Some("< 200".to_string()),
                label: None,
            },
        ],
        correlations: vec![],
    };

    let mut engine = MonteCarloEngine::new(config).unwrap();
    let dist = Distribution::normal(150.0, 30.0).unwrap();
    engine.add_distribution("value", dist);

    let result = engine.run().unwrap();
    let value_output = &result.outputs["value"];

    // For Normal(150, 30):
    // P(X > 100) should be high (100 is 1.67 std devs below mean)
    // P(X < 200) should be high (200 is 1.67 std devs above mean)
    if let Some(&prob_gt_100) = value_output.threshold_probabilities.get("> 100") {
        assert!(
            prob_gt_100 > 0.90,
            "P(X > 100) should be > 90%, got {}",
            prob_gt_100
        );
    }

    if let Some(&prob_lt_200) = value_output.threshold_probabilities.get("< 200") {
        assert!(
            prob_lt_200 > 0.90,
            "P(X < 200) should be > 90%, got {}",
            prob_lt_200
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// OUTPUT FORMAT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_yaml_output_format() {
    let yaml_content = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 1000
  sampling: latin_hypercube
  seed: 6666
  outputs:
    - variable: metric
      percentiles: [10, 50, 90]
      threshold: "> 1000"

scalars:
  metric:
    formula: "=MC.Normal(1500, 200)"
"#;

    let (_temp_dir, yaml_path) = create_test_yaml(yaml_content);
    let model = parser::parse_model(&yaml_path).unwrap();
    let config = parse_monte_carlo_config(yaml_content).unwrap();

    let mut engine = MonteCarloEngine::new(config).unwrap();
    engine.parse_distributions_from_model(&model).unwrap();

    let result = engine.run().unwrap();
    let yaml_output = result.to_yaml();

    // Verify YAML structure
    assert!(yaml_output.contains("monte_carlo_results:"));
    assert!(yaml_output.contains("iterations: 1000"));
    assert!(yaml_output.contains("sampling: latin_hypercube"));
    assert!(yaml_output.contains("seed: 6666"));
    assert!(yaml_output.contains("metric:"));
    assert!(yaml_output.contains("mean:"));
    assert!(yaml_output.contains("median:"));
    assert!(yaml_output.contains("std_dev:"));
    assert!(yaml_output.contains("percentiles:"));
    assert!(yaml_output.contains("p10:"));
    assert!(yaml_output.contains("p50:"));
    assert!(yaml_output.contains("p90:"));
    assert!(yaml_output.contains("thresholds:"));
    assert!(yaml_output.contains("\"> 1000\":"));
}

#[test]
fn test_json_output_format() {
    let yaml_content = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 1000
  sampling: monte_carlo
  seed: 7777
  outputs:
    - variable: value
      percentiles: [25, 50, 75]

value:
  formula: "=MC.Uniform(50, 150)"
"#;

    let (_temp_dir, yaml_path) = create_test_yaml(yaml_content);
    let model = parser::parse_model(&yaml_path).unwrap();
    let config = parse_monte_carlo_config(yaml_content).unwrap();

    let mut engine = MonteCarloEngine::new(config).unwrap();
    engine.parse_distributions_from_model(&model).unwrap();

    let result = engine.run().unwrap();
    let json_output = result.to_json().unwrap();

    // Verify JSON structure
    assert!(json_output.contains("\"monte_carlo_results\""));
    assert!(json_output.contains("\"iterations\": 1000"));
    assert!(json_output.contains("\"sampling\": \"monte_carlo\""));
    assert!(json_output.contains("\"seed\": 7777"));
    assert!(json_output.contains("\"value\""));
    assert!(json_output.contains("\"mean\""));
    assert!(json_output.contains("\"median\""));
    assert!(json_output.contains("\"std_dev\""));
    assert!(json_output.contains("\"percentiles\""));
    assert!(json_output.contains("\"p25\""));
    assert!(json_output.contains("\"p50\""));
    assert!(json_output.contains("\"p75\""));

    // Verify valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json_output).unwrap();
    assert!(parsed["monte_carlo_results"]["iterations"].is_number());
}

// ═══════════════════════════════════════════════════════════════════════════
// SEED REPRODUCIBILITY TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_seed_reproducibility_identical_results() {
    let yaml_content = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 1000
  sampling: latin_hypercube
  seed: 99999
  outputs:
    - variable: revenue
      percentiles: [10, 50, 90]

scalars:
  revenue:
    formula: "=MC.Normal(100000, 15000)"
"#;

    // Run 1
    let (_temp_dir1, yaml_path1) = create_test_yaml(yaml_content);
    let model1 = parser::parse_model(&yaml_path1).unwrap();
    let config1 = parse_monte_carlo_config(yaml_content).unwrap();
    let mut engine1 = MonteCarloEngine::new(config1).unwrap();
    engine1.parse_distributions_from_model(&model1).unwrap();
    let result1 = engine1.run().unwrap();

    // Run 2
    let (_temp_dir2, yaml_path2) = create_test_yaml(yaml_content);
    let model2 = parser::parse_model(&yaml_path2).unwrap();
    let config2 = parse_monte_carlo_config(yaml_content).unwrap();
    let mut engine2 = MonteCarloEngine::new(config2).unwrap();
    engine2.parse_distributions_from_model(&model2).unwrap();
    let result2 = engine2.run().unwrap();

    // Results should be identical
    let samples1 = result1
        .input_samples
        .get("revenue")
        .or_else(|| result1.input_samples.get("scalars.revenue"))
        .expect("Should have revenue samples");
    let samples2 = result2
        .input_samples
        .get("revenue")
        .or_else(|| result2.input_samples.get("scalars.revenue"))
        .expect("Should have revenue samples");

    assert_eq!(
        samples1, samples2,
        "Same seed should produce identical samples"
    );

    let stats1 = &result1.outputs["revenue"].statistics;
    let stats2 = &result2.outputs["revenue"].statistics;

    assert_eq!(stats1.mean, stats2.mean, "Means should be identical");
    assert_eq!(
        stats1.std_dev, stats2.std_dev,
        "Std devs should be identical"
    );
    assert_eq!(
        stats1.percentile(50),
        stats2.percentile(50),
        "Medians should be identical"
    );
}

#[test]
fn test_different_seeds_different_results() {
    let yaml_seed1 = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 1000
  sampling: latin_hypercube
  seed: 11111
  outputs:
    - variable: value

value:
  formula: "=MC.Normal(100, 15)"
"#;

    let yaml_seed2 = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 1000
  sampling: latin_hypercube
  seed: 22222
  outputs:
    - variable: value

value:
  formula: "=MC.Normal(100, 15)"
"#;

    // Run with seed 1
    let (_temp_dir1, yaml_path1) = create_test_yaml(yaml_seed1);
    let model1 = parser::parse_model(&yaml_path1).unwrap();
    let config1 = parse_monte_carlo_config(yaml_seed1).unwrap();
    let mut engine1 = MonteCarloEngine::new(config1).unwrap();
    engine1.parse_distributions_from_model(&model1).unwrap();
    let result1 = engine1.run().unwrap();

    // Run with seed 2
    let (_temp_dir2, yaml_path2) = create_test_yaml(yaml_seed2);
    let model2 = parser::parse_model(&yaml_path2).unwrap();
    let config2 = parse_monte_carlo_config(yaml_seed2).unwrap();
    let mut engine2 = MonteCarloEngine::new(config2).unwrap();
    engine2.parse_distributions_from_model(&model2).unwrap();
    let result2 = engine2.run().unwrap();

    // Samples should be different
    let samples1 = &result1.input_samples["value"];
    let samples2 = &result2.input_samples["value"];

    assert_ne!(
        samples1, samples2,
        "Different seeds should produce different samples"
    );

    // But statistics should still be reasonable (both close to theoretical)
    let stats1 = &result1.outputs["value"].statistics;
    let stats2 = &result2.outputs["value"].statistics;

    assert_within_percent(stats1.mean, 100.0, 5.0, "Seed 1 mean");
    assert_within_percent(stats2.mean, 100.0, 5.0, "Seed 2 mean");
}

// ═══════════════════════════════════════════════════════════════════════════
// MULTIPLE DISTRIBUTIONS TEST
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_multiple_distributions_combined() {
    let yaml_content = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 5000
  sampling: latin_hypercube
  seed: 8888
  outputs:
    - variable: revenue
      percentiles: [10, 50, 90]
    - variable: cost
      percentiles: [10, 50, 90]
    - variable: margin
      percentiles: [10, 50, 90]

scalars:
  revenue:
    formula: "=MC.Normal(500000, 50000)"
  cost:
    formula: "=MC.Triangular(200000, 250000, 350000)"
  margin:
    formula: "=MC.Uniform(0.2, 0.4)"
"#;

    let (_temp_dir, yaml_path) = create_test_yaml(yaml_content);
    let model = parser::parse_model(&yaml_path).unwrap();
    let config = parse_monte_carlo_config(yaml_content).unwrap();

    let mut engine = MonteCarloEngine::new(config).unwrap();
    engine.parse_distributions_from_model(&model).unwrap();

    let result = engine.run().unwrap();

    // Verify all three outputs exist
    assert!(result.outputs.contains_key("revenue"));
    assert!(result.outputs.contains_key("cost"));
    assert!(result.outputs.contains_key("margin"));

    // Verify each distribution has reasonable statistics
    let revenue_output = &result.outputs["revenue"];
    assert_within_percent(
        revenue_output.statistics.mean,
        500000.0,
        5.0,
        "Revenue mean",
    );

    let cost_output = &result.outputs["cost"];
    let cost_expected_mean = (200000.0 + 250000.0 + 350000.0) / 3.0;
    assert_within_percent(
        cost_output.statistics.mean,
        cost_expected_mean,
        5.0,
        "Cost mean",
    );

    let margin_output = &result.outputs["margin"];
    assert_within_percent(margin_output.statistics.mean, 0.3, 5.0, "Margin mean");
}

// ═══════════════════════════════════════════════════════════════════════════
// EVALUATOR FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_run_with_evaluator_profit_calculation() {
    let config = MonteCarloConfig {
        enabled: true,
        iterations: 5000,
        sampling: "latin_hypercube".to_string(),
        seed: Some(9999),
        outputs: vec![royalbit_forge::monte_carlo::config::OutputConfig {
            variable: "profit".to_string(),
            percentiles: vec![10, 50, 90],
            threshold: Some("> 0".to_string()),
            label: None,
        }],
        correlations: vec![],
    };

    let mut engine = MonteCarloEngine::new(config).unwrap();

    // Add input distributions
    engine.add_distribution("revenue", Distribution::normal(100000.0, 10000.0).unwrap());
    engine.add_distribution("costs", Distribution::normal(60000.0, 5000.0).unwrap());

    // Run with evaluator: profit = revenue - costs
    let result = engine
        .run_with_evaluator(|inputs| {
            let revenue = inputs.get("revenue").copied().unwrap_or(0.0);
            let costs = inputs.get("costs").copied().unwrap_or(0.0);
            let mut outputs = HashMap::new();
            outputs.insert("profit".to_string(), revenue - costs);
            outputs
        })
        .unwrap();

    let profit_output = &result.outputs["profit"];

    // Expected profit mean ≈ 100000 - 60000 = 40000
    assert_within_percent(profit_output.statistics.mean, 40000.0, 10.0, "Profit mean");

    // Probability of profit > 0 should be very high
    let prob_positive = profit_output.threshold_probabilities["> 0"];
    assert!(
        prob_positive > 0.99,
        "Probability of positive profit should be > 99%, got {}",
        prob_positive
    );
}

#[test]
fn test_run_with_evaluator_complex_formula() {
    let config = MonteCarloConfig {
        enabled: true,
        iterations: 3000,
        sampling: "latin_hypercube".to_string(),
        seed: Some(10101),
        outputs: vec![royalbit_forge::monte_carlo::config::OutputConfig {
            variable: "npv".to_string(),
            percentiles: vec![5, 50, 95],
            threshold: Some("> 10000".to_string()),
            label: None,
        }],
        correlations: vec![],
    };

    let mut engine = MonteCarloEngine::new(config).unwrap();

    engine.add_distribution(
        "initial_investment",
        Distribution::uniform(80000.0, 100000.0).unwrap(),
    );
    engine.add_distribution(
        "annual_cashflow",
        Distribution::normal(30000.0, 5000.0).unwrap(),
    );
    engine.add_distribution("discount_rate", Distribution::uniform(0.08, 0.12).unwrap());

    // NPV = -initial_investment + sum of discounted cashflows (simplified 3-year)
    let result = engine
        .run_with_evaluator(|inputs| {
            let investment = inputs.get("initial_investment").copied().unwrap_or(0.0);
            let cashflow = inputs.get("annual_cashflow").copied().unwrap_or(0.0);
            let rate = inputs.get("discount_rate").copied().unwrap_or(0.1);

            let mut npv = -investment;
            for year in 1..=3 {
                npv += cashflow / (1.0 + rate).powi(year);
            }

            let mut outputs = HashMap::new();
            outputs.insert("npv".to_string(), npv);
            outputs
        })
        .unwrap();

    let npv_output = &result.outputs["npv"];

    // NPV should be reasonable (rough estimate: -90000 + 3*30000/(1.1) ≈ -8k to 0)
    // This is a rough check, actual value depends on distributions
    assert!(
        npv_output.statistics.mean > -20000.0 && npv_output.statistics.mean < 20000.0,
        "NPV mean should be reasonable, got {}",
        npv_output.statistics.mean
    );

    // Probability of NPV > 10000 should be calculable
    assert!(npv_output.threshold_probabilities.contains_key("> 10000"));
}

// ═══════════════════════════════════════════════════════════════════════════
// VALIDATION AND ERROR TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_invalid_distribution_parameters() {
    // Normal with negative std dev
    assert!(Distribution::normal(100.0, -15.0).is_err());

    // Triangular with min >= max
    assert!(Distribution::triangular(100.0, 90.0, 80.0).is_err());

    // Triangular with mode outside bounds
    assert!(Distribution::triangular(0.0, 150.0, 100.0).is_err());

    // Uniform with min >= max
    assert!(Distribution::uniform(10.0, 5.0).is_err());

    // PERT with invalid bounds
    assert!(Distribution::pert(100.0, 50.0, 30.0).is_err());

    // Lognormal with negative mean
    assert!(Distribution::lognormal(-50.0, 10.0).is_err());

    // Discrete with mismatched lengths
    assert!(Distribution::discrete(vec![1.0, 2.0], vec![0.5]).is_err());

    // Discrete with probabilities not summing to 1
    assert!(Distribution::discrete(vec![1.0, 2.0], vec![0.3, 0.5]).is_err());
}

#[test]
fn test_config_validation_errors() {
    // Zero iterations
    let mut config = MonteCarloConfig::new().enabled().with_iterations(0);
    assert!(config.validate().is_err());

    // Too many iterations
    config = MonteCarloConfig::new().enabled().with_iterations(2_000_000);
    assert!(config.validate().is_err());

    // Invalid sampling method
    config = MonteCarloConfig::new()
        .enabled()
        .with_sampling("invalid_method");
    assert!(config.validate().is_err());
}

// ═══════════════════════════════════════════════════════════════════════════
// ITERATION COUNT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_different_iteration_counts() {
    for iterations in [100, 500, 1000, 5000] {
        let config = MonteCarloConfig::new()
            .enabled()
            .with_iterations(iterations)
            .with_sampling("latin_hypercube")
            .with_seed(42);

        let mut full_config = config.clone();
        full_config.outputs = vec![royalbit_forge::monte_carlo::config::OutputConfig {
            variable: "value".to_string(),
            percentiles: vec![50],
            threshold: None,
            label: None,
        }];

        let mut engine = MonteCarloEngine::new(full_config).unwrap();
        engine.add_distribution("value", Distribution::normal(100.0, 15.0).unwrap());

        let result = engine.run().unwrap();

        assert_eq!(
            result.iterations_completed, iterations,
            "Should complete {} iterations",
            iterations
        );
        assert_eq!(
            result.input_samples["value"].len(),
            iterations,
            "Should have {} samples",
            iterations
        );

        // All iteration counts should produce reasonable results
        let value_output = &result.outputs["value"];
        assert_within_percent(value_output.statistics.mean, 100.0, 10.0, "Mean");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HISTOGRAM TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_histogram_generation() {
    let yaml_content = r#"
_forge_version: "5.0.0"

monte_carlo:
  enabled: true
  iterations: 1000
  sampling: latin_hypercube
  seed: 12121
  outputs:
    - variable: value

value:
  formula: "=MC.Normal(100, 15)"
"#;

    let (_temp_dir, yaml_path) = create_test_yaml(yaml_content);
    let model = parser::parse_model(&yaml_path).unwrap();
    let config = parse_monte_carlo_config(yaml_content).unwrap();

    let mut engine = MonteCarloEngine::new(config).unwrap();
    engine.parse_distributions_from_model(&model).unwrap();

    let result = engine.run().unwrap();
    let value_output = &result.outputs["value"];

    // Verify histogram exists and has correct structure
    let hist = &value_output.histogram;
    assert_eq!(hist.counts.len(), 50, "Should have 50 bins");
    assert_eq!(hist.frequencies.len(), 50, "Should have 50 frequencies");
    assert_eq!(hist.bin_edges.len(), 51, "Should have 51 bin edges");

    // Total frequency should sum to 1.0
    let freq_sum: f64 = hist.frequencies.iter().sum();
    assert_within_abs(freq_sum, 1.0, 0.001, "Frequencies should sum to 1");

    // Total count should equal iterations
    let count_sum: usize = hist.counts.iter().sum();
    assert_eq!(count_sum, 1000, "Counts should sum to iterations");
}
