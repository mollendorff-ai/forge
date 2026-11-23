//! Sampling Methods for Monte Carlo Simulation
//!
//! Supports:
//! - Monte Carlo (pure random sampling)
//! - Latin Hypercube (stratified sampling, 5x faster convergence)

use crate::core::precision;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

/// Sampling method enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingMethod {
    /// Pure random Monte Carlo sampling
    MonteCarlo,
    /// Latin Hypercube Sampling (stratified, faster convergence)
    LatinHypercube,
}

impl std::str::FromStr for SamplingMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "monte_carlo" | "montecarlo" | "mc" => Ok(Self::MonteCarlo),
            "latin_hypercube" | "latinhypercube" | "lhs" => Ok(Self::LatinHypercube),
            _ => Err(format!(
                "Unknown sampling method: {s}. Use 'monte_carlo' or 'latin_hypercube'"
            )),
        }
    }
}

impl std::fmt::Display for SamplingMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MonteCarlo => write!(f, "monte_carlo"),
            Self::LatinHypercube => write!(f, "latin_hypercube"),
        }
    }
}

/// Sampler for generating random values
pub struct Sampler {
    method: SamplingMethod,
    rng: StdRng,
}

impl Sampler {
    /// Create a new sampler with the given method and optional seed
    #[must_use]
    pub fn new(method: SamplingMethod, seed: Option<u64>) -> Self {
        let rng = seed.map_or_else(|| StdRng::from_rng(&mut rand::rng()), StdRng::seed_from_u64);
        Self { method, rng }
    }

    /// Get the sampling method
    #[must_use]
    pub const fn method(&self) -> SamplingMethod {
        self.method
    }

    /// Generate n uniform samples in [0, 1)
    /// For Monte Carlo: pure random
    /// For Latin Hypercube: stratified sampling
    pub fn generate_uniform_samples(&mut self, n: usize) -> Vec<f64> {
        match self.method {
            SamplingMethod::MonteCarlo => self.monte_carlo_samples(n),
            SamplingMethod::LatinHypercube => self.latin_hypercube_samples(n),
        }
    }

    /// Generate samples for multiple dimensions
    /// Returns n samples for each of d dimensions
    pub fn generate_uniform_samples_nd(&mut self, n: usize, d: usize) -> Vec<Vec<f64>> {
        match self.method {
            SamplingMethod::MonteCarlo => (0..d).map(|_| self.monte_carlo_samples(n)).collect(),
            SamplingMethod::LatinHypercube => self.latin_hypercube_samples_nd(n, d),
        }
    }

    /// Pure random Monte Carlo samples
    fn monte_carlo_samples(&mut self, n: usize) -> Vec<f64> {
        (0..n).map(|_| self.rng.random::<f64>()).collect()
    }

    /// Latin Hypercube samples for 1 dimension
    fn latin_hypercube_samples(&mut self, n: usize) -> Vec<f64> {
        // Divide [0, 1) into n equal intervals
        // Sample one value from each interval
        // Then shuffle
        let mut samples: Vec<f64> = (0..n)
            .map(|i| {
                let lower = precision::count(i) / precision::count(n);
                let upper = precision::count(i + 1) / precision::count(n);
                self.rng.random::<f64>().mul_add(upper - lower, lower)
            })
            .collect();

        // Fisher-Yates shuffle
        for i in (1..n).rev() {
            let j = self.rng.random_range(0..=i);
            samples.swap(i, j);
        }

        samples
    }

    /// Latin Hypercube samples for d dimensions
    /// Each dimension is independently stratified, then shuffled
    fn latin_hypercube_samples_nd(&mut self, n: usize, d: usize) -> Vec<Vec<f64>> {
        (0..d).map(|_| self.latin_hypercube_samples(n)).collect()
    }

    /// Get mutable reference to RNG for custom sampling
    pub const fn rng_mut(&mut self) -> &mut StdRng {
        &mut self.rng
    }
}

/// Statistics about a sample set
#[derive(Debug, Clone)]
pub struct SampleStats {
    pub mean: f64,
    pub variance: f64,
    pub min: f64,
    pub max: f64,
}

impl SampleStats {
    /// Calculate statistics from samples
    pub fn from_samples(samples: &[f64]) -> Self {
        if samples.is_empty() {
            return Self {
                mean: 0.0,
                variance: 0.0,
                min: 0.0,
                max: 0.0,
            };
        }

        let n = precision::count(samples.len());
        let mean = samples.iter().sum::<f64>() / n;
        let variance = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        let min = samples.iter().copied().fold(f64::INFINITY, f64::min);
        let max = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        Self {
            mean,
            variance,
            min,
            max,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::ToPrimitive;
    use std::str::FromStr;

    #[test]
    fn test_sampling_method_from_str() {
        assert_eq!(
            SamplingMethod::from_str("monte_carlo").unwrap(),
            SamplingMethod::MonteCarlo
        );
        assert_eq!(
            SamplingMethod::from_str("latin_hypercube").unwrap(),
            SamplingMethod::LatinHypercube
        );
        assert_eq!(
            SamplingMethod::from_str("LHS").unwrap(),
            SamplingMethod::LatinHypercube
        );
        assert!(SamplingMethod::from_str("invalid").is_err());
    }

    /// A method is written by its name, which `from_str` reads back.
    #[test]
    fn a_method_is_displayed_as_the_name_that_reads_back() {
        for (method, name) in [
            (SamplingMethod::MonteCarlo, "monte_carlo"),
            (SamplingMethod::LatinHypercube, "latin_hypercube"),
        ] {
            assert_eq!(method.to_string(), name);
            assert_eq!(SamplingMethod::from_str(name).unwrap(), method);
        }
    }

    /// The sampler says its method, and its generator is the one the samples
    /// come from: the same seed draws the same number as a sample.
    #[test]
    fn the_sampler_says_its_method_and_lends_its_generator() {
        let mut sampler = Sampler::new(SamplingMethod::LatinHypercube, Some(7));
        assert_eq!(sampler.method(), SamplingMethod::LatinHypercube);
        let mut other = Sampler::new(SamplingMethod::MonteCarlo, Some(7));
        let drawn = sampler.rng_mut().random::<f64>();
        assert!((drawn - other.generate_uniform_samples(1)[0]).abs() < 1e-15);
    }

    /// No samples have no statistics but zeros, not NaN.
    #[test]
    fn no_samples_have_zero_statistics() {
        let stats = SampleStats::from_samples(&[]);
        assert!(stats.mean.abs() < 1e-15 && stats.variance.abs() < 1e-15);
        assert!(stats.min.abs() < 1e-15 && stats.max.abs() < 1e-15);
    }

    #[test]
    fn test_monte_carlo_samples() {
        let mut sampler = Sampler::new(SamplingMethod::MonteCarlo, Some(12345));
        let draws = sampler.generate_uniform_samples(1000);

        assert_eq!(draws.len(), 1000);
        assert!(draws.iter().all(|&x| (0.0..1.0).contains(&x)));

        // Mean should be approximately 0.5
        let mean = draws.iter().sum::<f64>() / precision::count(draws.len());
        assert!((mean - 0.5).abs() < 0.05);
    }

    #[test]
    fn test_latin_hypercube_samples() {
        let mut sampler = Sampler::new(SamplingMethod::LatinHypercube, Some(12345));
        let draws = sampler.generate_uniform_samples(1000);

        assert_eq!(draws.len(), 1000);
        assert!(draws.iter().all(|&x| (0.0..1.0).contains(&x)));

        // Mean should be approximately 0.5
        let mean = draws.iter().sum::<f64>() / precision::count(draws.len());
        assert!((mean - 0.5).abs() < 0.02); // LHS should be closer to 0.5

        // LHS should have better coverage - check each stratum has exactly one sample
        let n = draws.len();
        let mut stratum_counts = vec![0; n];
        for &sample in &draws {
            let stratum = (sample * precision::count(n))
                .floor()
                .to_usize()
                .expect("a draw in 0..1");
            stratum_counts[stratum] += 1;
        }
        // Each stratum should have approximately 1 sample
        // (May not be exactly 1 due to floating point)
        let variance: f64 = stratum_counts
            .iter()
            .map(|&c| (f64::from(c) - 1.0).powi(2))
            .sum::<f64>()
            / precision::count(n);
        assert!(
            variance < 0.1,
            "LHS stratum counts should be uniform, variance: {variance}"
        );
    }

    #[test]
    fn test_lhs_better_convergence() {
        // LHS should have lower variance for the same sample size
        let n = 1000;

        // Monte Carlo variance
        let mut mc_variances = Vec::new();
        for seed in 0..10 {
            let mut sampler = Sampler::new(SamplingMethod::MonteCarlo, Some(seed));
            let draws = sampler.generate_uniform_samples(n);
            let mean = draws.iter().sum::<f64>() / precision::count(n);
            mc_variances.push((mean - 0.5).powi(2));
        }
        let mc_avg_variance: f64 =
            mc_variances.iter().sum::<f64>() / precision::count(mc_variances.len());

        // LHS variance
        let mut lhs_variances = Vec::new();
        for seed in 0..10 {
            let mut sampler = Sampler::new(SamplingMethod::LatinHypercube, Some(seed));
            let draws = sampler.generate_uniform_samples(n);
            let mean = draws.iter().sum::<f64>() / precision::count(n);
            lhs_variances.push((mean - 0.5).powi(2));
        }
        let lhs_avg_variance: f64 =
            lhs_variances.iter().sum::<f64>() / precision::count(lhs_variances.len());

        // LHS should have lower variance
        assert!(
            lhs_avg_variance < mc_avg_variance,
            "LHS ({lhs_avg_variance}) should have lower variance than MC ({mc_avg_variance})"
        );
    }

    #[test]
    fn test_seed_reproducibility() {
        let mut first_sampler = Sampler::new(SamplingMethod::LatinHypercube, Some(42));
        let first_draws = first_sampler.generate_uniform_samples(100);

        let mut second_sampler = Sampler::new(SamplingMethod::LatinHypercube, Some(42));
        let second_draws = second_sampler.generate_uniform_samples(100);

        assert_eq!(
            first_draws, second_draws,
            "Same seed should produce identical results"
        );
    }

    #[test]
    fn test_multidimensional_samples() {
        let mut sampler = Sampler::new(SamplingMethod::LatinHypercube, Some(12345));
        let draws = sampler.generate_uniform_samples_nd(100, 3);

        assert_eq!(draws.len(), 3);
        assert!(draws.iter().all(|dim| dim.len() == 100));
        assert!(draws
            .iter()
            .all(|dim| dim.iter().all(|&x| (0.0..1.0).contains(&x))));
    }

    #[test]
    fn test_sample_stats() {
        let draws = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = SampleStats::from_samples(&draws);

        assert_exact!(stats.mean, 3.0);
        assert_exact!(stats.min, 1.0);
        assert_exact!(stats.max, 5.0);
        assert!((stats.variance - 2.0).abs() < 0.001);
    }
}
