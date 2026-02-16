//! Monte Carlo Distribution Types
//!
//! Provides distribution sampling for FP&A modeling:
//! - MC.Normal(mean, stdev) - Symmetric variability
//! - MC.Triangular(min, mode, max) - Expert estimates
//! - MC.Uniform(min, max) - Complete uncertainty
//! - MC.PERT(min, mode, max) - Smooth project estimates
//! - MC.Lognormal(mean, stdev) - Non-negative values (Phase 4)

use rand::{Rng, RngExt};
use rand_distr::{Distribution as RandDistribution, Normal, Triangular, Uniform};
use std::fmt;

/// Distribution type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum DistributionType {
    /// Normal (Gaussian) distribution: MC.Normal(mean, stdev)
    Normal { mean: f64, stdev: f64 },

    /// Triangular distribution: MC.Triangular(min, mode, max)
    Triangular { min: f64, mode: f64, max: f64 },

    /// Uniform distribution: MC.Uniform(min, max)
    Uniform { min: f64, max: f64 },

    /// PERT distribution: MC.PERT(min, mode, max)
    /// Beta distribution shaped by min/mode/max, smoother than triangular
    PERT { min: f64, mode: f64, max: f64 },

    /// Lognormal distribution: MC.Lognormal(mean, stdev)
    /// For non-negative values like prices, revenue
    Lognormal { mean: f64, stdev: f64 },

    /// Discrete distribution: MC.Discrete({value: prob, ...})
    /// For scenario probabilities
    Discrete {
        values: Vec<f64>,
        probabilities: Vec<f64>,
    },
}

impl fmt::Display for DistributionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal { mean, stdev } => {
                write!(f, "MC.Normal({mean}, {stdev})")
            },
            Self::Triangular { min, mode, max } => {
                write!(f, "MC.Triangular({min}, {mode}, {max})")
            },
            Self::Uniform { min, max } => {
                write!(f, "MC.Uniform({min}, {max})")
            },
            Self::PERT { min, mode, max } => {
                write!(f, "MC.PERT({min}, {mode}, {max})")
            },
            Self::Lognormal { mean, stdev } => {
                write!(f, "MC.Lognormal({mean}, {stdev})")
            },
            Self::Discrete {
                values,
                probabilities,
            } => {
                write!(f, "MC.Discrete({values:?}, {probabilities:?})")
            },
        }
    }
}

/// A distribution that can be sampled
pub struct Distribution {
    pub dist_type: DistributionType,
}

impl Distribution {
    /// Create a Normal distribution
    ///
    /// # Errors
    ///
    /// Returns an error if `stdev` is not positive.
    pub fn normal(mean: f64, stdev: f64) -> Result<Self, String> {
        if stdev <= 0.0 {
            return Err("Normal distribution stdev must be > 0".to_string());
        }
        Ok(Self {
            dist_type: DistributionType::Normal { mean, stdev },
        })
    }

    /// Create a Triangular distribution
    ///
    /// # Errors
    ///
    /// Returns an error if `min >= max` or `mode` is outside `[min, max]`.
    pub fn triangular(min: f64, mode: f64, max: f64) -> Result<Self, String> {
        if min >= max {
            return Err("Triangular distribution requires min < max".to_string());
        }
        if mode < min || mode > max {
            return Err("Triangular distribution requires min <= mode <= max".to_string());
        }
        Ok(Self {
            dist_type: DistributionType::Triangular { min, mode, max },
        })
    }

    /// Create a Uniform distribution
    ///
    /// # Errors
    ///
    /// Returns an error if `min >= max`.
    pub fn uniform(min: f64, max: f64) -> Result<Self, String> {
        if min >= max {
            return Err("Uniform distribution requires min < max".to_string());
        }
        Ok(Self {
            dist_type: DistributionType::Uniform { min, max },
        })
    }

    /// Create a PERT distribution (Beta-PERT)
    ///
    /// # Errors
    ///
    /// Returns an error if `min >= max` or `mode` is outside `[min, max]`.
    pub fn pert(min: f64, mode: f64, max: f64) -> Result<Self, String> {
        if min >= max {
            return Err("PERT distribution requires min < max".to_string());
        }
        if mode < min || mode > max {
            return Err("PERT distribution requires min <= mode <= max".to_string());
        }
        Ok(Self {
            dist_type: DistributionType::PERT { min, mode, max },
        })
    }

    /// Create a Lognormal distribution
    ///
    /// # Errors
    ///
    /// Returns an error if `stdev` is not positive or `mean` is not positive.
    pub fn lognormal(mean: f64, stdev: f64) -> Result<Self, String> {
        if stdev <= 0.0 {
            return Err("Lognormal distribution stdev must be > 0".to_string());
        }
        if mean <= 0.0 {
            return Err("Lognormal distribution mean must be > 0".to_string());
        }
        Ok(Self {
            dist_type: DistributionType::Lognormal { mean, stdev },
        })
    }

    /// Create a Discrete distribution
    ///
    /// # Errors
    ///
    /// Returns an error if `values` and `probabilities` have different lengths,
    /// are empty, probabilities don't sum to 1.0, or any probability is negative.
    pub fn discrete(values: Vec<f64>, probabilities: Vec<f64>) -> Result<Self, String> {
        if values.len() != probabilities.len() {
            return Err(
                "Discrete distribution requires equal number of values and probabilities"
                    .to_string(),
            );
        }
        if values.is_empty() {
            return Err("Discrete distribution requires at least one value".to_string());
        }
        let sum: f64 = probabilities.iter().sum();
        if (sum - 1.0).abs() > 0.001 {
            return Err(format!(
                "Discrete distribution probabilities must sum to 1.0 (got {sum})"
            ));
        }
        if probabilities.iter().any(|&p| p < 0.0) {
            return Err("Discrete distribution probabilities must be >= 0".to_string());
        }
        Ok(Self {
            dist_type: DistributionType::Discrete {
                values,
                probabilities,
            },
        })
    }

    /// Sample a single value from the distribution
    ///
    /// # Panics
    ///
    /// Panics if the underlying `rand_distr` distribution cannot be constructed
    /// (should not happen with validated parameters).
    pub fn sample<R: Rng>(&self, rng: &mut R) -> f64 {
        match &self.dist_type {
            DistributionType::Normal { mean, stdev } => {
                let dist = Normal::new(*mean, *stdev).unwrap();
                dist.sample(rng)
            },
            DistributionType::Triangular { min, mode, max } => {
                let dist = Triangular::new(*min, *max, *mode).unwrap();
                dist.sample(rng)
            },
            DistributionType::Uniform { min, max } => {
                let dist = Uniform::new(*min, *max).unwrap();
                dist.sample(rng)
            },
            DistributionType::PERT { min, mode, max } => {
                // PERT is a scaled Beta distribution
                // Shape parameters based on PERT formula
                sample_pert(rng, *min, *mode, *max)
            },
            DistributionType::Lognormal { mean, stdev } => sample_lognormal(rng, *mean, *stdev),
            DistributionType::Discrete {
                values,
                probabilities,
            } => sample_discrete(rng, values, probabilities),
        }
    }

    /// Sample multiple values from the distribution
    pub fn sample_n<R: Rng>(&self, rng: &mut R, n: usize) -> Vec<f64> {
        (0..n).map(|_| self.sample(rng)).collect()
    }

    /// Get theoretical mean of the distribution
    #[must_use]
    pub fn theoretical_mean(&self) -> f64 {
        match &self.dist_type {
            DistributionType::Normal { mean, .. } => *mean,
            DistributionType::Triangular { min, mode, max } => (min + mode + max) / 3.0,
            DistributionType::Uniform { min, max } => (min + max) / 2.0,
            DistributionType::PERT { min, mode, max } => (min + 4.0 * mode + max) / 6.0,
            DistributionType::Lognormal { mean, stdev } => {
                // For lognormal, mean of log-space params
                let variance = stdev * stdev;
                let mu = (mean * mean / (mean * mean + variance).sqrt()).ln();
                let sigma_sq = (variance / (mean * mean)).ln_1p();
                (mu + sigma_sq / 2.0).exp()
            },
            DistributionType::Discrete {
                values,
                probabilities,
            } => values
                .iter()
                .zip(probabilities.iter())
                .map(|(v, p)| v * p)
                .sum(),
        }
    }

    /// Get theoretical variance of the distribution
    #[must_use]
    pub fn theoretical_variance(&self) -> f64 {
        match &self.dist_type {
            DistributionType::Normal { stdev, .. } => stdev * stdev,
            DistributionType::Triangular { min, mode, max } => {
                // Correct formula: Var = (a^2 + b^2 + c^2 - ab - ac - bc) / 18
                #[allow(clippy::suspicious_operation_groupings)]
                let numerator =
                    min * min + mode * mode + max * max - min * mode - min * max - mode * max;
                numerator / 18.0
            },
            DistributionType::Uniform { min, max } => (max - min).powi(2) / 12.0,
            DistributionType::PERT { min, mode, max } => {
                let mean = (min + 4.0 * mode + max) / 6.0;
                // PERT variance approximation
                ((max - min) / 6.0).powi(2) * (1.0 + (mode - mean).abs() / (max - min))
            },
            DistributionType::Lognormal { mean, stdev } => {
                let variance = stdev * stdev;
                ((variance / (mean * mean)).ln() + 1.0).exp_m1()
            },
            DistributionType::Discrete {
                values,
                probabilities,
            } => {
                let mean = self.theoretical_mean();
                values
                    .iter()
                    .zip(probabilities.iter())
                    .map(|(v, p)| p * (v - mean).powi(2))
                    .sum()
            },
        }
    }
}

/// Sample from PERT (Beta-PERT) distribution
fn sample_pert<R: Rng>(rng: &mut R, min: f64, mode: f64, max: f64) -> f64 {
    // PERT uses Beta distribution with shape parameters
    // alpha = 1 + 4 * (mode - min) / (max - min)
    // beta = 1 + 4 * (max - mode) / (max - min)
    let range = max - min;
    if range <= 0.0 {
        return mode;
    }

    let alpha = 1.0 + 4.0 * (mode - min) / range;
    let beta = 1.0 + 4.0 * (max - mode) / range;

    // Sample from Beta using rejection method or gamma ratio
    let x = sample_beta(rng, alpha, beta);

    // Scale to [min, max]
    min + x * range
}

/// Sample from Beta distribution using gamma ratio method
fn sample_beta<R: Rng>(rng: &mut R, alpha: f64, beta: f64) -> f64 {
    use rand_distr::Gamma;

    let gamma_a = Gamma::new(alpha, 1.0).unwrap();
    let gamma_b = Gamma::new(beta, 1.0).unwrap();

    let x: f64 = gamma_a.sample(rng);
    let y: f64 = gamma_b.sample(rng);

    x / (x + y)
}

/// Sample from Lognormal distribution
fn sample_lognormal<R: Rng>(rng: &mut R, mean: f64, stdev: f64) -> f64 {
    // Convert mean/stdev of lognormal to underlying normal parameters
    let variance = stdev * stdev;
    let mu = (mean * mean / mean.mul_add(mean, variance).sqrt()).ln();
    let sigma = (variance / (mean * mean)).ln_1p().sqrt();

    let normal = Normal::new(mu, sigma).unwrap();
    let z: f64 = normal.sample(rng);
    z.exp()
}

/// Sample from Discrete distribution
fn sample_discrete<R: Rng>(rng: &mut R, values: &[f64], probabilities: &[f64]) -> f64 {
    let u: f64 = rng.random();
    let mut cumulative = 0.0;

    for (value, prob) in values.iter().zip(probabilities.iter()) {
        cumulative += prob;
        if u <= cumulative {
            return *value;
        }
    }

    // Fallback to last value (should not reach here with valid probabilities)
    *values.last().unwrap_or(&0.0)
}

/// Parse a distribution from formula string (e.g., "`MC.Normal(100, 15)`")
///
/// # Errors
///
/// Returns an error if the formula is malformed, uses an unknown distribution type,
/// has the wrong number of arguments, or has invalid parameter values.
pub fn parse_distribution(formula: &str) -> Result<Distribution, String> {
    let formula = formula.trim();

    // Check for MC. prefix
    if !formula.starts_with("MC.") {
        return Err(format!(
            "Distribution must start with 'MC.' prefix: {formula}"
        ));
    }

    let without_prefix = &formula[3..];

    // Parse function name and arguments
    let paren_pos = without_prefix
        .find('(')
        .ok_or_else(|| format!("Missing opening parenthesis: {formula}"))?;

    let func_name = &without_prefix[..paren_pos];
    let args_str = without_prefix[paren_pos + 1..]
        .strip_suffix(')')
        .ok_or_else(|| format!("Missing closing parenthesis: {formula}"))?;

    // Parse arguments
    let args: Vec<f64> = args_str
        .split(',')
        .map(|s| s.trim().parse::<f64>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Invalid argument: {e}"))?;

    match func_name.to_uppercase().as_str() {
        "NORMAL" => {
            if args.len() != 2 {
                return Err("MC.Normal requires 2 arguments: (mean, stdev)".to_string());
            }
            Distribution::normal(args[0], args[1])
        },
        "TRIANGULAR" => {
            if args.len() != 3 {
                return Err("MC.Triangular requires 3 arguments: (min, mode, max)".to_string());
            }
            Distribution::triangular(args[0], args[1], args[2])
        },
        "UNIFORM" => {
            if args.len() != 2 {
                return Err("MC.Uniform requires 2 arguments: (min, max)".to_string());
            }
            Distribution::uniform(args[0], args[1])
        },
        "PERT" => {
            if args.len() != 3 {
                return Err("MC.PERT requires 3 arguments: (min, mode, max)".to_string());
            }
            Distribution::pert(args[0], args[1], args[2])
        },
        "LOGNORMAL" => {
            if args.len() != 2 {
                return Err("MC.Lognormal requires 2 arguments: (mean, stdev)".to_string());
            }
            Distribution::lognormal(args[0], args[1])
        },
        _ => Err(format!("Unknown distribution type: {func_name}")),
    }
}

// Financial math: exact float comparison validated against Excel/Gnumeric/R
#[allow(clippy::float_cmp)]
#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn seeded_rng() -> StdRng {
        StdRng::seed_from_u64(12345)
    }

    #[test]
    fn test_normal_distribution() {
        let dist = Distribution::normal(100.0, 15.0).unwrap();
        let mut rng = seeded_rng();

        let samples: Vec<f64> = dist.sample_n(&mut rng, 10000);
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        let variance: f64 =
            samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / samples.len() as f64;

        // Mean should be within 2% of theoretical
        assert!((mean - 100.0).abs() < 2.0, "Mean {mean} not close to 100");
        // Variance should be within 10% of theoretical (15^2 = 225)
        assert!(
            (variance - 225.0).abs() < 30.0,
            "Variance {variance} not close to 225"
        );
    }

    #[test]
    fn test_triangular_distribution() {
        let dist = Distribution::triangular(0.0, 5.0, 10.0).unwrap();
        let mut rng = seeded_rng();

        let samples: Vec<f64> = dist.sample_n(&mut rng, 10000);
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;

        // Theoretical mean = (0 + 5 + 10) / 3 = 5
        assert!((mean - 5.0).abs() < 0.2, "Mean {mean} not close to 5");

        // All samples should be within bounds
        assert!(samples.iter().all(|&x| (0.0..=10.0).contains(&x)));
    }

    #[test]
    fn test_uniform_distribution() {
        let dist = Distribution::uniform(10.0, 20.0).unwrap();
        let mut rng = seeded_rng();

        let samples: Vec<f64> = dist.sample_n(&mut rng, 10000);
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;

        // Theoretical mean = (10 + 20) / 2 = 15
        assert!((mean - 15.0).abs() < 0.2, "Mean {mean} not close to 15");

        // All samples should be within bounds
        assert!(samples.iter().all(|&x| (10.0..20.0).contains(&x)));
    }

    #[test]
    fn test_pert_distribution() {
        let dist = Distribution::pert(0.0, 3.0, 10.0).unwrap();
        let mut rng = seeded_rng();

        let samples: Vec<f64> = dist.sample_n(&mut rng, 10000);
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;

        // Theoretical mean = (0 + 4*3 + 10) / 6 = 22/6 ≈ 3.67
        let theoretical = (4.0f64.mul_add(3.0, 0.0) + 10.0) / 6.0;
        assert!(
            (mean - theoretical).abs() < 0.3,
            "Mean {mean} not close to {theoretical}"
        );

        // All samples should be within bounds
        assert!(samples.iter().all(|&x| (0.0..=10.0).contains(&x)));
    }

    #[test]
    fn test_parse_distribution() {
        let dist = parse_distribution("MC.Normal(100, 15)").unwrap();
        assert!(matches!(
            dist.dist_type,
            DistributionType::Normal {
                mean: 100.0,
                stdev: 15.0
            }
        ));

        let dist = parse_distribution("MC.Triangular(0, 5, 10)").unwrap();
        assert!(matches!(
            dist.dist_type,
            DistributionType::Triangular {
                min: 0.0,
                mode: 5.0,
                max: 10.0
            }
        ));

        let dist = parse_distribution("MC.Uniform(10, 20)").unwrap();
        assert!(matches!(
            dist.dist_type,
            DistributionType::Uniform {
                min: 10.0,
                max: 20.0
            }
        ));

        let dist = parse_distribution("MC.PERT(0, 3, 10)").unwrap();
        assert!(matches!(
            dist.dist_type,
            DistributionType::PERT {
                min: 0.0,
                mode: 3.0,
                max: 10.0
            }
        ));
    }

    #[test]
    fn test_parse_distribution_errors() {
        assert!(parse_distribution("Normal(100, 15)").is_err()); // Missing MC. prefix
        assert!(parse_distribution("MC.Normal(100)").is_err()); // Wrong arg count
        assert!(parse_distribution("MC.Normal(100, -5)").is_err()); // Negative stdev
        assert!(parse_distribution("MC.Triangular(10, 5, 0)").is_err()); // min > max
        assert!(parse_distribution("MC.Unknown(1, 2)").is_err()); // Unknown type
    }

    #[test]
    fn test_seed_reproducibility() {
        let dist = Distribution::normal(100.0, 15.0).unwrap();

        let mut rng1 = StdRng::seed_from_u64(42);
        let samples1: Vec<f64> = dist.sample_n(&mut rng1, 100);

        let mut rng2 = StdRng::seed_from_u64(42);
        let samples2: Vec<f64> = dist.sample_n(&mut rng2, 100);

        assert_eq!(
            samples1, samples2,
            "Same seed should produce identical results"
        );
    }

    #[test]
    fn test_theoretical_values() {
        let normal = Distribution::normal(100.0, 15.0).unwrap();
        assert_eq!(normal.theoretical_mean(), 100.0);
        assert_eq!(normal.theoretical_variance(), 225.0);

        let uniform = Distribution::uniform(10.0, 20.0).unwrap();
        assert_eq!(uniform.theoretical_mean(), 15.0);
        assert!((uniform.theoretical_variance() - 100.0 / 12.0).abs() < 0.001);

        let triangular = Distribution::triangular(0.0, 5.0, 10.0).unwrap();
        assert_eq!(triangular.theoretical_mean(), 5.0);
    }
}
