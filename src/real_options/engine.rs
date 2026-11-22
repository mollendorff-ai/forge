//! Real Options Engine
//!
//! Orchestrates option valuation using configured method.

use super::binomial::BinomialTree;
use super::black_scholes::BlackScholes;
use super::config::{OptionDefinition, OptionType, RealOptionsConfig, ValuationMethod};
use crate::core::precision::money_whole;
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
    /// What the option adds over the traditional NPV: for a defer option, what
    /// waiting adds; for any other, its value. `None` without a traditional NPV
    /// for a defer option.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_added: Option<f64>,
}

/// Complete options analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionsResult {
    /// Analysis name
    pub name: String,
    /// How the options were valued
    #[serde(default)]
    pub method: ValuationMethod,
    /// Underlying asset parameters
    pub underlying: UnderlyingResult,
    /// Traditional NPV (without options), if given
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traditional_npv: Option<f64>,
    /// Individual option results
    pub options: std::collections::BTreeMap<String, OptionResult>,
    /// The options' names, in the order the file lists them
    #[serde(default)]
    pub option_order: Vec<String>,
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
    ///
    /// # Panics
    ///
    /// Cannot panic: the result is text and numbers, which YAML holds.
    #[must_use]
    pub fn to_yaml(&self) -> String {
        yaml_serde::to_string(&crate::core::precision::Rounded(self))
            .expect("an options result is text and numbers")
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
    /// # Panics
    ///
    /// Cannot panic: `new` refuses a configuration without options.
    #[must_use]
    pub fn analyze(&self) -> OptionsResult {
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
        let decision = if needs_npv {
            "UNDECIDED: set traditional_npv to value expand, abandon, contract, switch or compound options".to_string()
        } else {
            // `validate` asks for an option, and each has a project value here
            let (name, value) = best.as_ref().expect("an option with a project value");
            if *value > 0.0 {
                format!("ACCEPT (with {name})")
            } else {
                "REJECT".to_string()
            }
        };

        let recommendation = self.generate_recommendation(&option_results, best.as_ref());

        OptionsResult {
            name: self.config.name.clone(),
            method: self.config.method,
            underlying: UnderlyingResult {
                current_value: self.config.underlying.current_value,
                volatility: self.config.underlying.volatility,
                risk_free_rate: self.config.underlying.risk_free_rate,
                time_horizon: self.config.underlying.time_horizon,
            },
            traditional_npv: self.traditional_npv,
            options: option_results,
            option_order: self.config.options.iter().map(|o| o.name.clone()).collect(),
            best_option: best.as_ref().map(|(name, _)| name.clone()),
            project_value_with_options: best.map(|(_, value)| value),
            decision,
            recommendation,
        }
    }

    /// Value a single option
    fn value_option(&self, option: &OptionDefinition) -> OptionResult {
        let value = match self.config.method {
            ValuationMethod::BlackScholes => self.value_with_black_scholes(option),
            ValuationMethod::Binomial => {
                self.value_with_binomial(option, self.config.binomial_steps)
            },
            // Not a simulation yet: a binomial tree of 200 steps, whatever
            // `binomial_steps` says (`monte_carlo_iterations` is not used)
            ValuationMethod::MonteCarlo => self.value_with_binomial(option, 200),
        };

        let trigger = self.determine_trigger(option);
        let project_value = match option.option_type {
            OptionType::Defer => Some(value),
            _ => self.traditional_npv.map(|npv| npv + value),
        };
        let value_added = match option.option_type {
            OptionType::Defer => self.traditional_npv.map(|npv| value - npv),
            _ => Some(value),
        };

        OptionResult {
            name: option.name.clone(),
            option_type: option.option_type,
            value,
            probability_exercise: None, // Would be populated from simulation
            optimal_trigger: trigger,
            project_value,
            value_added,
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
                self.value_with_binomial(option, self.config.binomial_steps)
            },
        }
    }

    /// Value option using a binomial tree of `steps` steps
    fn value_with_binomial(&self, option: &OptionDefinition, steps: u16) -> f64 {
        let u = &self.config.underlying;

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

    /// Determine optimal trigger condition
    fn determine_trigger(&self, option: &OptionDefinition) -> Option<String> {
        let u = &self.config.underlying;

        match option.option_type {
            OptionType::Defer => {
                let trigger_value = option.exercise_cost * 1.1; // 10% above exercise cost
                Some(format!("value > {}", money_whole(trigger_value)))
            },
            OptionType::Expand => {
                let trigger_value = option.exercise_cost * 2.0; // Good ROI on expansion
                Some(format!("value > {}", money_whole(trigger_value)))
            },
            OptionType::Abandon => {
                let trigger_value = option.salvage_value * 1.2;
                Some(format!("value < {}", money_whole(trigger_value)))
            },
            OptionType::Contract => {
                let trigger_value = u.current_value * option.contraction_factor;
                Some(format!("value < {}", money_whole(trigger_value)))
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
            parts.push(format!(
                "Best option: {name}, project value {}",
                money_whole(*value)
            ));
            if let Some(npv) = self.traditional_npv {
                if npv < 0.0 && *value > 0.0 {
                    parts.push(format!(
                        "Traditional NPV is negative ({}), but with {name} the project is worth {}",
                        money_whole(npv),
                        money_whole(*value)
                    ));
                }
                if options
                    .get(name)
                    .is_some_and(|o| o.option_type == OptionType::Defer)
                {
                    let premium = value - npv.max(0.0);
                    parts.push(format!(
                        "Keeping the right to wait is worth {} more than deciding now",
                        money_whole(premium)
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

        let result = engine.analyze();

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

        let result = engine.analyze();

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
        let result = engine.analyze();

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
        let result = engine.analyze();
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
        let result = engine.analyze();
        let json = result.to_json().unwrap();

        assert!(json.contains("\"project_value_with_options\""));
        assert!(json.contains("\"options\""));
    }

    /// A project of 10M with 30% volatility, 5% rates, 3 years and a 1%
    /// dividend yield
    fn underlying() -> UnderlyingConfig {
        UnderlyingConfig::new(10_000_000.0, 0.30, 0.05, 3.0).with_dividend_yield(0.01)
    }

    fn analyze(method: ValuationMethod, option: OptionDefinition, npv: f64) -> OptionsResult {
        let config = RealOptionsConfig::new("p", underlying())
            .with_method(method)
            .with_option(option)
            .with_binomial_steps(50);
        RealOptionsEngine::new(config)
            .unwrap()
            .with_traditional_npv(npv)
            .analyze()
    }

    /// Black-Scholes with a dividend yield (Merton), against mpmath at 40
    /// digits, to within 1e-12 of the project's value: expanding by half
    /// is a call on 5M at 4M, abandoning for 3M a put on 10M at 3M, and
    /// contracting by a fifth for savings of 1M a put on 2M at 1M
    #[test]
    fn black_scholes_values_each_option_as_its_call_or_put() {
        for (option, want) in [
            (
                OptionDefinition::expand("e", 1.5, 4_000_000.0),
                1_727_720.784_089_318_4,
            ),
            (
                OptionDefinition::abandon("a", 3_000_000.0),
                4_363.531_923_619_636,
            ),
            (
                OptionDefinition::contract("c", 0.8, 1_000_000.0),
                16_534.191_782_498_034,
            ),
        ] {
            let name = option.name.clone();
            let value = analyze(ValuationMethod::BlackScholes, option, 0.0).options[&name].value;
            assert!(
                (value - want).abs() <= 1e-12 * 10_000_000.0,
                "{name}: {value}"
            );
        }
    }

    /// Switch and compound options have no closed form here: under
    /// Black-Scholes they are valued on the binomial tree. A switch is the
    /// larger of expanding by a fifth and contracting by a fifth (whatever its
    /// own factors), and a compound option half an expansion plus waiting a
    /// year at half the cost: approximations, stated here as they are.
    #[test]
    fn switch_and_compound_are_approximated_on_the_tree() {
        let u = underlying();
        let tree = BinomialTree::new(10_000_000.0, 10_000_000.0, 0.05, 0.30, 3.0, 50)
            .with_dividend_yield(u.dividend_yield);
        let base = OptionDefinition::expand("x", 1.5, 2_000_000.0);
        let switch = OptionDefinition {
            option_type: OptionType::Switch,
            ..base.clone()
        };
        let compound = OptionDefinition {
            option_type: OptionType::Compound,
            ..base
        };
        let switch_value = tree
            .expand_option_value(1.2, 2_000_000.0)
            .max(tree.contract_option_value(0.8, 2_000_000.0));
        let compound_value = tree
            .expand_option_value(1.5, 2_000_000.0)
            .mul_add(0.5, tree.defer_option_value(1.0, 1_000_000.0));
        for (method, option, want) in [
            (ValuationMethod::BlackScholes, switch.clone(), switch_value),
            (ValuationMethod::Binomial, switch, switch_value),
            (ValuationMethod::Binomial, compound, compound_value),
        ] {
            let value = analyze(method, option, 0.0).options["x"].value;
            assert_eq!(
                value.to_bits(),
                want.to_bits(),
                "{method:?}: {value} {want}"
            );
        }
    }

    /// `method: monte_carlo` is not a simulation yet: it is the binomial tree
    /// with 200 steps
    #[test]
    fn monte_carlo_is_the_binomial_tree_with_200_steps() {
        let option = OptionDefinition::defer("d", 2.0, 8_000_000.0);
        let config = |method, steps| {
            RealOptionsConfig::new("p", underlying())
                .with_method(method)
                .with_option(option.clone())
                .with_binomial_steps(steps)
        };
        let value = |method, steps| {
            RealOptionsEngine::new(config(method, steps))
                .unwrap()
                .analyze()
                .options["d"]
                .value
        };
        assert_eq!(
            value(ValuationMethod::MonteCarlo, 50).to_bits(),
            value(ValuationMethod::Binomial, 200).to_bits()
        );
    }

    /// Of two options worth as much, the first by name is the best; a project
    /// worth less than nothing with its best option is rejected
    #[test]
    fn a_tie_goes_to_the_first_name_and_a_losing_project_is_rejected() {
        let config = RealOptionsConfig::new("p", underlying())
            .with_option(OptionDefinition::abandon("b", 3_000_000.0))
            .with_option(OptionDefinition::abandon("a", 3_000_000.0));
        let result = RealOptionsEngine::new(config)
            .unwrap()
            .with_traditional_npv(-1_000_000_000.0)
            .analyze();
        assert_eq!(
            (result.best_option.as_deref(), result.decision.as_str()),
            (Some("a"), "REJECT")
        );
    }
}
