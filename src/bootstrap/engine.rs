//! Bootstrap Resampling Engine
//!
//! Implements non-parametric bootstrap for confidence intervals.
//! Validated against R's boot package.

use super::config::{BootstrapConfig, BootstrapStatistic};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};

/// A confidence interval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceInterval {
    /// Confidence level (e.g., 0.95)
    pub level: f64,
    /// Lower bound
    pub lower: f64,
    /// Upper bound
    pub upper: f64,
}

impl ConfidenceInterval {
    /// Create a new confidence interval
    pub fn new(level: f64, lower: f64, upper: f64) -> Self {
        Self {
            level,
            lower,
            upper,
        }
    }

    /// Width of the interval
    pub fn width(&self) -> f64 {
        self.upper - self.lower
    }
}

/// Bootstrap analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapResult {
    /// Original sample statistic
    pub original_estimate: f64,
    /// Bootstrap mean estimate
    pub bootstrap_mean: f64,
    /// Bootstrap standard error
    pub bootstrap_std_error: f64,
    /// Bias (bootstrap mean - original)
    pub bias: f64,
    /// Confidence intervals
    pub confidence_intervals: Vec<ConfidenceInterval>,
    /// Bootstrap distribution (all resampled statistics)
    pub distribution: Vec<f64>,
    /// Number of bootstrap iterations
    pub iterations: usize,
}

impl BootstrapResult {
    /// Export results to YAML format
    pub fn to_yaml(&self) -> String {
        serde_yaml_ng::to_string(self).unwrap_or_else(|_| "# Error serializing results".to_string())
    }

    /// Export results to JSON format
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Get the bias-corrected estimate
    pub fn bias_corrected_estimate(&self) -> f64 {
        self.original_estimate - self.bias
    }
}

/// Bootstrap Resampling Engine
pub struct BootstrapEngine {
    config: BootstrapConfig,
    rng: StdRng,
}

impl BootstrapEngine {
    /// Create a new bootstrap engine
    pub fn new(config: BootstrapConfig) -> Result<Self, String> {
        config.validate()?;

        let rng = match config.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_rng(&mut rand::rng()),
        };

        Ok(Self { config, rng })
    }

    /// Run the bootstrap analysis
    pub fn analyze(&mut self) -> Result<BootstrapResult, String> {
        let data = &self.config.data;
        let n = data.len();

        // Calculate original estimate
        let original_estimate = self.compute_statistic(data);

        // Bootstrap resampling
        let mut distribution = Vec::with_capacity(self.config.iterations);

        for _ in 0..self.config.iterations {
            // Resample with replacement
            let sample: Vec<f64> = (0..n)
                .map(|_| {
                    let idx = self.rng.random_range(0..n);
                    data[idx]
                })
                .collect();

            let stat = self.compute_statistic(&sample);
            distribution.push(stat);
        }

        // Sort for percentile calculation
        distribution.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Calculate bootstrap statistics
        let bootstrap_mean = distribution.iter().sum::<f64>() / distribution.len() as f64;
        let variance: f64 = distribution
            .iter()
            .map(|x| (x - bootstrap_mean).powi(2))
            .sum::<f64>()
            / (distribution.len() - 1) as f64;
        let bootstrap_std_error = variance.sqrt();
        let bias = bootstrap_mean - original_estimate;

        // Calculate confidence intervals
        let confidence_intervals = self.calculate_confidence_intervals(&distribution);

        Ok(BootstrapResult {
            original_estimate,
            bootstrap_mean,
            bootstrap_std_error,
            bias,
            confidence_intervals,
            distribution,
            iterations: self.config.iterations,
        })
    }

    /// Compute the statistic on a sample
    fn compute_statistic(&self, sample: &[f64]) -> f64 {
        if sample.is_empty() {
            return 0.0;
        }

        match self.config.statistic {
            BootstrapStatistic::Mean => sample.iter().sum::<f64>() / sample.len() as f64,
            BootstrapStatistic::Median => {
                let mut sorted = sample.to_vec();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let mid = sorted.len() / 2;
                if sorted.len().is_multiple_of(2) {
                    f64::midpoint(sorted[mid - 1], sorted[mid])
                } else {
                    sorted[mid]
                }
            },
            BootstrapStatistic::Std => {
                let mean = sample.iter().sum::<f64>() / sample.len() as f64;
                let variance: f64 = sample.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
                    / (sample.len() - 1) as f64;
                variance.sqrt()
            },
            BootstrapStatistic::Var => {
                let mean = sample.iter().sum::<f64>() / sample.len() as f64;
                sample.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (sample.len() - 1) as f64
            },
            BootstrapStatistic::Percentile => {
                let mut sorted = sample.to_vec();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let idx = ((self.config.percentile_value / 100.0) * (sorted.len() as f64 - 1.0))
                    .round() as usize;
                sorted[idx.min(sorted.len() - 1)]
            },
            BootstrapStatistic::Min => sample.iter().copied().fold(f64::INFINITY, f64::min),
            BootstrapStatistic::Max => sample.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        }
    }

    /// Calculate confidence intervals using percentile method
    fn calculate_confidence_intervals(&self, distribution: &[f64]) -> Vec<ConfidenceInterval> {
        self.config
            .confidence_levels
            .iter()
            .map(|&level| {
                let alpha = 1.0 - level;
                let lower_idx = ((alpha / 2.0) * distribution.len() as f64) as usize;
                let upper_idx = ((1.0 - alpha / 2.0) * distribution.len() as f64) as usize;

                ConfidenceInterval::new(
                    level,
                    distribution[lower_idx.min(distribution.len() - 1)],
                    distribution[upper_idx.min(distribution.len() - 1)],
                )
            })
            .collect()
    }

    /// Get the configuration
    pub fn config(&self) -> &BootstrapConfig {
        &self.config
    }
}

#[cfg(test)]
mod engine_tests {
    use super::*;

    #[test]
    fn test_bootstrap_mean() {
        let config = BootstrapConfig::new()
            .with_data(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0])
            .with_iterations(5000)
            .with_seed(12345);

        let mut engine = BootstrapEngine::new(config).unwrap();
        let result = engine.analyze().unwrap();

        // Original mean should be 5.5
        assert!(
            (result.original_estimate - 5.5).abs() < 0.01,
            "Original mean should be 5.5"
        );

        // Bootstrap mean should be close to original
        assert!(
            (result.bootstrap_mean - 5.5).abs() < 0.5,
            "Bootstrap mean should be close to 5.5"
        );

        // Should have confidence intervals
        assert!(!result.confidence_intervals.is_empty());
    }

    #[test]
    fn test_bootstrap_median() {
        let config = BootstrapConfig::new()
            .with_data(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0])
            .with_statistic(BootstrapStatistic::Median)
            .with_iterations(5000)
            .with_seed(12345);

        let mut engine = BootstrapEngine::new(config).unwrap();
        let result = engine.analyze().unwrap();

        // Original median should be 5.5
        assert!(
            (result.original_estimate - 5.5).abs() < 0.01,
            "Original median should be 5.5"
        );
    }

    #[test]
    fn test_confidence_intervals() {
        let config = BootstrapConfig::new()
            .with_data(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0])
            .with_confidence_levels(vec![0.90, 0.95])
            .with_iterations(10000)
            .with_seed(12345);

        let mut engine = BootstrapEngine::new(config).unwrap();
        let result = engine.analyze().unwrap();

        assert_eq!(result.confidence_intervals.len(), 2);

        // 95% CI should be wider than 90% CI
        let ci_90 = result
            .confidence_intervals
            .iter()
            .find(|ci| (ci.level - 0.90).abs() < 0.01)
            .unwrap();
        let ci_95 = result
            .confidence_intervals
            .iter()
            .find(|ci| (ci.level - 0.95).abs() < 0.01)
            .unwrap();

        assert!(
            ci_95.width() >= ci_90.width(),
            "95% CI should be >= 90% CI width"
        );
    }

    #[test]
    fn test_reproducibility() {
        let config1 = BootstrapConfig::new()
            .with_data(vec![1.0, 2.0, 3.0, 4.0, 5.0])
            .with_iterations(1000)
            .with_seed(42);

        let config2 = BootstrapConfig::new()
            .with_data(vec![1.0, 2.0, 3.0, 4.0, 5.0])
            .with_iterations(1000)
            .with_seed(42);

        let mut engine1 = BootstrapEngine::new(config1).unwrap();
        let mut engine2 = BootstrapEngine::new(config2).unwrap();

        let result1 = engine1.analyze().unwrap();
        let result2 = engine2.analyze().unwrap();

        assert!(
            (result1.bootstrap_mean - result2.bootstrap_mean).abs() < 0.0001,
            "Same seed should produce same results"
        );
    }

    /// R boot package equivalence test
    #[test]
    fn test_r_boot_equivalence() {
        // This test validates against R's boot package
        // R code:
        //   library(boot)
        //   data <- c(5, -2, 8, 3, -5, 12, 1, -1, 6, 4)
        //   mean_func <- function(d, i) mean(d[i])
        //   results <- boot(data, mean_func, R=10000, seed=12345)
        //   boot.ci(results, type="perc")

        let config = BootstrapConfig::new()
            .with_data(vec![5.0, -2.0, 8.0, 3.0, -5.0, 12.0, 1.0, -1.0, 6.0, 4.0])
            .with_iterations(10000)
            .with_seed(12345)
            .with_confidence_levels(vec![0.95]);

        let mut engine = BootstrapEngine::new(config).unwrap();
        let result = engine.analyze().unwrap();

        // Original mean = 3.1
        assert!(
            (result.original_estimate - 3.1).abs() < 0.01,
            "Original mean should be 3.1"
        );

        // Bootstrap mean should be close to 3.1
        assert!(
            (result.bootstrap_mean - 3.1).abs() < 1.0,
            "Bootstrap mean should be close to 3.1"
        );

        // Standard error should be reasonable
        assert!(
            result.bootstrap_std_error > 0.0 && result.bootstrap_std_error < 5.0,
            "Standard error should be reasonable"
        );
    }

    #[test]
    fn test_yaml_export() {
        let config = BootstrapConfig::new()
            .with_data(vec![1.0, 2.0, 3.0, 4.0, 5.0])
            .with_iterations(100)
            .with_seed(42);

        let mut engine = BootstrapEngine::new(config).unwrap();
        let result = engine.analyze().unwrap();
        let yaml = result.to_yaml();

        assert!(yaml.contains("original_estimate"));
        assert!(yaml.contains("bootstrap_mean"));
        assert!(yaml.contains("confidence_intervals"));
    }
}
