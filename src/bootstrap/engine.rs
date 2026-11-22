//! Bootstrap Resampling Engine
//!
//! Implements non-parametric bootstrap for confidence intervals.
//! Validated against R's boot package.

use super::config::{BootstrapConfig, BootstrapMethod, BootstrapStatistic};
use crate::core::precision;
use num_traits::ToPrimitive;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use statrs::distribution::{ContinuousCDF, Normal};

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
    #[must_use]
    pub const fn new(level: f64, lower: f64, upper: f64) -> Self {
        Self {
            level,
            lower,
            upper,
        }
    }

    /// Width of the interval
    #[must_use]
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
    /// How the confidence intervals were built
    pub method: BootstrapMethod,
    /// Confidence intervals
    pub confidence_intervals: Vec<ConfidenceInterval>,
    /// Bootstrap distribution (all resampled statistics)
    pub distribution: Vec<f64>,
    /// Number of bootstrap iterations
    pub iterations: usize,
}

impl BootstrapResult {
    /// Export results to YAML format
    ///
    /// # Panics
    ///
    /// Cannot panic: the result is numbers and a method's name, which YAML holds.
    #[must_use]
    pub fn to_yaml(&self) -> String {
        yaml_serde::to_string(&crate::core::precision::Rounded(self))
            .expect("a bootstrap result is numbers and a method's name")
    }

    /// Export results to JSON format
    ///
    /// # Errors
    ///
    /// Returns an error if JSON serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&crate::core::precision::Rounded(self))
    }

    /// Get the bias-corrected estimate
    #[must_use]
    pub fn bias_corrected_estimate(&self) -> f64 {
        self.original_estimate - self.bias
    }
}

/// Value at probability `p` of a sorted distribution (index `floor(p * len)`, kept inside the data)
fn quantile(sorted: &[f64], p: f64) -> f64 {
    // A NaN probability reads the smallest value
    let index = (p.clamp(0.0, 1.0) * precision::count(sorted.len()))
        .to_usize()
        .unwrap_or(0);
    sorted[index.min(sorted.len() - 1)]
}

/// Probability that the `BCa` method reads from the bootstrap distribution for nominal probability `p`
///
/// Fails when the acceleration makes the adjustment undefined.
pub(super) fn bca_adjusted_probability(z0: f64, a: f64, p: f64) -> Result<f64, String> {
    let z = Normal::standard().inverse_cdf(p);
    let denominator = a.mul_add(-(z0 + z), 1.0);
    if denominator <= 0.0 {
        return Err(
            "BCa interval undefined: the acceleration is too large for this data; use method percentile or basic"
                .to_string(),
        );
    }
    // The distribution function NORM.S.DIST uses: `statrs`'s is off by up to
    // 9e-11 for |z| from 0.9 up
    Ok(crate::core::array_calculator::evaluator::standard_cdf(
        z0 + (z0 + z) / denominator,
    ))
}

/// Bootstrap Resampling Engine
pub struct BootstrapEngine {
    config: BootstrapConfig,
    rng: StdRng,
}

impl BootstrapEngine {
    /// Create a new bootstrap engine
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid (see [`BootstrapConfig::validate`]).
    pub fn new(config: BootstrapConfig) -> Result<Self, String> {
        config.validate()?;

        let rng = config
            .seed
            .map_or_else(|| StdRng::from_rng(&mut rand::rng()), StdRng::seed_from_u64);

        Ok(Self { config, rng })
    }

    /// Run the bootstrap analysis
    ///
    /// # Errors
    ///
    /// Returns an error if the analysis fails.
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
        distribution.sort_by(f64::total_cmp);

        // Calculate bootstrap statistics
        let bootstrap_mean =
            distribution.iter().sum::<f64>() / precision::count(distribution.len());
        let variance: f64 = distribution
            .iter()
            .map(|x| (x - bootstrap_mean).powi(2))
            .sum::<f64>()
            / precision::count(distribution.len() - 1);
        let bootstrap_std_error = variance.sqrt();
        let bias = bootstrap_mean - original_estimate;

        // Calculate confidence intervals
        let confidence_intervals =
            self.calculate_confidence_intervals(&distribution, original_estimate)?;

        Ok(BootstrapResult {
            original_estimate,
            bootstrap_mean,
            bootstrap_std_error,
            bias,
            method: self.config.method,
            confidence_intervals,
            distribution,
            iterations: self.config.iterations,
        })
    }

    /// Compute the statistic on a sample: never empty, as the data has at
    /// least 2 values (`BootstrapConfig::validate`), a resample as many and a
    /// jackknife sample one fewer
    fn compute_statistic(&self, sample: &[f64]) -> f64 {
        match self.config.statistic {
            BootstrapStatistic::Mean => sample.iter().sum::<f64>() / precision::count(sample.len()),
            BootstrapStatistic::Median => {
                let mut sorted = sample.to_vec();
                sorted.sort_by(f64::total_cmp);
                let mid = sorted.len() / 2;
                if sorted.len().is_multiple_of(2) {
                    f64::midpoint(sorted[mid - 1], sorted[mid])
                } else {
                    sorted[mid]
                }
            },
            BootstrapStatistic::Std => {
                let mean = sample.iter().sum::<f64>() / precision::count(sample.len());
                let variance: f64 = sample.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
                    / precision::count(sample.len() - 1);
                variance.sqrt()
            },
            BootstrapStatistic::Var => {
                let mean = sample.iter().sum::<f64>() / precision::count(sample.len());
                sample.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
                    / precision::count(sample.len() - 1)
            },
            BootstrapStatistic::Percentile => {
                let mut sorted = sample.to_vec();
                sorted.sort_by(f64::total_cmp);
                // Below 0 is the smallest value, above 100 the largest, and NaN the smallest
                let last = precision::count(sorted.len()) - 1.0;
                let idx = ((self.config.percentile_value / 100.0) * last)
                    .clamp(0.0, last)
                    .round()
                    .to_usize()
                    .unwrap_or(0);
                sorted[idx]
            },
            BootstrapStatistic::Min => sample.iter().copied().fold(f64::INFINITY, f64::min),
            BootstrapStatistic::Max => sample.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        }
    }

    /// Calculate confidence intervals with the configured method
    fn calculate_confidence_intervals(
        &self,
        distribution: &[f64],
        estimate: f64,
    ) -> Result<Vec<ConfidenceInterval>, String> {
        /// The method, with the `BCa` constants worked out once for every level
        enum Interval {
            Percentile,
            Basic,
            Bca { z0: f64, a: f64 },
        }
        let interval = match self.config.method {
            BootstrapMethod::Percentile => Interval::Percentile,
            BootstrapMethod::Basic => Interval::Basic,
            BootstrapMethod::Bca => {
                let (z0, a) = self.bca_constants(distribution, estimate)?;
                Interval::Bca { z0, a }
            },
        };
        self.config
            .confidence_levels
            .iter()
            .map(|&level| {
                let alpha = 1.0 - level;
                let (lower_p, upper_p) = (alpha / 2.0, 1.0 - alpha / 2.0);
                let (lower, upper) = match interval {
                    Interval::Percentile => (
                        quantile(distribution, lower_p),
                        quantile(distribution, upper_p),
                    ),
                    Interval::Basic => (
                        2.0f64.mul_add(estimate, -quantile(distribution, upper_p)),
                        2.0f64.mul_add(estimate, -quantile(distribution, lower_p)),
                    ),
                    Interval::Bca { z0, a } => (
                        quantile(distribution, bca_adjusted_probability(z0, a, lower_p)?),
                        quantile(distribution, bca_adjusted_probability(z0, a, upper_p)?),
                    ),
                };
                Ok(ConfidenceInterval::new(level, lower, upper))
            })
            .collect()
    }

    /// Bias correction `z0` and acceleration `a` for the `BCa` method
    ///
    /// `z0` is the normal quantile of the share of bootstrap values below the estimate;
    /// `a` comes from the jackknife (leave-one-out) values of the statistic.
    fn bca_constants(&self, distribution: &[f64], estimate: f64) -> Result<(f64, f64), String> {
        let below = distribution.partition_point(|&x| x < estimate);
        if below == 0 || below == distribution.len() {
            return Err(
                "BCa interval undefined: every bootstrap value is on the same side of the estimate; use method percentile or basic"
                    .to_string(),
            );
        }
        let z0 = Normal::standard()
            .inverse_cdf(precision::count(below) / precision::count(distribution.len()));

        let data = &self.config.data;
        let leave_one_out: Vec<f64> = (0..data.len())
            .map(|i| {
                let rest: Vec<f64> = data
                    .iter()
                    .enumerate()
                    .filter(|&(j, _)| j != i)
                    .map(|(_, &x)| x)
                    .collect();
                self.compute_statistic(&rest)
            })
            .collect();
        let mean = leave_one_out.iter().sum::<f64>() / precision::count(leave_one_out.len());
        let sum_squares: f64 = leave_one_out.iter().map(|x| (mean - x).powi(2)).sum();
        let sum_cubes: f64 = leave_one_out.iter().map(|x| (mean - x).powi(3)).sum();
        let a = if sum_squares > 0.0 {
            sum_cubes / (6.0 * sum_squares.powf(1.5))
        } else {
            0.0
        };
        Ok((z0, a))
    }

    /// Get the configuration
    #[must_use]
    pub const fn config(&self) -> &BootstrapConfig {
        &self.config
    }
}

#[cfg(test)]
mod engine_tests {
    use super::*;

    #[test]
    fn a_quantile_stays_inside_the_data_and_nan_reads_the_smallest() {
        let sorted = [1.0, 2.0, 3.0, 4.0];
        assert_exact!(quantile(&sorted, 0.5), 3.0);
        assert_exact!(quantile(&sorted, 0.0), 1.0);
        assert_exact!(quantile(&sorted, 1.0), 4.0);
        assert_exact!(quantile(&sorted, 2.0), 4.0);
        assert_exact!(quantile(&sorted, -1.0), 1.0);
        assert_exact!(quantile(&sorted, f64::NAN), 1.0);
    }

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

    /// A `BCa` engine over `data`, at one level so near 1 that its normal
    /// quantile is +-7.1305 (1 - 0.5e-12 a side)
    fn extreme_bca(data: Vec<f64>, statistic: BootstrapStatistic) -> BootstrapEngine {
        let config = BootstrapConfig::new()
            .with_data(data)
            .with_statistic(statistic)
            .with_method(BootstrapMethod::Bca)
            .with_iterations(2000)
            .with_seed(1)
            .with_confidence_levels(vec![1.0 - 1e-12]);
        BootstrapEngine::new(config).unwrap()
    }

    const TOO_LARGE: &str = "BCa interval undefined: the acceleration is too large for this data; use method percentile or basic";

    /// `BCa` divides by 1 - a (z0 + z). The jackknife gives the acceleration
    /// `a` without any draw (by hand and in Python: +-0.154518 for both data
    /// below), so the interval fails once z0 + z passes 1 / a = 6.4717.
    ///
    /// The largest of nineteen 0s, a 0.5 and a 1: a = +0.154518, and z0 is
    /// the normal quantile of the share of resamples without the 1, (20/21)^21
    /// = 0.359 (z0 = -0.36): above the 0.255 at which the upper end fails,
    /// and the lower end (z = -7.13) is defined.
    ///
    /// The variance of ten -1s, ten 1s and a 0: leaving the 0 out raises it,
    /// so a = -0.154518, and the lower end fails while the share of resamples
    /// below the estimate is under 0.745 (with seed 1 it is 0.70)
    #[test]
    fn an_acceleration_too_large_fails_at_either_end() {
        let mut data = vec![0.0; 19];
        data.extend([1.0, 0.5]);
        let mut upper = extreme_bca(data, BootstrapStatistic::Max);
        let mut data: Vec<f64> = (0..20)
            .map(|i| if i % 2 == 0 { -1.0 } else { 1.0 })
            .collect();
        data.push(0.0);
        let mut lower = extreme_bca(data, BootstrapStatistic::Var);
        for (engine, lower_end_ok) in [(&mut upper, true), (&mut lower, false)] {
            let result = engine.analyze();
            assert_eq!(result.unwrap_err(), TOO_LARGE);
            let distribution = {
                let mut percentile = BootstrapEngine::new(
                    engine
                        .config()
                        .clone()
                        .with_method(BootstrapMethod::Percentile),
                )
                .unwrap();
                percentile.analyze().unwrap()
            };
            let (z0, a) = engine
                .bca_constants(&distribution.distribution, distribution.original_estimate)
                .unwrap();
            assert!((a.abs() - 0.154_517_511_550_184).abs() <= 1e-12, "{a}");
            assert_eq!(
                (
                    bca_adjusted_probability(z0, a, 0.5e-12).is_ok(),
                    bca_adjusted_probability(z0, a, 1.0 - 0.5e-12).is_ok()
                ),
                (lower_end_ok, !lower_end_ok)
            );
        }
    }

    /// The middle value of an odd number of values, and the configuration kept
    #[test]
    fn the_median_of_an_odd_count_is_its_middle_value() {
        let config = BootstrapConfig::new()
            .with_data(vec![3.0, 1.0, 2.0])
            .with_statistic(BootstrapStatistic::Median)
            .with_iterations(10)
            .with_seed(3);
        let mut engine = BootstrapEngine::new(config).unwrap();
        assert_eq!(engine.config().data, vec![3.0, 1.0, 2.0]);
        assert_exact!(engine.analyze().unwrap().original_estimate, 2.0);
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
