//! Bootstrap Integration Tests

// Financial math: exact float comparison validated against Excel/Gnumeric/R
#![allow(clippy::float_cmp)]

use super::*;

/// Full workflow test with historical returns data
#[test]
fn test_financial_returns_bootstrap() {
    // Simulated historical monthly returns
    let returns = vec![
        0.05, -0.02, 0.08, 0.03, -0.05, 0.12, 0.01, -0.01, 0.06, 0.04, 0.02, -0.03, 0.07, 0.04,
        -0.02, 0.09, 0.00, -0.04, 0.05, 0.03,
    ];

    let config = BootstrapConfig::new()
        .with_data(returns)
        .with_iterations(10000)
        .with_seed(12345)
        .with_confidence_levels(vec![0.90, 0.95, 0.99]);

    let mut engine = BootstrapEngine::new(config).unwrap();
    let result = engine.analyze().unwrap();

    // Original mean return
    let expected_mean: f64 = vec![
        0.05, -0.02, 0.08, 0.03, -0.05, 0.12, 0.01, -0.01, 0.06, 0.04, 0.02, -0.03, 0.07, 0.04,
        -0.02, 0.09, 0.00, -0.04, 0.05, 0.03,
    ]
    .iter()
    .sum::<f64>()
        / 20.0;

    assert!(
        (result.original_estimate - expected_mean).abs() < 0.001,
        "Original estimate should match calculated mean"
    );

    // Should have 3 confidence intervals
    assert_eq!(result.confidence_intervals.len(), 3);

    // Intervals should be properly ordered by width
    let widths: Vec<f64> = result
        .confidence_intervals
        .iter()
        .map(super::engine::ConfidenceInterval::width)
        .collect();
    for i in 0..widths.len() - 1 {
        // Higher confidence should mean wider interval
        let ci_current = &result.confidence_intervals[i];
        let ci_next = &result.confidence_intervals[i + 1];
        if ci_current.level < ci_next.level {
            assert!(
                widths[i] <= widths[i + 1],
                "Higher confidence should mean wider interval"
            );
        }
    }
}

/// Test standard deviation bootstrap
#[test]
fn test_std_bootstrap() {
    let data = vec![10.0, 12.0, 23.0, 23.0, 16.0, 23.0, 21.0, 16.0];

    let config = BootstrapConfig::new()
        .with_data(data.clone())
        .with_statistic(BootstrapStatistic::Std)
        .with_iterations(5000)
        .with_seed(42);

    let mut engine = BootstrapEngine::new(config).unwrap();
    let result = engine.analyze().unwrap();

    // Calculate expected std
    let mean = data.iter().sum::<f64>() / data.len() as f64;
    let var = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (data.len() - 1) as f64;
    let expected_std = var.sqrt();

    assert!(
        (result.original_estimate - expected_std).abs() < 0.01,
        "Original std should match"
    );
}

/// Test variance bootstrap
#[test]
fn test_variance_bootstrap() {
    let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];

    let config = BootstrapConfig::new()
        .with_data(data.clone())
        .with_statistic(BootstrapStatistic::Var)
        .with_iterations(5000)
        .with_seed(42);

    let mut engine = BootstrapEngine::new(config).unwrap();
    let result = engine.analyze().unwrap();

    // Calculate expected variance
    let mean = data.iter().sum::<f64>() / data.len() as f64;
    let expected_var =
        data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (data.len() - 1) as f64;

    assert!(
        (result.original_estimate - expected_var).abs() < 0.01,
        "Original variance should match"
    );
}

/// Test percentile bootstrap
#[test]
fn test_percentile_bootstrap() {
    let data: Vec<f64> = (1..=100).map(f64::from).collect();

    let config = BootstrapConfig::new()
        .with_data(data)
        .with_percentile(75.0)
        .with_iterations(5000)
        .with_seed(42);

    let mut engine = BootstrapEngine::new(config).unwrap();
    let result = engine.analyze().unwrap();

    // 75th percentile of 1-100 should be around 75
    assert!(
        (result.original_estimate - 75.0).abs() < 2.0,
        "75th percentile should be around 75"
    );
}

/// Test bias estimation
#[test]
fn test_bias_estimation() {
    // Symmetric data should have low bias
    let data: Vec<f64> = vec![-5.0, -3.0, -1.0, 0.0, 1.0, 3.0, 5.0];

    let config = BootstrapConfig::new()
        .with_data(data)
        .with_iterations(10000)
        .with_seed(42);

    let mut engine = BootstrapEngine::new(config).unwrap();
    let result = engine.analyze().unwrap();

    // Bias should be small for symmetric data
    assert!(
        result.bias.abs() < 0.5,
        "Bias should be small for symmetric data: {}",
        result.bias
    );
}

/// Test with skewed data
#[test]
fn test_skewed_data() {
    // Right-skewed data (like income distribution)
    let data = vec![
        20_000.0, 25_000.0, 30_000.0, 32_000.0, 35_000.0, 40_000.0, 45_000.0, 50_000.0, 80_000.0,
        150_000.0,
    ];

    let config = BootstrapConfig::new()
        .with_data(data)
        .with_iterations(10000)
        .with_seed(42)
        .with_confidence_levels(vec![0.95]);

    let mut engine = BootstrapEngine::new(config).unwrap();
    let result = engine.analyze().unwrap();

    let ci = &result.confidence_intervals[0];

    // CI should include the original estimate
    assert!(
        ci.lower <= result.original_estimate && result.original_estimate <= ci.upper,
        "CI should contain original estimate"
    );
}

/// Test min/max statistics
#[test]
fn test_min_max_bootstrap() {
    let data = vec![1.0, 5.0, 3.0, 9.0, 2.0, 8.0, 4.0, 7.0, 6.0, 10.0];

    // Test min
    let config_min = BootstrapConfig::new()
        .with_data(data.clone())
        .with_statistic(BootstrapStatistic::Min)
        .with_iterations(1000)
        .with_seed(42);

    let mut engine_min = BootstrapEngine::new(config_min).unwrap();
    let result_min = engine_min.analyze().unwrap();
    assert_eq!(result_min.original_estimate, 1.0);

    // Test max
    let config_max = BootstrapConfig::new()
        .with_data(data)
        .with_statistic(BootstrapStatistic::Max)
        .with_iterations(1000)
        .with_seed(42);

    let mut engine_max = BootstrapEngine::new(config_max).unwrap();
    let result_max = engine_max.analyze().unwrap();
    assert_eq!(result_max.original_estimate, 10.0);
}

/// Test bias-corrected estimate
#[test]
fn test_bias_corrected() {
    let data: Vec<f64> = (1..=50).map(f64::from).collect();

    let config = BootstrapConfig::new()
        .with_data(data)
        .with_iterations(5000)
        .with_seed(42);

    let mut engine = BootstrapEngine::new(config).unwrap();
    let result = engine.analyze().unwrap();

    let bc_estimate = result.bias_corrected_estimate();

    // Bias-corrected should be close to original for this data
    assert!(
        (bc_estimate - result.original_estimate).abs() < 1.0,
        "Bias-corrected should be close to original"
    );
}

/// R boot package validation
#[test]
fn test_r_boot_package_validation() {
    // Exact test case from R boot package documentation
    // library(boot)
    // data <- c(49, 52, 53, 54, 54, 54, 55, 56, 57, 57)
    // boot.results <- boot(data, function(d,i) mean(d[i]), R=10000)
    // boot.ci(boot.results, type="perc", conf=0.95)

    let data = vec![49.0, 52.0, 53.0, 54.0, 54.0, 54.0, 55.0, 56.0, 57.0, 57.0];

    let config = BootstrapConfig::new()
        .with_data(data)
        .with_iterations(10000)
        .with_seed(12345)
        .with_confidence_levels(vec![0.95]);

    let mut engine = BootstrapEngine::new(config).unwrap();
    let result = engine.analyze().unwrap();

    // Original mean = 54.1
    assert!(
        (result.original_estimate - 54.1).abs() < 0.01,
        "Original mean should be 54.1"
    );

    // CI should be reasonable (R gives approximately [52.3, 55.9])
    let ci = &result.confidence_intervals[0];
    assert!(
        ci.lower > 50.0 && ci.lower < 54.0,
        "Lower bound should be reasonable"
    );
    assert!(
        ci.upper > 54.0 && ci.upper < 58.0,
        "Upper bound should be reasonable"
    );
}

/// Test JSON export
#[test]
fn test_json_export() {
    let config = BootstrapConfig::new()
        .with_data(vec![1.0, 2.0, 3.0, 4.0, 5.0])
        .with_iterations(100)
        .with_seed(42);

    let mut engine = BootstrapEngine::new(config).unwrap();
    let result = engine.analyze().unwrap();
    let json = result.to_json().unwrap();

    assert!(json.contains("\"original_estimate\""));
    assert!(json.contains("\"bootstrap_std_error\""));
    assert!(json.contains("\"confidence_intervals\""));
}

fn run(method: BootstrapMethod, statistic: BootstrapStatistic, data: &[f64]) -> BootstrapResult {
    let config = BootstrapConfig::new()
        .with_data(data.to_vec())
        .with_iterations(100_000)
        .with_seed(42)
        .with_confidence_levels(vec![0.95])
        .with_statistic(statistic)
        .with_method(method);
    BootstrapEngine::new(config).unwrap().analyze().unwrap()
}

const SKEWED: [f64; 10] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 50.0];
const RETURNS: [f64; 10] = [
    0.05, -0.02, 0.08, 0.03, -0.05, 0.12, 0.01, -0.01, 0.06, 0.04,
];

#[test]
fn test_method_defaults_to_percentile_and_parses_from_yaml() {
    assert_eq!(BootstrapConfig::new().method, BootstrapMethod::Percentile);
    let config: BootstrapConfig = yaml_serde::from_str("data: [1, 2, 3]\nmethod: bca\n").unwrap();
    assert_eq!(config.method, BootstrapMethod::Bca);
    let config: BootstrapConfig = yaml_serde::from_str("data: [1, 2, 3]\n").unwrap();
    assert_eq!(config.method, BootstrapMethod::Percentile);
    assert!(yaml_serde::from_str::<BootstrapConfig>("data: [1, 2, 3]\nmethod: magic\n").is_err());
}

#[test]
fn test_basic_interval_reflects_the_percentile_interval() {
    let percentile = run(
        BootstrapMethod::Percentile,
        BootstrapStatistic::Mean,
        &SKEWED,
    );
    let basic = run(BootstrapMethod::Basic, BootstrapStatistic::Mean, &SKEWED);
    let estimate = percentile.original_estimate;
    let (p, b) = (
        &percentile.confidence_intervals[0],
        &basic.confidence_intervals[0],
    );
    assert!((b.lower - 2.0f64.mul_add(estimate, -p.upper)).abs() < 1e-9);
    assert!((b.upper - 2.0f64.mul_add(estimate, -p.lower)).abs() < 1e-9);
    assert_eq!(basic.method, BootstrapMethod::Basic);
}

/// Reference values from scipy.stats.bootstrap (200,000 resamples, other random numbers):
/// mean of the skewed sample, 95%. Basic: (-0.1, 15.2). `BCa`: (4.4, 26.6). The bootstrap means
/// of this sample sit on a coarse grid (one value, 50, is far from the rest), so the upper
/// bound can differ by a grid step depending on how the quantile is read.
#[test]
fn test_basic_and_bca_agree_with_scipy_on_skewed_data() {
    let basic = run(BootstrapMethod::Basic, BootstrapStatistic::Mean, &SKEWED);
    let ci = &basic.confidence_intervals[0];
    assert!((ci.lower - -0.1).abs() < 1.0, "basic lower {}", ci.lower);
    assert!((ci.upper - 15.2).abs() < 1.0, "basic upper {}", ci.upper);

    let bca = run(BootstrapMethod::Bca, BootstrapStatistic::Mean, &SKEWED);
    let ci = &bca.confidence_intervals[0];
    assert!((ci.lower - 4.4).abs() < 1.0, "bca lower {}", ci.lower);
    assert!(ci.upper > 22.0 && ci.upper < 28.0, "bca upper {}", ci.upper);
    // BCa moves the interval up for right-skewed data, away from the percentile interval
    let percentile = run(
        BootstrapMethod::Percentile,
        BootstrapStatistic::Mean,
        &SKEWED,
    );
    assert!(ci.upper > percentile.confidence_intervals[0].upper);
}

/// scipy `BCa` for the sample variance of `RETURNS`, 95%: (0.00127, 0.00541)
#[test]
fn test_bca_agrees_with_scipy_for_variance() {
    let bca = run(BootstrapMethod::Bca, BootstrapStatistic::Var, &RETURNS);
    let ci = &bca.confidence_intervals[0];
    assert!((ci.lower - 0.00127).abs() < 0.0003, "lower {}", ci.lower);
    assert!((ci.upper - 0.00541).abs() < 0.0005, "upper {}", ci.upper);
}

#[test]
fn test_bca_with_zero_acceleration() {
    // Leaving out any one value leaves the maximum at 3, so the jackknife has no spread
    let bca = run(
        BootstrapMethod::Bca,
        BootstrapStatistic::Max,
        &[1.0, 2.0, 3.0, 3.0],
    );
    let ci = &bca.confidence_intervals[0];
    assert!(ci.lower <= ci.upper && ci.upper <= 3.0);
}

#[test]
fn test_bca_rejects_a_distribution_on_one_side_of_the_estimate() {
    let config = BootstrapConfig::new()
        .with_data(vec![5.0, 5.0, 5.0, 5.0])
        .with_iterations(1000)
        .with_seed(1)
        .with_method(BootstrapMethod::Bca);
    let error = BootstrapEngine::new(config).unwrap().analyze().unwrap_err();
    assert!(error.contains("BCa interval undefined"), "{error}");
}

#[test]
fn test_bca_adjustment_is_undefined_for_extreme_acceleration() {
    assert!(engine::bca_adjusted_probability(0.0, 0.0, 0.975).is_ok());
    assert!(engine::bca_adjusted_probability(1.0, 1.0, 0.975)
        .unwrap_err()
        .contains("BCa interval undefined"));
}
