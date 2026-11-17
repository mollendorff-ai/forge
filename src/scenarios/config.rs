//! Scenario Analysis Configuration
//!
//! Handles parsing and validation of scenario definitions from YAML.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Configuration for a single scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDefinition {
    /// Probability of this scenario (0.0 to 1.0)
    pub probability: f64,
    /// Human-readable description
    #[serde(default)]
    pub description: String,
    /// Scalar overrides for this scenario
    #[serde(default)]
    pub scalars: BTreeMap<String, ScalarOverride>,
}

/// A scalar override can be a fixed value or a Monte Carlo distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScalarOverride {
    /// Fixed numeric value
    Value(f64),
    /// Formula (can include MC.* distributions)
    Formula { formula: String },
    /// A date, for a date input (ADR-060)
    Date(String),
}

impl ScalarOverride {
    /// Get the value if this is a fixed value
    #[must_use]
    pub const fn as_value(&self) -> Option<f64> {
        match self {
            Self::Value(v) => Some(*v),
            Self::Formula { .. } | Self::Date(_) => None,
        }
    }

    /// Get the formula if this is a formula
    #[must_use]
    pub fn as_formula(&self) -> Option<&str> {
        match self {
            Self::Value(_) | Self::Date(_) => None,
            Self::Formula { formula } => Some(formula),
        }
    }
}

/// Configuration for scenario analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioConfig {
    /// Map of scenario name to definition
    #[serde(default)]
    pub scenarios: BTreeMap<String, ScenarioDefinition>,
}

impl ScenarioConfig {
    /// Create a new empty configuration
    #[must_use]
    pub const fn new() -> Self {
        Self {
            scenarios: BTreeMap::new(),
        }
    }

    /// Add a scenario to the configuration
    pub fn add_scenario(&mut self, name: &str, scenario: ScenarioDefinition) -> &mut Self {
        self.scenarios.insert(name.to_string(), scenario);
        self
    }

    /// Validate the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if no scenarios are defined, probabilities do not sum
    /// to 1.0, or any individual probability is out of range.
    pub fn validate(&self) -> Result<(), String> {
        const TOLERANCE: f64 = 0.001;

        if self.scenarios.is_empty() {
            return Err("No scenarios defined".to_string());
        }

        // Validate probabilities sum to 1.0 (within tolerance)
        let total_prob: f64 = self.scenarios.values().map(|s| s.probability).sum();
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
    #[must_use]
    pub fn has_scenario(&self, name: &str) -> bool {
        self.scenarios.contains_key(name)
    }

    /// Get a scenario by name
    #[must_use]
    pub fn get_scenario(&self, name: &str) -> Option<&ScenarioDefinition> {
        self.scenarios.get(name)
    }
}

/// Builder pattern for `ScenarioDefinition`
impl ScenarioDefinition {
    /// Create a new scenario with given probability
    #[must_use]
    pub const fn new(probability: f64) -> Self {
        Self {
            probability,
            description: String::new(),
            scalars: BTreeMap::new(),
        }
    }

    /// Set the description
    #[must_use]
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Add a scalar override with a fixed value
    #[must_use]
    pub fn with_scalar(mut self, name: &str, value: f64) -> Self {
        self.scalars
            .insert(name.to_string(), ScalarOverride::Value(value));
        self
    }

    /// Add a scalar override with a formula
    #[must_use]
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

#[cfg(test)]
mod tests {

    mod scenario_forms {
        //! A scenario with `scalars` and no `probability` (Session Prompt part 1,
        //! item 3). `forge compare` weighs nothing and needs none; `forge scenarios`
        //! weighs by it and says which scenario lacks it. The help of `compare` shows
        //! both forms (item 4), and the output in it is what Forge prints.
        //!
        //! Expected: 1200 customers * 49 = 58,800; * (1 + 0.12 - 0.01) = 65,268;
        //! * (1 + 0.02 - 0.05) = 57,036.

        use crate::cli::{compare_core, scenarios_core};
        use std::path::Path;

        const BASE: &str = r#"_forge_version: "5.1.0"
assumptions:
  growth_rate: 0.05
  churn_rate: 0.02
  customers: 1200
  price: 49
summary:
  revenue: {formula: "=assumptions.customers * assumptions.price * (1 + assumptions.growth_rate - assumptions.churn_rate)"}
"#;

        const FULL_NO_PROBABILITY: &str = r"scenarios:
  optimistic:
    description: Faster growth, less churn
    scalars:
      assumptions.growth_rate: 0.12
      assumptions.churn_rate: 0.01
  pessimistic:
    description: Slow growth, more churn
    scalars:
      assumptions.growth_rate: 0.02
      assumptions.churn_rate: 0.05
";

        const FLAT: &str = r"scenarios:
  optimistic:
    assumptions.growth_rate: 0.12
    assumptions.churn_rate: 0.01
  pessimistic:
    assumptions.growth_rate: 0.02
    assumptions.churn_rate: 0.05
";

        fn model(dir: &Path, scenarios: &str) -> std::path::PathBuf {
            let path = dir.join("m.yaml");
            std::fs::write(&path, format!("{BASE}{scenarios}")).unwrap();
            path
        }

        fn both() -> Vec<String> {
            vec!["optimistic".to_string(), "pessimistic".to_string()]
        }

        #[test]
        fn compare_takes_the_full_form_without_a_probability() {
            let dir = tempfile::tempdir().unwrap();
            let file = model(dir.path(), FULL_NO_PROBABILITY);
            let result = compare_core(&file, &both()).unwrap();
            let revenue = &result.values["summary.revenue"];
            assert!((revenue["optimistic"].unwrap() - 65_268.0).abs() < 1e-6);
            assert!((revenue["pessimistic"].unwrap() - 57_036.0).abs() < 1e-6);
        }

        #[test]
        fn compare_takes_the_flat_form_as_before() {
            let dir = tempfile::tempdir().unwrap();
            let file = model(dir.path(), FLAT);
            let result = compare_core(&file, &both()).unwrap();
            assert!(
                (result.values["summary.revenue"]["optimistic"].unwrap() - 65_268.0).abs() < 1e-6
            );
        }

        #[test]
        fn scenarios_names_the_scenario_without_a_probability() {
            let dir = tempfile::tempdir().unwrap();
            let file = model(dir.path(), FULL_NO_PROBABILITY);
            let error = scenarios_core(&file, None).unwrap_err().to_string();
            for expected in [
                "scenario 'optimistic' has no probability",
                "sum to 1",
                "forge compare",
            ] {
                assert!(error.contains(expected), "{error}");
            }
        }

        #[test]
        fn a_misspelt_probability_is_still_caught() {
            let dir = tempfile::tempdir().unwrap();
            let typo =
                FULL_NO_PROBABILITY.replace("description:", "probabilty: 0.5\n    description:");
            let file = model(dir.path(), &typo);
            let error = compare_core(&file, &both()).unwrap_err().to_string();
            assert!(error.contains("Did you mean 'probability'"), "{error}");
        }

        #[test]
        fn the_help_example_is_what_forge_prints() {
            // The scalars of the model in `forge compare --help`, with its output
            let dir = tempfile::tempdir().unwrap();
            let file = model(dir.path(), FULL_NO_PROBABILITY);
            let result = compare_core(&file, &both()).unwrap();
            assert_eq!(
                result.variables,
                [
                    "assumptions.growth_rate",
                    "assumptions.churn_rate",
                    "assumptions.customers",
                    "assumptions.price",
                    "summary.revenue"
                ]
            );
        }
    }
}
