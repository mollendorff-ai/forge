//! Statistical Calculations for Monte Carlo Results
//!
//! Provides:
//! - Percentiles (P5, P10, P25, P50, P75, P90, P95)
//! - Summary statistics (mean, median, std dev, min, max)
//! - Probability thresholds (P(X > target))
//! - Histogram bin data

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
    /// Variance
    pub variance: f64,
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
    pub fn from_samples(samples: &[f64]) -> Self {
        if samples.is_empty() {
            return Self::empty();
        }

        let count = samples.len();
        let n = count as f64;

        // Sort for percentiles
        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Basic statistics
        let sum: f64 = samples.iter().sum();
        let mean = sum / n;

        let variance = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        let std_dev = variance.sqrt();

        // Min/Max
        let min = sorted[0];
        let max = sorted[count - 1];

        // Median
        let median = percentile_sorted(&sorted, 50.0);

        // Skewness and Kurtosis
        let (skewness, kurtosis) = if std_dev > 0.0 {
            let m3: f64 = samples
                .iter()
                .map(|x| ((x - mean) / std_dev).powi(3))
                .sum::<f64>()
                / n;
            let m4: f64 = samples
                .iter()
                .map(|x| ((x - mean) / std_dev).powi(4))
                .sum::<f64>()
                / n;
            (m3, m4 - 3.0) // Excess kurtosis (normal = 0)
        } else {
            (0.0, 0.0)
        };

        // Standard percentiles
        let mut percentiles = BTreeMap::new();
        for p in [5, 10, 25, 50, 75, 90, 95] {
            percentiles.insert(p, percentile_sorted(&sorted, p as f64));
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
    fn empty() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            median: 0.0,
            std_dev: 0.0,
            variance: 0.0,
            min: 0.0,
            max: 0.0,
            skewness: 0.0,
            kurtosis: 0.0,
            percentiles: BTreeMap::new(),
        }
    }

    /// Get a specific percentile
    pub fn percentile(&self, p: u8) -> Option<f64> {
        self.percentiles.get(&p).copied()
    }

    /// Calculate probability of exceeding a threshold
    pub fn probability_greater_than(&self, samples: &[f64], threshold: f64) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }
        let count = samples.iter().filter(|&&x| x > threshold).count();
        count as f64 / samples.len() as f64
    }

    /// Calculate probability of being less than a threshold
    pub fn probability_less_than(&self, samples: &[f64], threshold: f64) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }
        let count = samples.iter().filter(|&&x| x < threshold).count();
        count as f64 / samples.len() as f64
    }

    /// Get specific percentiles by list
    pub fn get_percentiles(&self, percentile_list: &[u8]) -> BTreeMap<u8, f64> {
        percentile_list
            .iter()
            .filter_map(|&p| self.percentiles.get(&p).map(|v| (p, *v)))
            .collect()
    }
}

/// Calculate percentile from sorted array (linear interpolation)
fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }

    let n = sorted.len() as f64;
    let rank = p / 100.0 * (n - 1.0);
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;

    if lower == upper || upper >= sorted.len() {
        sorted[lower.min(sorted.len() - 1)]
    } else {
        let frac = rank - lower as f64;
        sorted[lower] * (1.0 - frac) + sorted[upper] * frac
    }
}

/// Calculate additional percentiles from samples
pub fn calculate_percentiles(samples: &[f64], percentile_list: &[u8]) -> BTreeMap<u8, f64> {
    if samples.is_empty() {
        return BTreeMap::new();
    }

    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    percentile_list
        .iter()
        .map(|&p| (p, percentile_sorted(&sorted, p as f64)))
        .collect()
}

/// Histogram data for visualization
#[derive(Debug, Clone)]
pub struct Histogram {
    /// Bin edges (len = num_bins + 1)
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

        let min = samples.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = samples.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        // Handle case where all values are the same
        let (actual_min, actual_max) = if (max - min).abs() < 1e-10 {
            (min - 0.5, max + 0.5)
        } else {
            (min, max)
        };

        let bin_width = (actual_max - actual_min) / num_bins as f64;

        // Create bin edges
        let bin_edges: Vec<f64> = (0..=num_bins)
            .map(|i| actual_min + i as f64 * bin_width)
            .collect();

        // Count samples per bin
        let mut counts = vec![0usize; num_bins];
        for &sample in samples {
            let bin = ((sample - actual_min) / bin_width).floor() as usize;
            let bin = bin.min(num_bins - 1); // Handle edge case
            counts[bin] += 1;
        }

        // Calculate frequencies
        let total = samples.len() as f64;
        let frequencies: Vec<f64> = counts.iter().map(|&c| c as f64 / total).collect();

        Self {
            bin_edges,
            counts,
            frequencies,
        }
    }

    /// Get bin centers for plotting
    pub fn bin_centers(&self) -> Vec<f64> {
        if self.bin_edges.len() < 2 {
            return vec![];
        }
        self.bin_edges
            .windows(2)
            .map(|w| (w[0] + w[1]) / 2.0)
            .collect()
    }
}

/// Parse threshold string (e.g., "> 0", "< 100000", ">= 50")
pub fn parse_threshold(threshold: &str) -> Result<(String, f64), String> {
    let threshold = threshold.trim();

    let operators = [">=", "<=", ">", "<", "="];
    for op in operators {
        if threshold.starts_with(op) {
            let value_str = threshold[op.len()..].trim();
            let value: f64 = value_str
                .parse()
                .map_err(|_| format!("Invalid threshold value: {}", value_str))?;
            return Ok((op.to_string(), value));
        }
    }

    Err(format!(
        "Invalid threshold format: {}. Use '> 0', '< 100', etc.",
        threshold
    ))
}

/// Evaluate a threshold against samples
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

    count as f64 / samples.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statistics_basic() {
        let samples: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let stats = Statistics::from_samples(&samples);

        assert_eq!(stats.count, 100);
        assert!((stats.mean - 50.5).abs() < 0.01);
        assert!((stats.median - 50.5).abs() < 0.5);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 100.0);
    }

    #[test]
    fn test_percentiles() {
        let samples: Vec<f64> = (1..=100).map(|x| x as f64).collect();
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
    fn test_probability_threshold() {
        let samples: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let stats = Statistics::from_samples(&samples);

        // P(X > 50) should be approximately 0.5
        let p_gt_50 = stats.probability_greater_than(&samples, 50.0);
        assert!((p_gt_50 - 0.5).abs() < 0.01);

        // P(X < 25) should be approximately 0.24
        let p_lt_25 = stats.probability_less_than(&samples, 25.0);
        assert!((p_lt_25 - 0.24).abs() < 0.01);
    }

    #[test]
    fn test_histogram() {
        let samples: Vec<f64> = (0..100).map(|x| x as f64).collect();
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
        assert_eq!(val, 0.0);

        let (op, val) = parse_threshold("<= 100000").unwrap();
        assert_eq!(op, "<=");
        assert_eq!(val, 100000.0);

        let (op, val) = parse_threshold(">= -50.5").unwrap();
        assert_eq!(op, ">=");
        assert_eq!(val, -50.5);

        assert!(parse_threshold("invalid").is_err());
    }

    #[test]
    fn test_evaluate_threshold() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        assert_eq!(evaluate_threshold(&samples, ">", 3.0), 0.4); // 4, 5
        assert_eq!(evaluate_threshold(&samples, ">=", 3.0), 0.6); // 3, 4, 5
        assert_eq!(evaluate_threshold(&samples, "<", 3.0), 0.4); // 1, 2
        assert_eq!(evaluate_threshold(&samples, "<=", 3.0), 0.6); // 1, 2, 3
    }

    #[test]
    fn test_empty_samples() {
        let samples: Vec<f64> = vec![];
        let stats = Statistics::from_samples(&samples);

        assert_eq!(stats.count, 0);
        assert_eq!(stats.mean, 0.0);
    }

    #[test]
    fn test_single_sample() {
        let samples = vec![42.0];
        let stats = Statistics::from_samples(&samples);

        assert_eq!(stats.count, 1);
        assert_eq!(stats.mean, 42.0);
        assert_eq!(stats.median, 42.0);
        assert_eq!(stats.min, 42.0);
        assert_eq!(stats.max, 42.0);
    }
}
