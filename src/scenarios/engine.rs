//! Scenario Analysis Engine
//!
//! Executes scenarios and calculates expected values.

use super::config::{ScalarOverride, ScenarioConfig, ScenarioDefinition};
use crate::core::ArrayCalculator;
use crate::types::{ParsedModel, Variable};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result for a single scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    /// Scenario name
    pub name: String,
    /// Probability assigned to this scenario
    pub probability: f64,
    /// Description
    pub description: String,
    /// Calculated scalar values
    pub scalars: HashMap<String, f64>,
    /// Key output metrics (NPV, IRR, etc.)
    pub outputs: HashMap<String, f64>,
}

/// Results from running all scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResults {
    /// Individual scenario results
    pub scenarios: Vec<ScenarioResult>,
    /// Expected values across all scenarios (probability-weighted)
    pub expected_values: HashMap<String, f64>,
    /// Probability of positive outcome for key metrics
    pub probability_positive: HashMap<String, f64>,
    /// Min/max ranges across scenarios
    pub ranges: HashMap<String, (f64, f64)>,
}

impl ScenarioResults {
    /// Export results to YAML format
    pub fn to_yaml(&self) -> String {
        serde_yaml_ng::to_string(self).unwrap_or_else(|_| "# Error serializing results".to_string())
    }

    /// Export results to JSON format
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Scenario Analysis Engine
pub struct ScenarioEngine {
    config: ScenarioConfig,
    base_model: ParsedModel,
    output_variables: Vec<String>,
}

impl ScenarioEngine {
    /// Create a new scenario engine
    pub fn new(config: ScenarioConfig, base_model: ParsedModel) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            base_model,
            output_variables: Vec::new(),
        })
    }

    /// Set which variables to track as outputs
    pub fn with_outputs(mut self, outputs: Vec<String>) -> Self {
        self.output_variables = outputs;
        self
    }

    /// Add an output variable to track
    pub fn add_output(&mut self, name: &str) {
        self.output_variables.push(name.to_string());
    }

    /// Run all scenarios and calculate results
    pub fn run(&self) -> Result<ScenarioResults, String> {
        let mut scenario_results = Vec::new();

        // Run each scenario
        for (name, scenario_def) in &self.config.scenarios {
            let result = self.run_scenario(name, scenario_def)?;
            scenario_results.push(result);
        }

        // Calculate expected values (probability-weighted means)
        let expected_values = self.calculate_expected_values(&scenario_results);

        // Calculate probability of positive outcomes
        let probability_positive = self.calculate_probability_positive(&scenario_results);

        // Calculate ranges
        let ranges = self.calculate_ranges(&scenario_results);

        Ok(ScenarioResults {
            scenarios: scenario_results,
            expected_values,
            probability_positive,
            ranges,
        })
    }

    /// Run a single scenario
    pub fn run_scenario(
        &self,
        name: &str,
        scenario: &ScenarioDefinition,
    ) -> Result<ScenarioResult, String> {
        // Clone the base model
        let mut model = self.base_model.clone();

        // Apply scenario overrides
        for (var_name, override_val) in &scenario.scalars {
            match override_val {
                ScalarOverride::Value(v) => {
                    if let Some(scalar) = model.scalars.get_mut(var_name) {
                        scalar.value = Some(*v);
                        scalar.formula = None;
                    } else {
                        model.scalars.insert(
                            var_name.clone(),
                            Variable::new(var_name.clone(), Some(*v), None),
                        );
                    }
                },
                ScalarOverride::Formula { formula } => {
                    if let Some(scalar) = model.scalars.get_mut(var_name) {
                        scalar.formula = Some(formula.clone());
                        scalar.value = None;
                    } else {
                        model.scalars.insert(
                            var_name.clone(),
                            Variable::new(var_name.clone(), None, Some(formula.clone())),
                        );
                    }
                },
            }
        }

        // Calculate the model
        let calculator = ArrayCalculator::new(model);
        let calculated = calculator.calculate_all().map_err(|e| e.to_string())?;

        // Extract scalar results
        let mut scalars = HashMap::new();
        for (var_name, var) in &calculated.scalars {
            if let Some(value) = var.value {
                scalars.insert(var_name.clone(), value);
            }
        }

        // Extract output values
        let mut outputs = HashMap::new();
        for output_name in &self.output_variables {
            if let Some(value) = scalars.get(output_name) {
                outputs.insert(output_name.clone(), *value);
            }
        }

        Ok(ScenarioResult {
            name: name.to_string(),
            probability: scenario.probability,
            description: scenario.description.clone(),
            scalars,
            outputs,
        })
    }

    /// Calculate probability-weighted expected values
    fn calculate_expected_values(&self, results: &[ScenarioResult]) -> HashMap<String, f64> {
        let mut expected = HashMap::new();

        // Get all variable names from first scenario
        if let Some(first) = results.first() {
            for var_name in first.scalars.keys() {
                let weighted_sum: f64 = results
                    .iter()
                    .filter_map(|r| r.scalars.get(var_name).map(|v| v * r.probability))
                    .sum();
                expected.insert(var_name.clone(), weighted_sum);
            }
        }

        expected
    }

    /// Calculate probability of positive outcomes for output variables
    fn calculate_probability_positive(&self, results: &[ScenarioResult]) -> HashMap<String, f64> {
        let mut prob_positive = HashMap::new();

        for output_name in &self.output_variables {
            let positive_prob: f64 = results
                .iter()
                .filter_map(|r| {
                    r.outputs
                        .get(output_name)
                        .map(|v| if *v > 0.0 { r.probability } else { 0.0 })
                })
                .sum();
            prob_positive.insert(output_name.clone(), positive_prob);
        }

        prob_positive
    }

    /// Calculate min/max ranges for output variables
    fn calculate_ranges(&self, results: &[ScenarioResult]) -> HashMap<String, (f64, f64)> {
        let mut ranges = HashMap::new();

        for output_name in &self.output_variables {
            let values: Vec<f64> = results
                .iter()
                .filter_map(|r| r.outputs.get(output_name).copied())
                .collect();

            if !values.is_empty() {
                let min = values.iter().copied().fold(f64::INFINITY, f64::min);
                let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                ranges.insert(output_name.clone(), (min, max));
            }
        }

        ranges
    }

    /// Get the scenario configuration
    pub fn config(&self) -> &ScenarioConfig {
        &self.config
    }
}

#[cfg(test)]
mod engine_tests {
    use super::*;

    fn create_test_model() -> ParsedModel {
        let mut model = ParsedModel::new();
        model.scalars.insert(
            "base_revenue".to_string(),
            Variable::new("base_revenue".to_string(), Some(1_000_000.0), None),
        );
        model.scalars.insert(
            "revenue_growth".to_string(),
            Variable::new("revenue_growth".to_string(), Some(0.05), None),
        );
        model.scalars.insert(
            "projected_revenue".to_string(),
            Variable::new(
                "projected_revenue".to_string(),
                None,
                Some("=base_revenue * (1 + revenue_growth)".to_string()),
            ),
        );
        model
    }

    fn create_test_config() -> ScenarioConfig {
        let mut config = ScenarioConfig::new();
        config.add_scenario(
            "base_case",
            ScenarioDefinition::new(0.50)
                .with_description("Base case")
                .with_scalar("revenue_growth", 0.05),
        );
        config.add_scenario(
            "bull_case",
            ScenarioDefinition::new(0.30)
                .with_description("Bull case")
                .with_scalar("revenue_growth", 0.15),
        );
        config.add_scenario(
            "bear_case",
            ScenarioDefinition::new(0.20)
                .with_description("Bear case")
                .with_scalar("revenue_growth", -0.10),
        );
        config
    }

    #[test]
    fn test_scenario_engine_creation() {
        let config = create_test_config();
        let model = create_test_model();
        let engine = ScenarioEngine::new(config, model);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_scenario_execution() {
        let config = create_test_config();
        let model = create_test_model();
        let engine = ScenarioEngine::new(config, model)
            .unwrap()
            .with_outputs(vec!["projected_revenue".to_string()]);

        let results = engine.run().unwrap();

        // Should have 3 scenarios
        assert_eq!(results.scenarios.len(), 3);

        // Validate individual scenario results
        for scenario in &results.scenarios {
            let growth = scenario.scalars.get("revenue_growth").unwrap();
            let revenue = scenario.scalars.get("projected_revenue").unwrap();
            let expected = 1_000_000.0 * (1.0 + growth);
            assert!(
                (revenue - expected).abs() < 0.01,
                "Revenue mismatch for {}: got {}, expected {}",
                scenario.name,
                revenue,
                expected
            );
        }
    }

    #[test]
    fn test_expected_value_calculation() {
        let config = create_test_config();
        let model = create_test_model();
        let engine = ScenarioEngine::new(config, model)
            .unwrap()
            .with_outputs(vec!["projected_revenue".to_string()]);

        let results = engine.run().unwrap();

        // Expected revenue = 0.5*1.05M + 0.3*1.15M + 0.2*0.9M
        //                  = 525000 + 345000 + 180000 = 1,050,000
        let expected_revenue = results.expected_values.get("projected_revenue").unwrap();
        let calculated = 0.5 * 1_050_000.0 + 0.3 * 1_150_000.0 + 0.2 * 900_000.0;
        assert!(
            (expected_revenue - calculated).abs() < 0.01,
            "Expected value mismatch: got {expected_revenue}, expected {calculated}"
        );
    }

    #[test]
    fn test_probability_positive() {
        let config = create_test_config();
        let model = create_test_model();
        let engine = ScenarioEngine::new(config, model)
            .unwrap()
            .with_outputs(vec!["projected_revenue".to_string()]);

        let results = engine.run().unwrap();

        // All scenarios have positive revenue
        let prob_positive = results
            .probability_positive
            .get("projected_revenue")
            .unwrap();
        assert!(
            (*prob_positive - 1.0).abs() < 0.001,
            "Probability positive should be 1.0, got {prob_positive}"
        );
    }

    #[test]
    fn test_yaml_export() {
        let config = create_test_config();
        let model = create_test_model();
        let engine = ScenarioEngine::new(config, model)
            .unwrap()
            .with_outputs(vec!["projected_revenue".to_string()]);

        let results = engine.run().unwrap();
        let yaml = results.to_yaml();

        assert!(yaml.contains("scenarios:"));
        assert!(yaml.contains("expected_values:"));
    }

    #[test]
    fn test_json_export() {
        let config = create_test_config();
        let model = create_test_model();
        let engine = ScenarioEngine::new(config, model)
            .unwrap()
            .with_outputs(vec!["projected_revenue".to_string()]);

        let results = engine.run().unwrap();
        let json = results.to_json().unwrap();

        assert!(json.contains("\"scenarios\""));
        assert!(json.contains("\"expected_values\""));
    }
}
