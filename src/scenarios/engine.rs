//! Scenario Analysis Engine
//!
//! Executes scenarios and calculates expected values.

use super::config::{ScalarOverride, ScenarioConfig, ScenarioDefinition};
use crate::core::ArrayCalculator;
use crate::types::ParsedModel;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    pub scalars: BTreeMap<String, f64>,
    /// Key output metrics (NPV, IRR, etc.)
    pub outputs: BTreeMap<String, f64>,
}

/// Results from running all scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResults {
    /// Individual scenario results
    pub scenarios: Vec<ScenarioResult>,
    /// Expected values across all scenarios (probability-weighted)
    pub expected_values: BTreeMap<String, f64>,
    /// Probability of positive outcome for key metrics
    pub probability_positive: BTreeMap<String, f64>,
    /// Min/max ranges across scenarios
    pub ranges: BTreeMap<String, (f64, f64)>,
    /// The scenarios without a probability, left out of the weighting, in the
    /// file's order (ADR-067)
    #[serde(default)]
    pub unweighted: Vec<String>,
}

impl ScenarioResults {
    /// Export results to YAML format
    ///
    /// # Panics
    ///
    /// Never: results are names and numbers, which serialize as YAML.
    #[must_use]
    pub fn to_yaml(&self) -> String {
        yaml_serde::to_string(&crate::core::precision::Rounded(self))
            .expect("results are strings and numbers, which serialize as YAML")
    }

    /// Export results to JSON format
    ///
    /// # Errors
    ///
    /// Returns an error if JSON serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&crate::core::precision::Rounded(self))
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
    ///
    /// # Errors
    ///
    /// Returns an error if the scenario configuration is invalid.
    pub fn new(config: ScenarioConfig, base_model: ParsedModel) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            base_model,
            output_variables: Vec::new(),
        })
    }

    /// Set which variables to track as outputs
    #[must_use]
    pub fn with_outputs(mut self, outputs: Vec<String>) -> Self {
        self.output_variables = outputs;
        self
    }

    /// Add an output variable to track
    pub fn add_output(&mut self, name: &str) {
        self.output_variables.push(name.to_string());
    }

    /// Run all scenarios and calculate results
    ///
    /// # Errors
    ///
    /// Returns an error if any scenario calculation fails.
    pub fn run(&self) -> Result<ScenarioResults, String> {
        let mut scenario_results = Vec::new();

        // Run each scenario
        for (name, scenario_def) in &self.config.scenarios {
            let result = self.run_scenario(name, scenario_def)?;
            scenario_results.push(result);
        }

        // Calculate expected values (probability-weighted means)
        let expected_values = Self::calculate_expected_values(&scenario_results);

        // Calculate probability of positive outcomes
        let probability_positive = self.calculate_probability_positive(&scenario_results);

        // Calculate ranges
        let ranges = self.calculate_ranges(&scenario_results);

        Ok(ScenarioResults {
            scenarios: scenario_results,
            expected_values,
            probability_positive,
            ranges,
            unweighted: self.config.unweighted.clone(),
        })
    }

    /// Run a single scenario
    ///
    /// # Errors
    ///
    /// Returns an error if the model calculation fails for this scenario.
    pub fn run_scenario(
        &self,
        name: &str,
        scenario: &ScenarioDefinition,
    ) -> Result<ScenarioResult, String> {
        // Clone the base model
        let mut model = self.base_model.clone();

        // Apply scenario overrides (resolving grouped names like "rate" → "assumptions.rate",
        // and `@namespace.name` to a scalar of an included file)
        for (var_name, override_val) in &scenario.scalars {
            let applied = match override_val {
                ScalarOverride::Value(v) => model.override_scalar(var_name, Some(*v), None),
                ScalarOverride::Formula { formula } => {
                    model.override_scalar(var_name, None, Some(formula.clone()))
                },
                ScalarOverride::Date(text) => match crate::parser::scalar_date(text, var_name) {
                    Ok(Some(serial)) => model.override_date(var_name, serial),
                    Ok(None) => Err(format!(
                        "'{var_name}': \"{text}\" is not a number or a date"
                    )),
                    Err(e) => Err(e.to_string()),
                },
            };
            applied.map_err(|e| format!("Scenario '{name}': {e}"))?;
        }

        // Calculate the model
        let calculator = ArrayCalculator::new(model);
        let calculated = calculator.calculate_all().map_err(|e| e.to_string())?;

        // Extract scalar results
        let mut scalars = BTreeMap::new();
        for (var_name, var) in &calculated.scalars {
            if let Some(value) = var.value {
                scalars.insert(var_name.clone(), value);
            }
        }

        // Extract output values
        let mut outputs = BTreeMap::new();
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
    fn calculate_expected_values(results: &[ScenarioResult]) -> BTreeMap<String, f64> {
        let mut expected = BTreeMap::new();

        // Get all variable names from first scenario (a run has at least one)
        let names = results
            .first()
            .into_iter()
            .flat_map(|first| first.scalars.keys());
        for var_name in names {
            let weighted_sum: f64 = results
                .iter()
                .filter_map(|r| r.scalars.get(var_name).map(|v| v * r.probability))
                .sum();
            expected.insert(var_name.clone(), weighted_sum);
        }

        expected
    }

    /// Calculate probability of positive outcomes for output variables
    fn calculate_probability_positive(&self, results: &[ScenarioResult]) -> BTreeMap<String, f64> {
        let mut prob_positive = BTreeMap::new();

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
    fn calculate_ranges(&self, results: &[ScenarioResult]) -> BTreeMap<String, (f64, f64)> {
        let mut ranges = BTreeMap::new();

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
    #[must_use]
    pub const fn config(&self) -> &ScenarioConfig {
        &self.config
    }
}

#[cfg(test)]
mod engine_tests {
    use super::*;
    use crate::types::Variable;

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
        let calculated = 0.2f64.mul_add(900_000.0, 0.5f64.mul_add(1_050_000.0, 0.3 * 1_150_000.0));
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

    fn engine_of(scenario: &ScenarioDefinition, outputs: &[&str]) -> ScenarioEngine {
        let mut config = ScenarioConfig::new();
        config.add_scenario("only", scenario.clone());
        let engine = ScenarioEngine::new(config, create_test_model()).unwrap();
        outputs.iter().fold(engine, |mut engine, output| {
            engine.add_output(output);
            engine
        })
    }

    /// A formula is an override too, and a scenario's result reads it.
    #[test]
    fn a_formula_override_is_calculated_and_outputs_are_added_one_by_one() {
        let scenario = ScenarioDefinition::new(1.0).with_formula("revenue_growth", "=0.5 * 0.2");
        let engine = engine_of(&scenario, &["projected_revenue"]);
        assert_eq!(engine.config().scenario_names(), ["only"]);
        let result = engine.run_scenario("only", &scenario).unwrap();
        assert!((result.scalars["projected_revenue"] - 1_100_000.0).abs() < 1e-6);
        assert_eq!(
            result.outputs.keys().collect::<Vec<_>>(),
            ["projected_revenue"]
        );
    }

    /// An output no scalar has is not in a result, nor in the ranges.
    #[test]
    fn an_output_no_scenario_has_is_left_out_of_the_results() {
        let scenario = ScenarioDefinition::new(1.0);
        let engine = engine_of(&scenario, &["nothing_like_it"]);
        let results = engine.run().unwrap();
        assert!(results.scenarios[0].outputs.is_empty());
        assert!(results.ranges.is_empty());
        assert!((results.probability_positive["nothing_like_it"]).abs() < 1e-15);
    }

    /// A scalar with no value, never calculated and with no formula, has no
    /// result either.
    #[test]
    fn a_scalar_with_no_value_has_no_result() {
        let mut model = create_test_model();
        model.scalars.insert(
            "gap".to_string(),
            Variable::new("gap".to_string(), None, None),
        );
        let mut config = ScenarioConfig::new();
        config.add_scenario("only", ScenarioDefinition::new(1.0));
        let results = ScenarioEngine::new(config, model).unwrap().run().unwrap();
        assert!(!results.scenarios[0].scalars.contains_key("gap"));
        assert!(results.scenarios[0].scalars.contains_key("base_revenue"));
    }

    /// What an override cannot do, and a calculation that fails, name the
    /// scenario or the formula.
    #[test]
    fn a_scenario_that_cannot_be_applied_or_calculated_is_an_error() {
        let date = |text: &str| {
            let mut scenario = ScenarioDefinition::new(1.0);
            scenario.scalars.insert(
                "revenue_growth".to_string(),
                ScalarOverride::Date(text.to_string()),
            );
            engine_of(&scenario, &[]).run().unwrap_err()
        };
        assert_eq!(
            date("hello"),
            "Scenario 'only': 'revenue_growth': \"hello\" is not a number or a date"
        );
        assert_eq!(
            date("2026-02-30"),
            "Scenario 'only': Parse error: 'revenue_growth' is not a date: 2026-02-30 is no day of the calendar (a date is written 2026-10-31)"
        );
        let zero = ScenarioDefinition::new(1.0)
            .with_scalar("base_revenue", 0.0)
            .with_formula("revenue_growth", "=1 / base_revenue");
        assert_eq!(
            engine_of(&zero, &[]).run().unwrap_err(),
            "Formula evaluation error: 1 formula failed:\n  - revenue_growth: Division by zero (formula: =1 / base_revenue)\n  Not calculated, because they use a failed formula:\n  - projected_revenue (uses revenue_growth)"
        );
    }
}
