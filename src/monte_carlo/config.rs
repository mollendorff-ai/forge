//! Monte Carlo configuration from YAML
//!
//! Parses the `monte_carlo`: section from YAML models.

use serde::{Deserialize, Serialize};

/// Monte Carlo simulation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloConfig {
    /// Enable/disable Monte Carlo simulation
    #[serde(default)]
    pub enabled: bool,

    /// Number of simulation iterations (default: 10000)
    #[serde(default = "default_iterations")]
    pub iterations: usize,

    /// Sampling method: "`monte_carlo`" or "`latin_hypercube`" (default)
    #[serde(default = "default_sampling")]
    pub sampling: String,

    /// Random seed for reproducibility (optional)
    #[serde(default)]
    pub seed: Option<u64>,

    /// Output variables to track
    #[serde(default)]
    pub outputs: Vec<OutputConfig>,

    /// Correlation specifications (Phase 3)
    #[serde(default)]
    pub correlations: Vec<CorrelationConfig>,
}

/// Output variable configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Variable name to track (e.g., "valuation.npv")
    pub variable: String,

    /// Percentiles to calculate (e.g., [10, 50, 90])
    #[serde(default = "default_percentiles")]
    pub percentiles: Vec<u8>,

    /// Probability threshold (e.g., "> 0", "< 100000")
    #[serde(default)]
    pub threshold: Option<String>,

    /// Custom label for output
    #[serde(default)]
    pub label: Option<String>,
}

/// Correlation specification between variables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationConfig {
    /// Variables to correlate (exactly 2)
    pub variables: Vec<String>,

    /// Correlation coefficient (-1.0 to 1.0)
    pub coefficient: f64,
}

impl Default for MonteCarloConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            iterations: default_iterations(),
            sampling: default_sampling(),
            seed: None,
            outputs: Vec::new(),
            correlations: Vec::new(),
        }
    }
}

impl MonteCarloConfig {
    /// Create a new Monte Carlo config with defaults
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder: set iterations
    #[must_use]
    pub const fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    /// Builder: set sampling method
    #[must_use]
    pub fn with_sampling(mut self, sampling: &str) -> Self {
        self.sampling = sampling.to_string();
        self
    }

    /// Builder: set seed
    #[must_use]
    pub const fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Builder: enable Monte Carlo
    #[must_use]
    pub const fn enabled(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Validate the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if iterations is zero or exceeds 1,000,000,
    /// sampling method is invalid, or correlation specs are malformed.
    pub fn validate(&self) -> Result<(), String> {
        if self.iterations == 0 {
            return Err("iterations must be > 0".to_string());
        }

        if self.iterations > 1_000_000 {
            return Err("iterations must be <= 1,000,000".to_string());
        }

        let valid_sampling = ["monte_carlo", "latin_hypercube"];
        if !valid_sampling.contains(&self.sampling.as_str()) {
            return Err(format!("sampling must be one of: {valid_sampling:?}"));
        }

        for corr in &self.correlations {
            if corr.variables.len() != 2 {
                return Err("correlation must specify exactly 2 variables".to_string());
            }
            if corr.coefficient < -1.0 || corr.coefficient > 1.0 {
                return Err("correlation coefficient must be between -1.0 and 1.0".to_string());
            }
        }

        Ok(())
    }
}

const fn default_iterations() -> usize {
    10_000
}

fn default_sampling() -> String {
    "latin_hypercube".to_string()
}

fn default_percentiles() -> Vec<u8> {
    vec![10, 50, 90]
}

// Financial math: exact float comparison validated against Excel/Gnumeric/R
#[allow(clippy::float_cmp)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MonteCarloConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.iterations, 10_000);
        assert_eq!(config.sampling, "latin_hypercube");
        assert!(config.seed.is_none());
    }

    #[test]
    fn test_builder_pattern() {
        let config = MonteCarloConfig::new()
            .enabled()
            .with_iterations(5000)
            .with_sampling("monte_carlo")
            .with_seed(12345);

        assert!(config.enabled);
        assert_eq!(config.iterations, 5000);
        assert_eq!(config.sampling, "monte_carlo");
        assert_eq!(config.seed, Some(12345));
    }

    #[test]
    fn test_validation() {
        let mut config = MonteCarloConfig::default();
        assert!(config.validate().is_ok());

        config.iterations = 0;
        assert!(config.validate().is_err());

        config.iterations = 10_000;
        config.sampling = "invalid".to_string();
        assert!(config.validate().is_err());

        config.sampling = "latin_hypercube".to_string();
        config.correlations.push(CorrelationConfig {
            variables: vec!["a".to_string()], // Only 1 variable - invalid
            coefficient: 0.5,
        });
        assert!(config.validate().is_err());

        config.correlations[0].variables.push("b".to_string());
        config.correlations[0].coefficient = 1.5; // Out of range
        assert!(config.validate().is_err());

        config.correlations[0].coefficient = 0.7;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_yaml_parsing() {
        let yaml = r#"
enabled: true
iterations: 5000
sampling: latin_hypercube
seed: 42
outputs:
  - variable: valuation.npv
    percentiles: [5, 50, 95]
    threshold: "> 0"
correlations:
  - variables: [revenue, costs]
    coefficient: -0.3
"#;
        let config: MonteCarloConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(config.enabled);
        assert_eq!(config.iterations, 5000);
        assert_eq!(config.seed, Some(42));
        assert_eq!(config.outputs.len(), 1);
        assert_eq!(config.outputs[0].variable, "valuation.npv");
        assert_eq!(config.outputs[0].percentiles, vec![5, 50, 95]);
        assert_eq!(config.correlations.len(), 1);
        assert_eq!(config.correlations[0].coefficient, -0.3);
    }
}
