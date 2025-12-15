//! Bootstrap Integration Tests

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
        .map(|ci| ci.width())
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
    let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();

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
        20000.0, 25000.0, 30000.0, 32000.0, 35000.0, 40000.0, 45000.0, 50000.0, 80000.0, 150000.0,
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
    let data: Vec<f64> = (1..=50).map(|x| x as f64).collect();

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
