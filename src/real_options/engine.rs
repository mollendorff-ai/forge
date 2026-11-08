//! Real Options Engine
//!
//! Orchestrates option valuation using configured method.

use super::binomial::BinomialTree;
use super::black_scholes::BlackScholes;
use super::config::{OptionDefinition, OptionType, RealOptionsConfig, ValuationMethod};
use serde::{Deserialize, Serialize};

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
    /// Rule of thumb for when to exercise (not the optimal exercise boundary)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimal_trigger: Option<String>,
    /// Value of the project when it has this option. A defer option already
    /// includes investing today, so this is the option value itself; any other
    /// option is held on top of the project, so this is traditional NPV + value.
    /// `None` when that needs a traditional NPV and none was given.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_value: Option<f64>,
}

/// Complete options analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionsResult {
    /// Analysis name
    pub name: String,
    /// Underlying asset parameters
    pub underlying: UnderlyingResult,
    /// Traditional NPV (without options), if given
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traditional_npv: Option<f64>,
    /// Individual option results
    pub options: std::collections::BTreeMap<String, OptionResult>,
    /// Option whose project value is highest. Option values overlap (waiting and
    /// expanding both depend on the same project), so they are never added up.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_option: Option<String>,
    /// Project value with the best option
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_value_with_options: Option<f64>,
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
    #[must_use]
    pub fn to_yaml(&self) -> String {
        yaml_serde::to_string(self).unwrap_or_else(|_| "# Error serializing results".to_string())
    }

    /// Export results to JSON format
    ///
    /// # Errors
    ///
    /// Returns an error if JSON serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Real Options Engine
pub struct RealOptionsEngine {
    config: RealOptionsConfig,
    /// Traditional NPV (from the config, or set externally)
    traditional_npv: Option<f64>,
}

impl RealOptionsEngine {
    /// Create a new real options engine
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn new(config: RealOptionsConfig) -> Result<Self, String> {
        config.validate()?;
        let traditional_npv = config.traditional_npv;
        Ok(Self {
            config,
            traditional_npv,
        })
    }

    /// Set the traditional NPV for comparison
    #[must_use]
    pub const fn with_traditional_npv(mut self, npv: f64) -> Self {
        self.traditional_npv = Some(npv);
        self
    }

    /// Analyze all options
    ///
    /// # Errors
    ///
    /// Returns an error if any option valuation fails.
    pub fn analyze(&self) -> Result<OptionsResult, String> {
        let mut option_results = std::collections::BTreeMap::new();
        let mut best: Option<(String, f64)> = None;

        for option in &self.config.options {
            let result = self.value_option(option);
            if let Some(value) = result.project_value {
                let better = best.as_ref().is_none_or(|(name, v)| {
                    value
                        .total_cmp(v)
                        .then_with(|| name.cmp(&option.name))
                        .is_gt()
                });
                if better {
                    best = Some((option.name.clone(), value));
                }
            }
            option_results.insert(option.name.clone(), result);
        }

        // A best option can only be named when every option has a project value.
        let needs_npv = option_results.values().any(|r| r.project_value.is_none());
        if needs_npv {
            best = None;
        }
        let decision = match (&best, needs_npv) {
            (_, true) => "UNDECIDED: set traditional_npv to value expand, abandon, contract, switch or compound options".to_string(),
            (Some((name, value)), false) if *value > 0.0 => format!("ACCEPT (with {name})"),
            (Some(_), false) => "REJECT".to_string(),
            (None, false) => match self.traditional_npv {
                Some(npv) if npv > 0.0 => "ACCEPT".to_string(),
                Some(_) => "REJECT".to_string(),
                None => "UNDECIDED: no options and no traditional_npv".to_string(),
            },
        };

        let recommendation = self.generate_recommendation(&option_results, best.as_ref());

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
            best_option: best.as_ref().map(|(name, _)| name.clone()),
            project_value_with_options: best.map(|(_, value)| value),
            decision,
            recommendation,
        })
    }

    /// Value a single option
    fn value_option(&self, option: &OptionDefinition) -> OptionResult {
        let value = match self.config.method {
            ValuationMethod::BlackScholes => self.value_with_black_scholes(option),
            ValuationMethod::Binomial => self.value_with_binomial(option),
            ValuationMethod::MonteCarlo => self.value_with_monte_carlo(option),
        };

        let trigger = self.determine_trigger(option);
        let project_value = match option.option_type {
            OptionType::Defer => Some(value),
            _ => self.traditional_npv.map(|npv| npv + value),
        };

        OptionResult {
            name: option.name.clone(),
            option_type: option.option_type,
            value,
            probability_exercise: None, // Would be populated from simulation
            optimal_trigger: trigger,
            project_value,
        }
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
            },
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
            },
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
            },
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
            },
            OptionType::Switch | OptionType::Compound => {
                // Complex options - fallback to binomial
                self.value_with_binomial(option)
            },
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
            },
            OptionType::Abandon => tree.abandon_option_value(option.salvage_value),
            OptionType::Contract => {
                tree.contract_option_value(option.contraction_factor, option.exercise_cost.abs())
            },
            OptionType::Switch => {
                // Switch option approximated as max of expand and contract
                let expand = tree.expand_option_value(1.2, option.exercise_cost);
                let contract = tree.contract_option_value(0.8, option.exercise_cost.abs());
                expand.max(contract)
            },
            OptionType::Compound => {
                // Compound option - simplified as defer then expand
                let defer = tree.defer_option_value(1.0, option.exercise_cost * 0.5);
                let expand =
                    tree.expand_option_value(option.expansion_factor, option.exercise_cost);
                expand.mul_add(0.5, defer)
            },
        }
    }

    /// Value option using Monte Carlo simulation
    fn value_with_monte_carlo(&self, option: &OptionDefinition) -> f64 {
        // Simplified LSM (Longstaff-Schwartz) approximation
        // For full implementation, would use path simulation
        // Falls back to binomial for now with more steps
        let mut config = self.config.clone();
        config.binomial_steps = 200; // More accurate
        let engine = Self::new(config).unwrap();

        // Use binomial with more steps as approximation
        engine.value_with_binomial(option)
    }

    /// Determine optimal trigger condition
    fn determine_trigger(&self, option: &OptionDefinition) -> Option<String> {
        let u = &self.config.underlying;

        match option.option_type {
            OptionType::Defer => {
                let trigger_value = option.exercise_cost * 1.1; // 10% above exercise cost
                Some(format!("value > ${trigger_value:.0}"))
            },
            OptionType::Expand => {
                let trigger_value = option.exercise_cost * 2.0; // Good ROI on expansion
                Some(format!("value > ${trigger_value:.0}"))
            },
            OptionType::Abandon => {
                let trigger_value = option.salvage_value * 1.2;
                Some(format!("value < ${trigger_value:.0}"))
            },
            OptionType::Contract => {
                let trigger_value = u.current_value * option.contraction_factor;
                Some(format!("value < ${trigger_value:.0}"))
            },
            _ => None,
        }
    }

    /// Generate recommendation text
    fn generate_recommendation(
        &self,
        options: &std::collections::BTreeMap<String, OptionResult>,
        best: Option<&(String, f64)>,
    ) -> String {
        let mut parts = Vec::new();

        if let Some((name, value)) = best {
            parts.push(format!("Best option: {name}, project value ${value:.0}"));
            if let Some(npv) = self.traditional_npv {
                if npv < 0.0 && *value > 0.0 {
                    parts.push(format!(
                        "Traditional NPV is negative (${npv:.0}), but with {name} the project is worth ${value:.0}"
                    ));
                }
                if options
                    .get(name)
                    .is_some_and(|o| o.option_type == OptionType::Defer)
                {
                    let premium = value - npv.max(0.0);
                    parts.push(format!(
                        "Keeping the right to wait is worth ${premium:.0} more than deciding now"
                    ));
                }
            }
        }

        if options.len() > 1 {
            parts.push(
                "Option values overlap, so they are compared, not added together".to_string(),
            );
        }

        // Rules of thumb by option type, in a fixed order
        let mut names: Vec<&String> = options.keys().collect();
        names.sort();
        for name in names {
            if let Some(ref trigger) = options[name].optimal_trigger {
                parts.push(format!("{name}: rule of thumb, exercise when {trigger}"));
            }
        }

        parts.join(". ")
    }

    /// Get the configuration
    #[must_use]
    pub const fn config(&self) -> &RealOptionsConfig {
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

        // Every option has a project value, because an NPV was given
        assert!(result.options.values().all(|o| o.project_value.is_some()));
        assert!(result.project_value_with_options.is_some());
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

        // The best option turns the -$500K project into a positive one
        let best = result.best_option.as_deref().unwrap();
        let value = result.project_value_with_options.unwrap();
        assert!(value > 1_000_000.0, "best project value: {value}");
        assert_eq!(result.decision, format!("ACCEPT (with {best})"));
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
        let engine = RealOptionsEngine::new(config)
            .unwrap()
            .with_traditional_npv(-500_000.0);
        let result = engine.analyze().unwrap();
        let yaml = result.to_yaml();

        assert!(yaml.contains("best_option"));
        assert!(yaml.contains("project_value_with_options"));
        assert!(!yaml.contains("total_option_value"));
        assert!(yaml.contains("decision"));
    }

    #[test]
    fn test_json_export() {
        let config = create_test_config();
        let engine = RealOptionsEngine::new(config)
            .unwrap()
            .with_traditional_npv(-500_000.0);
        let result = engine.analyze().unwrap();
        let json = result.to_json().unwrap();

        assert!(json.contains("\"project_value_with_options\""));
        assert!(json.contains("\"options\""));
    }
}
