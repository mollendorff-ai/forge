//! Statistical comparison utilities.
//!
//! Provides functions for comparing stochastic outputs within tolerance.

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use serde::{Deserialize, Serialize};

/// Tolerance levels for statistical comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tolerance {
    /// Tolerance for mean comparison (fraction, e.g., 0.01 = 1%).
    pub mean: f64,
    /// Tolerance for standard deviation comparison.
    pub std: f64,
    /// Tolerance for percentile comparison.
    pub percentiles: f64,
    /// Minimum p-value for KS test.
    pub ks_pvalue: f64,
    /// Tolerance for CI bounds comparison.
    pub ci_bounds: f64,
}

impl Default for Tolerance {
    fn default() -> Self {
        Self {
            mean: 0.01,
            std: 0.05,
            percentiles: 0.02,
            ks_pvalue: 0.05,
            ci_bounds: 0.02,
        }
    }
}

impl Tolerance {
    /// Creates tolerance for deterministic outputs (stricter).
    #[must_use]
    pub const fn deterministic() -> Self {
        Self {
            mean: 0.001,
            std: 0.001,
            percentiles: 0.001,
            ks_pvalue: 0.05,
            ci_bounds: 0.001,
        }
    }

    /// Creates tolerance for stochastic outputs (looser).
    #[must_use]
    pub fn stochastic() -> Self {
        Self::default()
    }
}

/// Checks if actual value is within tolerance of expected value.
#[inline]
#[must_use]
pub fn within_tolerance(actual: f64, expected: f64, tolerance: f64) -> bool {
    if expected.abs() < f64::EPSILON {
        actual.abs() <= tolerance
    } else {
        let relative_diff = (actual - expected).abs() / expected.abs();
        relative_diff <= tolerance
    }
}

/// Calculates relative difference between two values.
#[inline]
#[must_use]
pub fn relative_difference(actual: f64, expected: f64) -> f64 {
    if expected.abs() < f64::EPSILON {
        actual.abs()
    } else {
        (actual - expected).abs() / expected.abs()
    }
}

/// A count as a number: exact up to 2^53, the nearest number above it. Built
/// from 32-bit halves, which convert without loss; this crate checks Forge
/// from outside and does not link it, so it does not share Forge's own
/// conversion.
fn count(n: usize) -> f64 {
    let bytes = n.to_le_bytes();
    let mut wide = [0_u8; 8];
    wide[..bytes.len()].copy_from_slice(&bytes);
    let [b0, b1, b2, b3, b4, b5, b6, b7] = wide;
    f64::from(u32::from_le_bytes([b4, b5, b6, b7])).mul_add(
        4_294_967_296.0,
        f64::from(u32::from_le_bytes([b0, b1, b2, b3])),
    )
}

/// Computes the Kolmogorov-Smirnov test statistic (D).
#[must_use]
pub fn ks_statistic(sample1: &[f64], sample2: &[f64]) -> f64 {
    if sample1.is_empty() || sample2.is_empty() {
        return 1.0;
    }

    let mut sorted1 = sample1.to_vec();
    let mut sorted2 = sample2.to_vec();
    sorted1.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    sorted2.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n1 = count(sorted1.len());
    let n2 = count(sorted2.len());

    let mut i = 0usize;
    let mut j = 0usize;
    let mut max_d = 0.0f64;

    while i < sorted1.len() && j < sorted2.len() {
        let ecdf1 = count(i + 1) / n1;
        let ecdf2 = count(j + 1) / n2;

        if sorted1[i] <= sorted2[j] {
            max_d = max_d.max((ecdf1 - (count(j) / n2)).abs());
            i += 1;
        } else {
            max_d = max_d.max(((count(i) / n1) - ecdf2).abs());
            j += 1;
        }
    }

    while i < sorted1.len() {
        let ecdf1 = count(i + 1) / n1;
        max_d = max_d.max((ecdf1 - 1.0).abs());
        i += 1;
    }
    while j < sorted2.len() {
        let ecdf2 = count(j + 1) / n2;
        max_d = max_d.max((1.0 - ecdf2).abs());
        j += 1;
    }

    max_d
}

/// Computes approximate p-value for KS test.
#[must_use]
pub fn ks_pvalue(d: f64, n1: usize, n2: usize) -> f64 {
    if d <= 0.0 {
        return 1.0;
    }
    if d >= 1.0 {
        return 0.0;
    }

    let n = (count(n1) * count(n2)) / (count(n1) + count(n2));
    let sqrt_n = n.sqrt();

    let lambda = (sqrt_n + 0.12 + 0.11 / sqrt_n) * d;
    let lambda_sq = lambda * lambda;

    let mut sum = 0.0;
    for k in 1..=100 {
        let k_f = f64::from(k);
        let term = (-2.0 * k_f * k_f * lambda_sq).exp();
        if k % 2 == 1 {
            sum += term;
        } else {
            sum -= term;
        }
        if term < 1e-10 {
            break;
        }
    }

    (2.0 * sum).clamp(0.0, 1.0)
}

/// Computes the two-sample KS test p-value.
#[must_use]
pub fn ks_test_pvalue(sample1: &[f64], sample2: &[f64]) -> f64 {
    let d = ks_statistic(sample1, sample2);
    ks_pvalue(d, sample1.len(), sample2.len())
}

/// Computes basic statistics for a sample.
#[must_use]
pub fn compute_stats(sample: &[f64]) -> (f64, f64) {
    if sample.is_empty() {
        return (0.0, 0.0);
    }

    let n = count(sample.len());
    let mean = sample.iter().sum::<f64>() / n;
    let variance = sample.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    let std = variance.sqrt();

    (mean, std)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_count_is_the_number_ieee_rounds_it_to() {
        for (n, number) in [
            (0, 0.0),
            (1, 1.0),
            (4_294_967_295, 4_294_967_295.0),
            (4_294_967_296, 4_294_967_296.0),
            (9_007_199_254_740_992, 9_007_199_254_740_992.0),
            // Above 2^53 the spacing is 2: a tie goes to the even neighbour
            (9_007_199_254_740_993, 9_007_199_254_740_992.0),
            (9_007_199_254_740_995, 9_007_199_254_740_996.0),
        ] {
            assert_eq!(count(n).to_bits(), f64::to_bits(number), "{n}");
        }
    }

    #[test]
    fn test_within_tolerance_pass() {
        assert!(within_tolerance(100.5, 100.0, 0.01));
    }

    #[test]
    fn test_within_tolerance_fail() {
        assert!(!within_tolerance(102.0, 100.0, 0.01));
    }

    #[test]
    fn test_tolerance_default() {
        let tol = Tolerance::default();
        assert!((tol.mean - 0.01).abs() < f64::EPSILON);
    }
}
