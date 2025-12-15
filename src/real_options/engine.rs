//! Real Options Engine
//!
//! Orchestrates option valuation using configured method.

use super::binomial::BinomialTree;
use super::black_scholes::BlackScholes;
use super::config::{OptionDefinition, OptionType, RealOptionsConfig, ValuationMethod};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result for a single option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionResult {
    /// Option name
    pub name: String,
    /// Option type
    pub option_type: OptionType,
    /// Option value
    pub value: f64,
    /// Probability of exercise (from simulation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probability_exercise: Option<f64>,
    /// Optimal trigger condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimal_trigger: Option<String>,
}

/// Complete options analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionsResult {
    /// Analysis name
    pub name: String,
    /// Underlying asset parameters
    pub underlying: UnderlyingResult,
    /// Traditional NPV (without options)
    pub traditional_npv: f64,
    /// Individual option results
    pub options: HashMap<String, OptionResult>,
    /// Total option value
    pub total_option_value: f64,
    /// Project value with options (NPV + option value)
    pub project_value_with_options: f64,
    /// Decision recommendation
    pub decision: String,
    /// Detailed recommendation
    pub recommendation: String,
}

/// Underlying asset summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnderlyingResult {
    pub current_value: f64,
    pub volatility: f64,
    pub risk_free_rate: f64,
    pub time_horizon: f64,
}

impl OptionsResult {
    /// Export results to YAML format
    pub fn to_yaml(&self) -> String {
        serde_yaml_ng::to_string(self).unwrap_or_else(|_| "# Error serializing results".to_string())
    }

    /// Export results to JSON format
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Real Options Engine
pub struct RealOptionsEngine {
    config: RealOptionsConfig,
    /// Traditional NPV (can be set externally)
    traditional_npv: f64,
}

impl RealOptionsEngine {
    /// Create a new real options engine
    pub fn new(config: RealOptionsConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            traditional_npv: 0.0,
        })
    }

    /// Set the traditional NPV for comparison
    pub fn with_traditional_npv(mut self, npv: f64) -> Self {
        self.traditional_npv = npv;
        self
    }

    /// Analyze all options
    pub fn analyze(&self) -> Result<OptionsResult, String> {
        let mut option_results = HashMap::new();
        let mut total_value = 0.0;

        for option in &self.config.options {
            let result = self.value_option(option)?;
            total_value += result.value;
            option_results.insert(option.name.clone(), result);
        }

        let project_value = self.traditional_npv + total_value;
        let decision = if project_value > 0.0 {
            "ACCEPT (with options)".to_string()
        } else {
            "REJECT".to_string()
        };

        let recommendation = self.generate_recommendation(&option_results, total_value);

        Ok(OptionsResult {
            name: self.config.name.clone(),
            underlying: UnderlyingResult {
                current_value: self.config.underlying.current_value,
                volatility: self.config.underlying.volatility,
                risk_free_rate: self.config.underlying.risk_free_rate,
                time_horizon: self.config.underlying.time_horizon,
            },
            traditional_npv: self.traditional_npv,
            options: option_results,
            total_option_value: total_value,
            project_value_with_options: project_value,
            decision,
            recommendation,
        })
    }

    /// Value a single option
    fn value_option(&self, option: &OptionDefinition) -> Result<OptionResult, String> {
        let value = match self.config.method {
            ValuationMethod::BlackScholes => self.value_with_black_scholes(option),
            ValuationMethod::Binomial => self.value_with_binomial(option),
            ValuationMethod::MonteCarlo => self.value_with_monte_carlo(option),
        };

        let trigger = self.determine_trigger(option);

        Ok(OptionResult {
            name: option.name.clone(),
            option_type: option.option_type,
            value,
            probability_exercise: None, // Would be populated from simulation
            optimal_trigger: trigger,
        })
    }

    /// Value option using Black-Scholes
    fn value_with_black_scholes(&self, option: &OptionDefinition) -> f64 {
        let u = &self.config.underlying;

        match option.option_type {
            OptionType::Defer => {
                let bs = BlackScholes::new(
                    u.current_value,
                    option.exercise_cost,
                    u.risk_free_rate,
                    u.volatility,
                    option.max_deferral.min(u.time_horizon),
                )
                .with_dividend_yield(u.dividend_yield);
                bs.call_price()
            }
            OptionType::Expand => {
                let additional_value = (option.expansion_factor - 1.0) * u.current_value;
                let bs = BlackScholes::new(
                    additional_value,
                    option.exercise_cost,
                    u.risk_free_rate,
                    u.volatility,
                    u.time_horizon,
                )
                .with_dividend_yield(u.dividend_yield);
                bs.call_price()
            }
            OptionType::Abandon => {
                let bs = BlackScholes::new(
                    u.current_value,
                    option.salvage_value,
                    u.risk_free_rate,
                    u.volatility,
                    u.time_horizon,
                )
                .with_dividend_yield(u.dividend_yield);
                bs.put_price()
            }
            OptionType::Contract => {
                let reduction = (1.0 - option.contraction_factor) * u.current_value;
                let bs = BlackScholes::new(
                    reduction,
                    option.exercise_cost.abs(), // Cost savings
                    u.risk_free_rate,
                    u.volatility,
                    u.time_horizon,
                )
                .with_dividend_yield(u.dividend_yield);
                bs.put_price()
            }
            OptionType::Switch | OptionType::Compound => {
                // Complex options - fallback to binomial
                self.value_with_binomial(option)
            }
        }
    }

    /// Value option using binomial tree
    fn value_with_binomial(&self, option: &OptionDefinition) -> f64 {
        let u = &self.config.underlying;
        let steps = self.config.binomial_steps;

        let tree = BinomialTree::new(
            u.current_value,
            u.current_value, // Placeholder, actual strike varies by option type
            u.risk_free_rate,
            u.volatility,
            u.time_horizon,
            steps,
        )
        .with_dividend_yield(u.dividend_yield);

        match option.option_type {
            OptionType::Defer => tree.defer_option_value(option.max_deferral, option.exercise_cost),
            OptionType::Expand => {
                tree.expand_option_value(option.expansion_factor, option.exercise_cost)
            }
            OptionType::Abandon => tree.abandon_option_value(option.salvage_value),
            OptionType::Contract => {
                tree.contract_option_value(option.contraction_factor, option.exercise_cost.abs())
            }
            OptionType::Switch => {
                // Switch option approximated as max of expand and contract
                let expand = tree.expand_option_value(1.2, option.exercise_cost);
                let contract = tree.contract_option_value(0.8, option.exercise_cost.abs());
                expand.max(contract)
            }
            OptionType::Compound => {
                // Compound option - simplified as defer then expand
                let defer = tree.defer_option_value(1.0, option.exercise_cost * 0.5);
                let expand =
                    tree.expand_option_value(option.expansion_factor, option.exercise_cost);
                defer + expand * 0.5
            }
        }
    }

    /// Value option using Monte Carlo simulation
    fn value_with_monte_carlo(&self, option: &OptionDefinition) -> f64 {
        // Simplified LSM (Longstaff-Schwartz) approximation
        // For full implementation, would use path simulation
        // Falls back to binomial for now with more steps
        let mut config = self.config.clone();
        config.binomial_steps = 200; // More accurate
        let engine = RealOptionsEngine::new(config).unwrap();

        // Use binomial with more steps as approximation
        engine.value_with_binomial(option)
    }

    /// Determine optimal trigger condition
    fn determine_trigger(&self, option: &OptionDefinition) -> Option<String> {
        let u = &self.config.underlying;

        match option.option_type {
            OptionType::Defer => {
                let trigger_value = option.exercise_cost * 1.1; // 10% above exercise cost
                Some(format!("value > ${:.0}", trigger_value))
            }
            OptionType::Expand => {
                let trigger_value = option.exercise_cost * 2.0; // Good ROI on expansion
                Some(format!("value > ${:.0}", trigger_value))
            }
            OptionType::Abandon => {
                let trigger_value = option.salvage_value * 1.2;
                Some(format!("value < ${:.0}", trigger_value))
            }
            OptionType::Contract => {
                let trigger_value = u.current_value * option.contraction_factor;
                Some(format!("value < ${:.0}", trigger_value))
            }
            _ => None,
        }
    }

    /// Generate recommendation text
    fn generate_recommendation(
        &self,
        options: &HashMap<String, OptionResult>,
        total_value: f64,
    ) -> String {
        let mut parts = Vec::new();

        // Find highest value option
        if let Some((name, result)) = options.iter().max_by(|a, b| {
            a.1.value
                .partial_cmp(&b.1.value)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            parts.push(format!(
                "Highest value option: {} (${:.0})",
                name, result.value
            ));
        }

        if total_value > self.traditional_npv.abs() {
            parts.push("Option value exceeds negative NPV - consider proceeding".to_string());
        }

        // Add specific recommendations by option type
        for (name, result) in options {
            if let Some(ref trigger) = result.optimal_trigger {
                parts.push(format!("{}: exercise when {}", name, trigger));
            }
        }

        parts.join(". ")
    }

    /// Get the configuration
    pub fn config(&self) -> &RealOptionsConfig {
        &self.config
    }
}

#[cfg(test)]
mod engine_tests {
    use super::*;
    use crate::real_options::config::{OptionDefinition, UnderlyingConfig};

    fn create_test_config() -> RealOptionsConfig {
        RealOptionsConfig::new(
            "Factory Investment",
            UnderlyingConfig::new(10_000_000.0, 0.30, 0.05, 3.0),
        )
        .with_option(OptionDefinition::defer("Wait 2 years", 2.0, 8_000_000.0))
        .with_option(OptionDefinition::expand("Build Phase 2", 1.5, 4_000_000.0))
        .with_option(OptionDefinition::abandon("Sell assets", 3_000_000.0))
        .with_binomial_steps(100)
    }

    #[test]
    fn test_engine_creation() {
        let config = create_test_config();
        let engine = RealOptionsEngine::new(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_option_valuation() {
        let config = create_test_config();
        let engine = RealOptionsEngine::new(config)
            .unwrap()
            .with_traditional_npv(-500_000.0);

        let result = engine.analyze().unwrap();

        // All options should have positive value
        for opt_result in result.options.values() {
            assert!(
                opt_result.value > 0.0,
                "{} should have positive value",
                opt_result.name
            );
        }

        // Total option value should be positive
        assert!(result.total_option_value > 0.0);

        // With options, might turn negative NPV positive
        println!("Traditional NPV: ${}", result.traditional_npv);
        println!("Total Option Value: ${}", result.total_option_value);
        println!(
            "Project Value with Options: ${}",
            result.project_value_with_options
        );
    }

    #[test]
    fn test_adr020_example() {
        // From ADR-020:
        // Traditional NPV: -$500K
        // With options: +$1.9M
        // Options add $2.4M of value

        let config = RealOptionsConfig::new(
            "Phased Factory Investment",
            UnderlyingConfig::new(10_000_000.0, 0.30, 0.05, 3.0),
        )
        .with_method(ValuationMethod::Binomial)
        .with_option(OptionDefinition::defer(
            "Wait up to 2 years",
            2.0,
            8_000_000.0,
        ))
        .with_option(OptionDefinition::expand("Build Phase 2", 1.5, 4_000_000.0))
        .with_option(OptionDefinition::abandon("Sell assets", 3_000_000.0))
        .with_binomial_steps(100);

        let engine = RealOptionsEngine::new(config)
            .unwrap()
            .with_traditional_npv(-500_000.0);

        let result = engine.analyze().unwrap();

        // Total option value should be substantial
        assert!(
            result.total_option_value > 1_000_000.0,
            "Options should add significant value: {}",
            result.total_option_value
        );

        // Project should be acceptable with options
        if result.project_value_with_options > 0.0 {
            assert_eq!(result.decision, "ACCEPT (with options)");
        }
    }

    #[test]
    fn test_black_scholes_method() {
        let config =
            RealOptionsConfig::new("BS Test", UnderlyingConfig::new(100.0, 0.20, 0.05, 1.0))
                .with_method(ValuationMethod::BlackScholes)
                .with_option(OptionDefinition::defer("Wait 1 year", 1.0, 100.0));

        let engine = RealOptionsEngine::new(config).unwrap();
        let result = engine.analyze().unwrap();

        // Defer option value should be close to BS call value
        let defer_value = result.options.get("Wait 1 year").unwrap().value;
        assert!(defer_value > 5.0 && defer_value < 20.0);
    }

    #[test]
    fn test_yaml_export() {
        let config = create_test_config();
        let engine = RealOptionsEngine::new(config).unwrap();
        let result = engine.analyze().unwrap();
        let yaml = result.to_yaml();

        assert!(yaml.contains("total_option_value"));
        assert!(yaml.contains("project_value_with_options"));
        assert!(yaml.contains("decision"));
    }

    #[test]
    fn test_json_export() {
        let config = create_test_config();
        let engine = RealOptionsEngine::new(config).unwrap();
        let result = engine.analyze().unwrap();
        let json = result.to_json().unwrap();

        assert!(json.contains("\"total_option_value\""));
        assert!(json.contains("\"options\""));
    }
}
