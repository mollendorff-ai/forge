//! Tornado Diagram Configuration
//!
//! Handles parsing and validation of sensitivity analysis definitions.

use serde::{Deserialize, Serialize};

/// Configuration for an input variable range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputRange {
    /// Variable name
    pub name: String,
    /// Low value for sensitivity
    pub low: f64,
    /// High value for sensitivity
    pub high: f64,
    /// Base value (optional, uses model default if not specified)
    pub base: Option<f64>,
}

impl InputRange {
    /// Create a new input range
    pub fn new(name: &str, low: f64, high: f64) -> Self {
        Self {
            name: name.to_string(),
            low,
            high,
            base: None,
        }
    }

    /// Set the base value
    pub fn with_base(mut self, base: f64) -> Self {
        self.base = Some(base);
        self
    }

    /// Validate the range
    pub fn validate(&self) -> Result<(), String> {
        if self.low >= self.high {
            return Err(format!(
                "Input '{}': low ({}) must be less than high ({})",
                self.name, self.low, self.high
            ));
        }
        if let Some(base) = self.base {
            if base < self.low || base > self.high {
                return Err(format!(
                    "Input '{}': base ({}) must be between low ({}) and high ({})",
                    self.name, base, self.low, self.high
                ));
            }
        }
        Ok(())
    }

    /// Get the base value, defaulting to midpoint if not specified
    pub fn base_value(&self) -> f64 {
        self.base.unwrap_or(f64::midpoint(self.low, self.high))
    }
}

/// Configuration for tornado diagram
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TornadoConfig {
    /// Output variable to analyze
    pub output: String,
    /// Input variables with ranges
    #[serde(default)]
    pub inputs: Vec<InputRange>,
    /// Number of steps for sensitivity (default: 2 for low/high only)
    #[serde(default = "default_steps")]
    pub steps: usize,
}

fn default_steps() -> usize {
    2
}

impl TornadoConfig {
    /// Create a new configuration
    pub fn new(output: &str) -> Self {
        Self {
            output: output.to_string(),
            inputs: Vec::new(),
            steps: 2,
        }
    }

    /// Add an input variable
    pub fn with_input(mut self, input: InputRange) -> Self {
        self.inputs.push(input);
        self
    }

    /// Set the number of steps
    pub fn with_steps(mut self, steps: usize) -> Self {
        self.steps = steps;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.output.is_empty() {
            return Err("Output variable must be specified".to_string());
        }

        if self.inputs.is_empty() {
            return Err("At least one input variable must be specified".to_string());
        }

        if self.steps < 2 {
            return Err("Steps must be at least 2".to_string());
        }

        for input in &self.inputs {
            input.validate()?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = TornadoConfig::new("npv")
            .with_input(InputRange::new("revenue_growth", 0.02, 0.08))
            .with_input(InputRange::new("discount_rate", 0.08, 0.12));

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_empty_output_rejected() {
        let config = TornadoConfig::new("").with_input(InputRange::new("x", 0.0, 1.0));

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_no_inputs_rejected() {
        let config = TornadoConfig::new("output");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_range_rejected() {
        let config = TornadoConfig::new("output").with_input(InputRange::new("x", 1.0, 0.0)); // Low > high

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_base_value() {
        let input = InputRange::new("x", 0.0, 10.0);
        assert_eq!(input.base_value(), 5.0);

        let input_with_base = InputRange::new("x", 0.0, 10.0).with_base(3.0);
        assert_eq!(input_with_base.base_value(), 3.0);
    }
}
