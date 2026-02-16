//! Bootstrap Resampling Configuration
//!
//! Handles parsing and validation of bootstrap configuration.

use serde::{Deserialize, Serialize};

/// Statistic to compute from bootstrap samples
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BootstrapStatistic {
    /// Sample mean
    #[default]
    Mean,
    /// Sample median
    Median,
    /// Sample standard deviation
    Std,
    /// Sample variance
    Var,
    /// Specific percentile (requires additional parameter)
    Percentile,
    /// Sample minimum
    Min,
    /// Sample maximum
    Max,
}

/// Configuration for bootstrap resampling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    /// Number of bootstrap iterations
    #[serde(default = "default_iterations")]
    pub iterations: usize,
    /// Confidence levels for intervals (e.g., 0.90, 0.95, 0.99)
    #[serde(default = "default_confidence_levels")]
    pub confidence_levels: Vec<f64>,
    /// Random seed for reproducibility
    pub seed: Option<u64>,
    /// Historical data to resample from
    #[serde(default)]
    pub data: Vec<f64>,
    /// Statistic to compute
    #[serde(default)]
    pub statistic: BootstrapStatistic,
    /// Percentile value (if statistic is Percentile)
    #[serde(default = "default_percentile")]
    pub percentile_value: f64,
}

const fn default_iterations() -> usize {
    10000
}

fn default_confidence_levels() -> Vec<f64> {
    vec![0.90, 0.95, 0.99]
}

const fn default_percentile() -> f64 {
    50.0
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            iterations: 10000,
            confidence_levels: vec![0.90, 0.95, 0.99],
            seed: None,
            data: Vec::new(),
            statistic: BootstrapStatistic::Mean,
            percentile_value: 50.0,
        }
    }
}

impl BootstrapConfig {
    /// Create a new configuration
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the data
    #[must_use]
    pub fn with_data(mut self, data: Vec<f64>) -> Self {
        self.data = data;
        self
    }

    /// Set the number of iterations
    #[must_use]
    pub const fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    /// Set confidence levels
    #[must_use]
    pub fn with_confidence_levels(mut self, levels: Vec<f64>) -> Self {
        self.confidence_levels = levels;
        self
    }

    /// Set the seed
    #[must_use]
    pub const fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set the statistic
    #[must_use]
    pub const fn with_statistic(mut self, stat: BootstrapStatistic) -> Self {
        self.statistic = stat;
        self
    }

    /// Set percentile value
    #[must_use]
    pub const fn with_percentile(mut self, percentile: f64) -> Self {
        self.statistic = BootstrapStatistic::Percentile;
        self.percentile_value = percentile;
        self
    }

    /// Validate configuration
    ///
    /// # Errors
    ///
    /// Returns an error if data is empty or has fewer than 2 observations,
    /// iterations is zero, confidence levels are missing or out of range,
    /// or percentile value is out of range when using `Percentile` statistic.
    pub fn validate(&self) -> Result<(), String> {
        if self.data.is_empty() {
            return Err("Data cannot be empty".to_string());
        }

        if self.data.len() < 2 {
            return Err("Data must have at least 2 observations".to_string());
        }

        if self.iterations == 0 {
            return Err("Iterations must be positive".to_string());
        }

        if self.confidence_levels.is_empty() {
            return Err("At least one confidence level required".to_string());
        }

        for level in &self.confidence_levels {
            if *level <= 0.0 || *level >= 1.0 {
                return Err(format!("Confidence level {level} must be between 0 and 1"));
            }
        }

        if self.statistic == BootstrapStatistic::Percentile
            && (self.percentile_value <= 0.0 || self.percentile_value >= 100.0)
        {
            return Err(format!(
                "Percentile {} must be between 0 and 100",
                self.percentile_value
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = BootstrapConfig::new()
            .with_data(vec![1.0, 2.0, 3.0, 4.0, 5.0])
            .with_iterations(1000);

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_empty_data_rejected() {
        let config = BootstrapConfig::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_single_observation_rejected() {
        let config = BootstrapConfig::new().with_data(vec![1.0]);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_confidence_rejected() {
        let config = BootstrapConfig::new()
            .with_data(vec![1.0, 2.0, 3.0])
            .with_confidence_levels(vec![1.5]); // Invalid

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_percentile_rejected() {
        let config = BootstrapConfig::new()
            .with_data(vec![1.0, 2.0, 3.0])
            .with_percentile(150.0); // Invalid

        assert!(config.validate().is_err());
    }
}
