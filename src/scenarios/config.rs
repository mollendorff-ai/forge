//! Scenario Analysis Configuration
//!
//! Handles parsing and validation of scenario definitions from YAML.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a single scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioDefinition {
    /// Probability of this scenario (0.0 to 1.0)
    pub probability: f64,
    /// Human-readable description
    #[serde(default)]
    pub description: String,
    /// Scalar overrides for this scenario
    #[serde(default)]
    pub scalars: HashMap<String, ScalarOverride>,
}

/// A scalar override can be a fixed value or a Monte Carlo distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScalarOverride {
    /// Fixed numeric value
    Value(f64),
    /// Formula (can include MC.* distributions)
    Formula { formula: String },
}

impl ScalarOverride {
    /// Get the value if this is a fixed value
    pub fn as_value(&self) -> Option<f64> {
        match self {
            ScalarOverride::Value(v) => Some(*v),
            ScalarOverride::Formula { .. } => None,
        }
    }

    /// Get the formula if this is a formula
    pub fn as_formula(&self) -> Option<&str> {
        match self {
            ScalarOverride::Value(_) => None,
            ScalarOverride::Formula { formula } => Some(formula),
        }
    }
}

/// Configuration for scenario analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenarioConfig {
    /// Map of scenario name to definition
    #[serde(default)]
    pub scenarios: HashMap<String, ScenarioDefinition>,
}

impl ScenarioConfig {
    /// Create a new empty configuration
    pub fn new() -> Self {
        Self {
            scenarios: HashMap::new(),
        }
    }

    /// Add a scenario to the configuration
    pub fn add_scenario(&mut self, name: &str, scenario: ScenarioDefinition) -> &mut Self {
        self.scenarios.insert(name.to_string(), scenario);
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.scenarios.is_empty() {
            return Err("No scenarios defined".to_string());
        }

        // Validate probabilities sum to 1.0 (within tolerance)
        let total_prob: f64 = self.scenarios.values().map(|s| s.probability).sum();
        const TOLERANCE: f64 = 0.001;
        if (total_prob - 1.0).abs() > TOLERANCE {
            return Err(format!(
                "Scenario probabilities must sum to 1.0, got {total_prob:.4}"
            ));
        }

        // Validate individual probabilities
        for (name, scenario) in &self.scenarios {
            if scenario.probability < 0.0 || scenario.probability > 1.0 {
                return Err(format!(
                    "Scenario '{}' probability must be between 0 and 1, got {}",
                    name, scenario.probability
                ));
            }
        }

        Ok(())
    }

    /// Get scenario names
    pub fn scenario_names(&self) -> Vec<&str> {
        self.scenarios
            .keys()
            .map(std::string::String::as_str)
            .collect()
    }

    /// Check if a scenario exists
    pub fn has_scenario(&self, name: &str) -> bool {
        self.scenarios.contains_key(name)
    }

    /// Get a scenario by name
    pub fn get_scenario(&self, name: &str) -> Option<&ScenarioDefinition> {
        self.scenarios.get(name)
    }
}

/// Builder pattern for ScenarioDefinition
impl ScenarioDefinition {
    /// Create a new scenario with given probability
    pub fn new(probability: f64) -> Self {
        Self {
            probability,
            description: String::new(),
            scalars: HashMap::new(),
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Add a scalar override with a fixed value
    pub fn with_scalar(mut self, name: &str, value: f64) -> Self {
        self.scalars
            .insert(name.to_string(), ScalarOverride::Value(value));
        self
    }

    /// Add a scalar override with a formula
    pub fn with_formula(mut self, name: &str, formula: &str) -> Self {
        self.scalars.insert(
            name.to_string(),
            ScalarOverride::Formula {
                formula: formula.to_string(),
            },
        );
        self
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_scenario_config_validation() {
        let mut config = ScenarioConfig::new();

        // Empty config should fail
        assert!(config.validate().is_err());

        // Add scenarios that sum to 1.0
        config.add_scenario(
            "base",
            ScenarioDefinition::new(0.5)
                .with_description("Base case")
                .with_scalar("revenue_growth", 0.05),
        );
        config.add_scenario(
            "bull",
            ScenarioDefinition::new(0.3)
                .with_description("Bull case")
                .with_scalar("revenue_growth", 0.15),
        );
        config.add_scenario(
            "bear",
            ScenarioDefinition::new(0.2)
                .with_description("Bear case")
                .with_scalar("revenue_growth", -0.10),
        );

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_probabilities_must_sum_to_one() {
        let mut config = ScenarioConfig::new();
        config.add_scenario("a", ScenarioDefinition::new(0.5));
        config.add_scenario("b", ScenarioDefinition::new(0.3));
        // Missing 0.2 - should fail

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("sum to 1.0"));
    }

    #[test]
    fn test_scalar_override_types() {
        let scenario = ScenarioDefinition::new(0.5)
            .with_scalar("fixed", 100.0)
            .with_formula("distribution", "=MC.Normal(1000, 100)");

        assert_eq!(
            scenario.scalars.get("fixed").unwrap().as_value(),
            Some(100.0)
        );
        assert_eq!(
            scenario.scalars.get("distribution").unwrap().as_formula(),
            Some("=MC.Normal(1000, 100)")
        );
    }
}
