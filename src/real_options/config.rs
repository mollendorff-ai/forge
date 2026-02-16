//! Real Options Configuration
//!
//! Handles parsing and validation of real options definitions from YAML.

use serde::{Deserialize, Serialize};

/// Type of real option
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OptionType {
    /// Wait before investing
    Defer,
    /// Scale up if successful
    Expand,
    /// Scale down if weak
    Contract,
    /// Exit and recover salvage
    Abandon,
    /// Change inputs/outputs
    Switch,
    /// Option on an option
    Compound,
}

/// Valuation method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ValuationMethod {
    /// Closed-form Black-Scholes (European options)
    BlackScholes,
    /// Binomial tree (American options, path-dependent)
    #[default]
    Binomial,
    /// Monte Carlo simulation (complex/exotic options)
    MonteCarlo,
}

/// Definition of a single option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionDefinition {
    /// Type of option
    #[serde(rename = "type")]
    pub option_type: OptionType,
    /// Human-readable name
    pub name: String,
    /// Cost to exercise the option
    #[serde(default)]
    pub exercise_cost: f64,
    /// Salvage value (for abandon options)
    #[serde(default)]
    pub salvage_value: f64,
    /// Maximum deferral period in years (for defer options)
    #[serde(default)]
    pub max_deferral: f64,
    /// Expansion factor (for expand options)
    #[serde(default = "default_expansion_factor")]
    pub expansion_factor: f64,
    /// Contraction factor (for contract options)
    #[serde(default = "default_contraction_factor")]
    pub contraction_factor: f64,
}

const fn default_expansion_factor() -> f64 {
    1.5
}

const fn default_contraction_factor() -> f64 {
    0.5
}

impl OptionDefinition {
    /// Create a defer option
    #[must_use]
    pub fn defer(name: &str, max_deferral: f64, exercise_cost: f64) -> Self {
        Self {
            option_type: OptionType::Defer,
            name: name.to_string(),
            exercise_cost,
            salvage_value: 0.0,
            max_deferral,
            expansion_factor: 1.0,
            contraction_factor: 1.0,
        }
    }

    /// Create an expand option
    #[must_use]
    pub fn expand(name: &str, expansion_factor: f64, exercise_cost: f64) -> Self {
        Self {
            option_type: OptionType::Expand,
            name: name.to_string(),
            exercise_cost,
            salvage_value: 0.0,
            max_deferral: 0.0,
            expansion_factor,
            contraction_factor: 1.0,
        }
    }

    /// Create an abandon option
    #[must_use]
    pub fn abandon(name: &str, salvage_value: f64) -> Self {
        Self {
            option_type: OptionType::Abandon,
            name: name.to_string(),
            exercise_cost: 0.0,
            salvage_value,
            max_deferral: 0.0,
            expansion_factor: 1.0,
            contraction_factor: 1.0,
        }
    }

    /// Create a contract option
    #[must_use]
    pub fn contract(name: &str, contraction_factor: f64, cost_savings: f64) -> Self {
        Self {
            option_type: OptionType::Contract,
            name: name.to_string(),
            exercise_cost: -cost_savings, // Negative cost = savings
            salvage_value: 0.0,
            max_deferral: 0.0,
            expansion_factor: 1.0,
            contraction_factor,
        }
    }
}

/// Underlying asset configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnderlyingConfig {
    /// Present value of project cash flows
    pub current_value: f64,
    /// Annual volatility of value
    pub volatility: f64,
    /// Annual risk-free rate
    pub risk_free_rate: f64,
    /// Time horizon in years
    pub time_horizon: f64,
    /// Dividend yield (continuous)
    #[serde(default)]
    pub dividend_yield: f64,
}

impl UnderlyingConfig {
    /// Create a new underlying configuration
    #[must_use]
    pub const fn new(
        current_value: f64,
        volatility: f64,
        risk_free_rate: f64,
        time_horizon: f64,
    ) -> Self {
        Self {
            current_value,
            volatility,
            risk_free_rate,
            time_horizon,
            dividend_yield: 0.0,
        }
    }

    /// Add dividend yield
    #[must_use]
    pub const fn with_dividend_yield(mut self, yield_rate: f64) -> Self {
        self.dividend_yield = yield_rate;
        self
    }

    /// Validate configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying parameters are out of range (e.g.,
    /// non-positive value, volatility, or time horizon).
    pub fn validate(&self) -> Result<(), String> {
        if self.current_value <= 0.0 {
            return Err("Current value must be positive".to_string());
        }
        if self.volatility <= 0.0 || self.volatility > 2.0 {
            return Err("Volatility must be between 0 and 200%".to_string());
        }
        if self.risk_free_rate < 0.0 || self.risk_free_rate > 1.0 {
            return Err("Risk-free rate must be between 0% and 100%".to_string());
        }
        if self.time_horizon <= 0.0 {
            return Err("Time horizon must be positive".to_string());
        }
        Ok(())
    }
}

/// Configuration for real options analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealOptionsConfig {
    /// Name of the analysis
    #[serde(default)]
    pub name: String,
    /// Valuation method
    #[serde(default)]
    pub method: ValuationMethod,
    /// Underlying asset configuration
    pub underlying: UnderlyingConfig,
    /// Options to value
    #[serde(default)]
    pub options: Vec<OptionDefinition>,
    /// Number of steps in binomial tree
    #[serde(default = "default_binomial_steps")]
    pub binomial_steps: usize,
    /// Number of Monte Carlo iterations
    #[serde(default = "default_mc_iterations")]
    pub monte_carlo_iterations: usize,
    /// Random seed for reproducibility
    pub seed: Option<u64>,
}

const fn default_binomial_steps() -> usize {
    100
}

const fn default_mc_iterations() -> usize {
    10000
}

impl RealOptionsConfig {
    /// Create a new configuration
    #[must_use]
    pub fn new(name: &str, underlying: UnderlyingConfig) -> Self {
        Self {
            name: name.to_string(),
            method: ValuationMethod::Binomial,
            underlying,
            options: Vec::new(),
            binomial_steps: 100,
            monte_carlo_iterations: 10000,
            seed: None,
        }
    }

    /// Set valuation method
    #[must_use]
    pub const fn with_method(mut self, method: ValuationMethod) -> Self {
        self.method = method;
        self
    }

    /// Add an option
    #[must_use]
    pub fn with_option(mut self, option: OptionDefinition) -> Self {
        self.options.push(option);
        self
    }

    /// Set binomial steps
    #[must_use]
    pub const fn with_binomial_steps(mut self, steps: usize) -> Self {
        self.binomial_steps = steps;
        self
    }

    /// Set seed
    #[must_use]
    pub const fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Validate configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying is invalid, no options are defined,
    /// or parameters are zero.
    pub fn validate(&self) -> Result<(), String> {
        self.underlying.validate()?;

        if self.options.is_empty() {
            return Err("At least one option must be defined".to_string());
        }

        if self.binomial_steps == 0 {
            return Err("Binomial steps must be positive".to_string());
        }

        if self.monte_carlo_iterations == 0 {
            return Err("Monte Carlo iterations must be positive".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
// Financial math: exact float comparison validated against Excel/Gnumeric/R
#[allow(clippy::float_cmp)]
mod config_tests {
    use super::*;

    #[test]
    fn test_underlying_validation() {
        let underlying = UnderlyingConfig::new(10_000_000.0, 0.30, 0.05, 3.0);
        assert!(underlying.validate().is_ok());

        let bad_value = UnderlyingConfig::new(-100.0, 0.30, 0.05, 3.0);
        assert!(bad_value.validate().is_err());

        let bad_vol = UnderlyingConfig::new(100.0, -0.1, 0.05, 3.0);
        assert!(bad_vol.validate().is_err());
    }

    #[test]
    fn test_config_builder() {
        let config = RealOptionsConfig::new(
            "Factory Investment",
            UnderlyingConfig::new(10_000_000.0, 0.30, 0.05, 3.0),
        )
        .with_method(ValuationMethod::Binomial)
        .with_option(OptionDefinition::defer("Wait 2 years", 2.0, 8_000_000.0))
        .with_option(OptionDefinition::abandon("Sell assets", 3_000_000.0))
        .with_binomial_steps(50);

        assert!(config.validate().is_ok());
        assert_eq!(config.options.len(), 2);
    }

    #[test]
    fn test_option_types() {
        let defer = OptionDefinition::defer("Wait", 2.0, 1_000_000.0);
        assert_eq!(defer.option_type, OptionType::Defer);

        let expand = OptionDefinition::expand("Scale up", 1.5, 500_000.0);
        assert_eq!(expand.option_type, OptionType::Expand);
        assert_eq!(expand.expansion_factor, 1.5);

        let abandon = OptionDefinition::abandon("Exit", 200_000.0);
        assert_eq!(abandon.option_type, OptionType::Abandon);
        assert_eq!(abandon.salvage_value, 200_000.0);
    }
}
