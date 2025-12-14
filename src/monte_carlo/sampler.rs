//! Sampling Methods for Monte Carlo Simulation
//!
//! Supports:
//! - Monte Carlo (pure random sampling)
//! - Latin Hypercube (stratified sampling, 5x faster convergence)

use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

/// Sampling method enumeration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SamplingMethod {
    /// Pure random Monte Carlo sampling
    MonteCarlo,
    /// Latin Hypercube Sampling (stratified, faster convergence)
    LatinHypercube,
}

impl SamplingMethod {
    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "monte_carlo" | "montecarlo" | "mc" => Ok(SamplingMethod::MonteCarlo),
            "latin_hypercube" | "latinhypercube" | "lhs" => Ok(SamplingMethod::LatinHypercube),
            _ => Err(format!(
                "Unknown sampling method: {}. Use 'monte_carlo' or 'latin_hypercube'",
                s
            )),
        }
    }
}

impl std::fmt::Display for SamplingMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SamplingMethod::MonteCarlo => write!(f, "monte_carlo"),
            SamplingMethod::LatinHypercube => write!(f, "latin_hypercube"),
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
    pub fn new(method: SamplingMethod, seed: Option<u64>) -> Self {
        let rng = match seed {
            Some(s) => StdRng::seed_from_u64(s),
            None => StdRng::from_os_rng(),
        };
        Self { method, rng }
    }

    /// Get the sampling method
    pub fn method(&self) -> SamplingMethod {
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
                let lower = i as f64 / n as f64;
                let upper = (i + 1) as f64 / n as f64;
                lower + self.rng.random::<f64>() * (upper - lower)
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
    pub fn rng_mut(&mut self) -> &mut StdRng {
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

        let n = samples.len() as f64;
        let mean = samples.iter().sum::<f64>() / n;
        let variance = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        let min = samples.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = samples.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

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

    #[test]
    fn test_monte_carlo_samples() {
        let mut sampler = Sampler::new(SamplingMethod::MonteCarlo, Some(12345));
        let samples = sampler.generate_uniform_samples(1000);

        assert_eq!(samples.len(), 1000);
        assert!(samples.iter().all(|&x| x >= 0.0 && x < 1.0));

        // Mean should be approximately 0.5
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!((mean - 0.5).abs() < 0.05);
    }

    #[test]
    fn test_latin_hypercube_samples() {
        let mut sampler = Sampler::new(SamplingMethod::LatinHypercube, Some(12345));
        let samples = sampler.generate_uniform_samples(1000);

        assert_eq!(samples.len(), 1000);
        assert!(samples.iter().all(|&x| x >= 0.0 && x < 1.0));

        // Mean should be approximately 0.5
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!((mean - 0.5).abs() < 0.02); // LHS should be closer to 0.5

        // LHS should have better coverage - check each stratum has exactly one sample
        let n = samples.len();
        let mut stratum_counts = vec![0; n];
        for &sample in &samples {
            let stratum = (sample * n as f64).floor() as usize;
            if stratum < n {
                stratum_counts[stratum] += 1;
            }
        }
        // Each stratum should have approximately 1 sample
        // (May not be exactly 1 due to floating point)
        let variance: f64 = stratum_counts
            .iter()
            .map(|&c| (c as f64 - 1.0).powi(2))
            .sum::<f64>()
            / n as f64;
        assert!(
            variance < 0.1,
            "LHS stratum counts should be uniform, variance: {}",
            variance
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
            let samples = sampler.generate_uniform_samples(n);
            let mean = samples.iter().sum::<f64>() / n as f64;
            mc_variances.push((mean - 0.5).powi(2));
        }
        let mc_avg_variance: f64 = mc_variances.iter().sum::<f64>() / mc_variances.len() as f64;

        // LHS variance
        let mut lhs_variances = Vec::new();
        for seed in 0..10 {
            let mut sampler = Sampler::new(SamplingMethod::LatinHypercube, Some(seed));
            let samples = sampler.generate_uniform_samples(n);
            let mean = samples.iter().sum::<f64>() / n as f64;
            lhs_variances.push((mean - 0.5).powi(2));
        }
        let lhs_avg_variance: f64 = lhs_variances.iter().sum::<f64>() / lhs_variances.len() as f64;

        // LHS should have lower variance
        assert!(
            lhs_avg_variance < mc_avg_variance,
            "LHS ({}) should have lower variance than MC ({})",
            lhs_avg_variance,
            mc_avg_variance
        );
    }

    #[test]
    fn test_seed_reproducibility() {
        let mut sampler1 = Sampler::new(SamplingMethod::LatinHypercube, Some(42));
        let samples1 = sampler1.generate_uniform_samples(100);

        let mut sampler2 = Sampler::new(SamplingMethod::LatinHypercube, Some(42));
        let samples2 = sampler2.generate_uniform_samples(100);

        assert_eq!(
            samples1, samples2,
            "Same seed should produce identical results"
        );
    }

    #[test]
    fn test_multidimensional_samples() {
        let mut sampler = Sampler::new(SamplingMethod::LatinHypercube, Some(12345));
        let samples = sampler.generate_uniform_samples_nd(100, 3);

        assert_eq!(samples.len(), 3);
        assert!(samples.iter().all(|dim| dim.len() == 100));
        assert!(samples
            .iter()
            .all(|dim| dim.iter().all(|&x| x >= 0.0 && x < 1.0)));
    }

    #[test]
    fn test_sample_stats() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = SampleStats::from_samples(&samples);

        assert_eq!(stats.mean, 3.0);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 5.0);
        assert!((stats.variance - 2.0).abs() < 0.001);
    }
}
