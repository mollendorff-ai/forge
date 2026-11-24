//! Statistical Calculations for Monte Carlo Results
//!
//! Provides:
//! - Percentiles (P5, P10, P25, P50, P75, P90, P95)
//! - Summary statistics (mean, median, std dev, min, max)
//! - Probability thresholds (P(X > target))
//! - Histogram bin data
//!
//! Every figure is worked out the plain way first. When that overflows
//! although every sample is a finite number (values above about 1e154, whose
//! squares pass the largest `f64`, or a sum of many near 1e306), it is worked
//! out again on the samples divided by a power of two, which is exact, and
//! scaled back: the figure is then a number whenever the samples are, and a
//! model that summarizes today keeps every digit (ADR-079, roadmap
//! T1-HUGE-STATS). Only the variance can still be too large to be a number.

use crate::core::precision;
use num_traits::ToPrimitive;
use std::collections::BTreeMap;

/// Comprehensive statistics for a simulation output
#[derive(Debug, Clone)]
pub struct Statistics {
    /// Number of samples
    pub count: usize,
    /// Mean (average)
    pub mean: f64,
    /// Median (P50)
    pub median: f64,
    /// Standard deviation
    pub std_dev: f64,
    /// Variance, or `None` when it is larger than the largest `f64`: the
    /// variance of values near 1e307 is near 1e613 (ADR-079)
    pub variance: Option<f64>,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Skewness (asymmetry)
    pub skewness: f64,
    /// Kurtosis (tail heaviness)
    pub kurtosis: f64,
    /// Percentiles (key: percentile 0-100, value: value)
    pub percentiles: BTreeMap<u8, f64>,
}

impl Statistics {
    /// Calculate statistics from samples
    #[must_use]
    pub fn from_samples(samples: &[f64]) -> Self {
        if samples.is_empty() {
            return Self::empty();
        }

        let count = samples.len();
        let n = precision::count(count);

        // Sort for percentiles
        let mut sorted = samples.to_vec();
        sorted.sort_by(f64::total_cmp);

        let mean = mean(samples);
        // The mean square of the deviations, each divided by `unit`
        let spread = |unit: f64| {
            samples
                .iter()
                .map(|x| (x / unit - mean / unit).powi(2))
                .sum::<f64>()
                / n
        };
        let mut unit = 1.0;
        let mut spread_in_units = spread(unit);
        if overflowed(spread_in_units, samples) {
            unit = power_of_two_below(samples);
            spread_in_units = spread(unit);
        }
        let std_dev_in_units = spread_in_units.sqrt();
        let std_dev = std_dev_in_units * unit;
        let variance = Some(spread_in_units * unit * unit).filter(|v| v.is_finite());

        // Min/Max
        let min = sorted[0];
        let max = sorted[count - 1];

        // Median
        let median = percentile_sorted(&sorted, 50.0);

        // Skewness and Kurtosis, of deviations in the same units
        let (skewness, kurtosis) = if std_dev_in_units > 0.0 {
            let z = |x: &f64| (x / unit - mean / unit) / std_dev_in_units;
            let m3: f64 = samples.iter().map(|x| z(x).powi(3)).sum::<f64>() / n;
            let m4: f64 = samples.iter().map(|x| z(x).powi(4)).sum::<f64>() / n;
            (m3, m4 - 3.0) // Excess kurtosis (normal = 0)
        } else {
            (0.0, 0.0)
        };

        // Standard percentiles
        let mut percentiles = BTreeMap::new();
        for p in [5, 10, 25, 50, 75, 90, 95] {
            percentiles.insert(p, percentile_sorted(&sorted, f64::from(p)));
        }

        Self {
            count,
            mean,
            median,
            std_dev,
            variance,
            min,
            max,
            skewness,
            kurtosis,
            percentiles,
        }
    }

    /// Create empty statistics
    const fn empty() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            median: 0.0,
            std_dev: 0.0,
            variance: Some(0.0),
            min: 0.0,
            max: 0.0,
            skewness: 0.0,
            kurtosis: 0.0,
            percentiles: BTreeMap::new(),
        }
    }

    /// Get a specific percentile
    #[must_use]
    pub fn percentile(&self, p: u8) -> Option<f64> {
        self.percentiles.get(&p).copied()
    }
}

/// The mean of `samples`, a number whenever they all are (ADR-079).
///
/// NaN for none, as 0 / 0. Summed plainly first; when the sum overflows
/// although every sample is finite, summed again with each sample divided by
/// a power of two.
#[must_use]
pub fn mean(samples: &[f64]) -> f64 {
    let n = precision::count(samples.len());
    let plain = samples.iter().sum::<f64>() / n;
    if !overflowed(plain, samples) {
        return plain;
    }
    let unit = power_of_two_below(samples);
    samples.iter().map(|x| x / unit).sum::<f64>() / n * unit
}

/// Whether `figure` is not a number although every one of `samples` is: the
/// arithmetic overflowed, and the samples can be scaled down
fn overflowed(figure: f64, samples: &[f64]) -> bool {
    !figure.is_finite() && samples.iter().all(|x| x.is_finite())
}

/// The power of two at or below the largest magnitude of `samples`, which
/// are finite and not all near zero (their arithmetic overflowed). Dividing
/// by it is exact and leaves every sample below 2 in magnitude.
fn power_of_two_below(samples: &[f64]) -> f64 {
    /// The exponent bits of an `f64`: with no mantissa, a power of two
    const EXPONENT: u64 = 0x7ff0_0000_0000_0000;
    let largest = samples.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
    f64::from_bits(largest.to_bits() & EXPONENT)
}

/// Calculate percentile from sorted array (linear interpolation). Both
/// callers return early for no samples, so `sorted` has at least one.
fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }

    let n = precision::count(sorted.len());
    // Below 0 is the smallest value and above 100 the largest; a NaN
    // percentile reads the smallest
    let rank = (p / 100.0 * (n - 1.0)).clamp(0.0, n - 1.0);
    let lower = rank.floor().to_usize().unwrap_or(0);
    let upper = rank.ceil().to_usize().unwrap_or(0);

    if lower == upper {
        sorted[lower]
    } else {
        let frac = rank - precision::count(lower);
        sorted[lower].mul_add(1.0 - frac, sorted[upper] * frac)
    }
}

/// Calculate additional percentiles from samples
#[must_use]
pub fn calculate_percentiles(samples: &[f64], percentile_list: &[u8]) -> BTreeMap<u8, f64> {
    if samples.is_empty() {
        return BTreeMap::new();
    }

    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);

    percentile_list
        .iter()
        .map(|&p| (p, percentile_sorted(&sorted, f64::from(p))))
        .collect()
}

/// Histogram data for visualization
#[derive(Debug, Clone)]
pub struct Histogram {
    /// Bin edges (len = `num_bins` + 1)
    pub bin_edges: Vec<f64>,
    /// Counts per bin
    pub counts: Vec<usize>,
    /// Frequencies (counts / total)
    pub frequencies: Vec<f64>,
}

impl Histogram {
    /// Create histogram from samples
    pub fn from_samples(samples: &[f64], num_bins: usize) -> Self {
        if samples.is_empty() || num_bins == 0 {
            return Self {
                bin_edges: vec![],
                counts: vec![],
                frequencies: vec![],
            };
        }

        let min = samples.iter().copied().fold(f64::INFINITY, f64::min);
        let max = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        // Handle case where all values are the same
        let (actual_min, actual_max) = if (max - min).abs() < 1e-10 {
            (min - 0.5, max + 0.5)
        } else {
            (min, max)
        };

        // A range wider than the largest `f64` (from below -9e307 to above
        // 9e307) is cut in halves: halving is exact, so a range that fits is
        // cut exactly as before (ADR-079)
        let half = if (actual_max - actual_min).is_finite() {
            1.0
        } else {
            0.5
        };
        let (low, high) = (actual_min * half, actual_max * half);
        let bin_width = (high - low) / precision::count(num_bins);

        // Create bin edges
        let bin_edges: Vec<f64> = (0..=num_bins)
            .map(|i| precision::count(i).mul_add(bin_width, low) / half)
            .collect();

        // Count samples per bin
        let mut counts = vec![0usize; num_bins];
        for &sample in samples {
            // The largest sample falls on the last edge: it counts in the last bin
            let halved = sample * half;
            let bin = ((halved - low) / bin_width)
                .floor()
                .to_usize()
                .unwrap_or(0)
                .min(num_bins - 1);
            counts[bin] += 1;
        }

        // Calculate frequencies
        let total = precision::count(samples.len());
        let frequencies: Vec<f64> = counts
            .iter()
            .map(|&c| precision::count(c) / total)
            .collect();

        Self {
            bin_edges,
            counts,
            frequencies,
        }
    }
}

/// Parse threshold string (e.g., "> 0", "< 100000", ">= 50")
///
/// # Errors
///
/// Returns an error if the threshold format is invalid or the value cannot be parsed.
pub fn parse_threshold(threshold: &str) -> Result<(String, f64), String> {
    let threshold = threshold.trim();

    let operators = [">=", "<=", ">", "<", "="];
    for op in operators {
        if let Some(rest) = threshold.strip_prefix(op) {
            let value_str = rest.trim();
            let value: f64 = value_str
                .parse()
                .map_err(|_| format!("Invalid threshold value: {value_str}"))?;
            return Ok((op.to_string(), value));
        }
    }

    Err(format!(
        "Invalid threshold format: {threshold}. Use '> 0', '< 100', etc."
    ))
}

/// Evaluate a threshold against samples
#[must_use]
pub fn evaluate_threshold(samples: &[f64], operator: &str, value: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }

    let count = match operator {
        ">" => samples.iter().filter(|&&x| x > value).count(),
        ">=" => samples.iter().filter(|&&x| x >= value).count(),
        "<" => samples.iter().filter(|&&x| x < value).count(),
        "<=" => samples.iter().filter(|&&x| x <= value).count(),
        "=" => samples
            .iter()
            .filter(|&&x| (x - value).abs() < 1e-10)
            .count(),
        _ => 0,
    };

    precision::count(count) / precision::count(samples.len())
}

// Financial math: exact float comparison validated against Excel/Gnumeric/R
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_percentile_outside_0_to_100_reads_the_end_and_nan_the_smallest() {
        let sorted = [1.0, 2.0, 3.0];
        assert_exact!(percentile_sorted(&sorted, 50.0), 2.0);
        assert_exact!(percentile_sorted(&sorted, 25.0), 1.5);
        assert_exact!(percentile_sorted(&sorted, 100.0), 3.0);
        assert_exact!(percentile_sorted(&sorted, 150.0), 3.0);
        assert_exact!(percentile_sorted(&sorted, -10.0), 1.0);
        assert_exact!(percentile_sorted(&sorted, f64::NAN), 1.0);
    }

    #[test]
    fn nan_samples_do_not_panic() {
        let samples = [3.0, f64::NAN, 1.0, 2.0];
        let _ = Statistics::from_samples(&samples);
        let _ = calculate_percentiles(&samples, &[50, 90]);
    }

    /// The plain formulas, as `from_samples` computed them before ADR-079
    fn plain(samples: &[f64]) -> (f64, f64, f64, f64, f64) {
        let n = precision::count(samples.len());
        let mean = samples.iter().sum::<f64>() / n;
        let variance = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        let sd = variance.sqrt();
        let m3 = samples
            .iter()
            .map(|x| ((x - mean) / sd).powi(3))
            .sum::<f64>()
            / n;
        let m4 = samples
            .iter()
            .map(|x| ((x - mean) / sd).powi(4))
            .sum::<f64>()
            / n;
        (mean, variance, sd, m3, m4 - 3.0)
    }

    /// A model that summarizes without overflow keeps every digit (ADR-079)
    #[test]
    fn figures_that_fit_are_the_plain_formulas_to_the_last_digit() {
        let samples: Vec<f64> = (1..=997)
            .map(|i| f64::from(i).sqrt().mul_add(1e150, -3e151))
            .collect();
        let stats = Statistics::from_samples(&samples);
        let (mean, variance, sd, skewness, kurtosis) = plain(&samples);
        assert_eq!(
            (stats.mean, stats.variance, stats.std_dev),
            (mean, Some(variance), sd)
        );
        assert_eq!((stats.skewness, stats.kurtosis), (skewness, kurtosis));
    }

    /// Values near 1e307 square past the largest `f64`: the standard
    /// deviation, skewness and kurtosis are numbers, the variance is not
    /// (roadmap T1-HUGE-STATS)
    #[test]
    fn the_spread_of_values_near_1e307_is_a_number_and_their_variance_none() {
        let samples = [-1e307, 1e307, -1e307, 1e307];
        assert!(
            plain(&samples).2.is_infinite(),
            "the plain formula overflows"
        );
        let stats = Statistics::from_samples(&samples);
        assert_exact!(stats.mean, 0.0);
        assert!(
            (stats.std_dev / 1e307 - 1.0).abs() < 1e-15,
            "{}",
            stats.std_dev
        );
        assert_eq!(stats.variance, None);
        // Two values, equally often: no skew, and an excess kurtosis of -2
        assert_exact!(stats.skewness, 0.0);
        assert!((stats.kurtosis + 2.0).abs() < 1e-15, "{}", stats.kurtosis);
    }

    /// A sum of values near 1e308 overflows; their mean does not
    #[test]
    fn the_mean_of_values_near_1e308_is_a_number() {
        let samples = [1e308, 1.5e308, 1.7e308];
        assert!(samples.iter().sum::<f64>().is_infinite());
        let expected = 1.4e308;
        assert!((mean(&samples) / expected - 1.0).abs() < 1e-15);
        let stats = Statistics::from_samples(&samples);
        assert_exact!(stats.mean, mean(&samples));
        // Deviations of -0.4, 0.1 and 0.3 times 1e308 square past it too
        let sd = (0.26_f64 / 3.0).sqrt() * 1e308;
        assert!(
            (stats.std_dev / sd - 1.0).abs() < 1e-15,
            "{}",
            stats.std_dev
        );
        assert_eq!(stats.variance, None);
    }

    /// A sample that is not a number is not scaled away
    #[test]
    fn samples_that_are_not_numbers_give_no_number() {
        assert!(mean(&[]).is_nan());
        assert!(mean(&[f64::INFINITY, 1.0]).is_infinite());
        let stats = Statistics::from_samples(&[f64::INFINITY, 1.0]);
        assert!(stats.std_dev.is_nan());
        assert_eq!(stats.variance, None);
        assert_eq!(Statistics::from_samples(&[2.0, 2.0]).variance, Some(0.0));
    }

    /// From -1e308 to 1e308 is wider than the largest `f64`: the histogram
    /// is cut in halves, its edges and counts exact
    #[test]
    fn a_histogram_wider_than_the_largest_number_is_cut_in_halves() {
        let hist = Histogram::from_samples(&[-1e308, 0.0, 1e308], 2);
        assert_eq!(
            (hist.bin_edges, hist.counts),
            (vec![-1e308, 0.0, 1e308], vec![1, 2])
        );
    }

    #[test]
    fn test_statistics_basic() {
        let samples: Vec<f64> = (1..=100).map(f64::from).collect();
        let stats = Statistics::from_samples(&samples);

        assert_eq!(stats.count, 100);
        assert!((stats.mean - 50.5).abs() < 0.01);
        assert!((stats.median - 50.5).abs() < 0.5);
        assert_exact!(stats.min, 1.0);
        assert_exact!(stats.max, 100.0);
    }

    #[test]
    fn test_percentiles() {
        let samples: Vec<f64> = (1..=100).map(f64::from).collect();
        let stats = Statistics::from_samples(&samples);

        // P50 should be median (around 50.5)
        let p50 = stats.percentile(50).unwrap();
        assert!((p50 - 50.5).abs() < 1.0);

        // P10 should be around 10
        let p10 = stats.percentile(10).unwrap();
        assert!((p10 - 10.0).abs() < 2.0);

        // P90 should be around 90
        let p90 = stats.percentile(90).unwrap();
        assert!((p90 - 90.0).abs() < 2.0);
    }

    #[test]
    fn test_histogram() {
        let samples: Vec<f64> = (0..100).map(f64::from).collect();
        let hist = Histogram::from_samples(&samples, 10);

        assert_eq!(hist.counts.len(), 10);
        assert_eq!(hist.bin_edges.len(), 11);

        // Each bin should have approximately 10 samples
        for count in &hist.counts {
            assert!(*count >= 9 && *count <= 11);
        }

        // Frequencies should sum to 1
        let freq_sum: f64 = hist.frequencies.iter().sum();
        assert!((freq_sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_threshold() {
        let (op, val) = parse_threshold("> 0").unwrap();
        assert_eq!(op, ">");
        assert_exact!(val, 0.0);

        let (op, val) = parse_threshold("<= 100000").unwrap();
        assert_eq!(op, "<=");
        assert_exact!(val, 100_000.0);

        let (op, val) = parse_threshold(">= -50.5").unwrap();
        assert_eq!(op, ">=");
        assert_exact!(val, -50.5);

        assert!(parse_threshold("invalid").is_err());
        assert_eq!(
            parse_threshold("> ten"),
            Err("Invalid threshold value: ten".to_string())
        );
    }

    #[test]
    fn test_evaluate_threshold() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        assert_exact!(evaluate_threshold(&samples, ">", 3.0), 0.4); // 4, 5
        assert_exact!(evaluate_threshold(&samples, ">=", 3.0), 0.6); // 3, 4, 5
        assert_exact!(evaluate_threshold(&samples, "<", 3.0), 0.4); // 1, 2
        assert_exact!(evaluate_threshold(&samples, "<=", 3.0), 0.6); // 1, 2, 3
        assert_exact!(evaluate_threshold(&samples, "=", 3.0), 0.2); // 3
        assert_exact!(evaluate_threshold(&samples, "=", 3.000_000_000_01), 0.2); // within 1e-10
        assert_exact!(evaluate_threshold(&samples, "=", 3.1), 0.0);
        // An operator parse_threshold does not give counts nothing
        assert_exact!(evaluate_threshold(&samples, "!=", 3.0), 0.0);
        // No samples: no probability
        assert_exact!(evaluate_threshold(&[], ">", 3.0), 0.0);
    }

    /// No samples, or no bins, give an empty histogram; samples all equal
    /// give one bin from half below to half above them.
    #[test]
    fn a_histogram_of_nothing_is_empty_and_of_one_value_is_a_unit_wide() {
        for (samples, bins) in [(&[][..], 10), (&[1.0, 2.0][..], 0)] {
            let hist = Histogram::from_samples(samples, bins);
            assert_eq!(
                (hist.bin_edges, hist.counts, hist.frequencies),
                (vec![], vec![], vec![])
            );
        }
        let hist = Histogram::from_samples(&[7.0, 7.0, 7.0], 2);
        assert_eq!(
            (hist.bin_edges, hist.counts, hist.frequencies),
            (vec![6.5, 7.0, 7.5], vec![0, 3], vec![0.0, 1.0])
        );
    }

    #[test]
    fn percentiles_of_no_samples_are_none() {
        assert_eq!(calculate_percentiles(&[], &[50, 90]), BTreeMap::new());
        assert_eq!(
            calculate_percentiles(&[4.0, 1.0, 3.0, 2.0], &[0, 50, 100]),
            BTreeMap::from([(0, 1.0), (50, 2.5), (100, 4.0)])
        );
    }

    #[test]
    fn test_empty_samples() {
        let samples: Vec<f64> = vec![];
        let stats = Statistics::from_samples(&samples);

        assert_eq!(stats.count, 0);
        assert_exact!(stats.mean, 0.0);
    }

    #[test]
    fn test_single_sample() {
        let samples = vec![42.0];
        let stats = Statistics::from_samples(&samples);

        assert_eq!(stats.count, 1);
        assert_exact!(stats.mean, 42.0);
        assert_exact!(stats.median, 42.0);
        assert_exact!(stats.min, 42.0);
        assert_exact!(stats.max, 42.0);
    }
}
