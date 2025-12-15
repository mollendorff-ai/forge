//! Real Options Analysis Module (Enterprise Only)
//!
//! Values managerial flexibility using options pricing theory:
//! - Defer option (wait before investing)
//! - Expand option (scale up if successful)
//! - Contract option (scale down if weak)
//! - Abandon option (exit and recover salvage)
//! - Switch option (change inputs/outputs)
//! - Black-Scholes and Binomial Tree methods
//! - QuantLib/RustQuant validated calculations
//!
//! # Example
//!
//! ```yaml
//! real_options:
//!   name: "Phased Factory Investment"
//!   method: binomial
//!
//!   underlying:
//!     current_value: 10000000
//!     volatility: 0.30
//!     risk_free_rate: 0.05
//!     time_horizon: 3
//!
//!   options:
//!     - type: defer
//!       name: "Wait up to 2 years"
//!       max_deferral: 2
//!       exercise_cost: 8000000
//!
//!     - type: abandon
//!       name: "Sell assets"
//!       salvage_value: 3000000
//! ```
//!
//! See ADR-020 for architecture decisions.

pub mod binomial;
pub mod black_scholes;
pub mod config;
pub mod engine;

// Re-exports
pub use binomial::BinomialTree;
pub use black_scholes::BlackScholes;
pub use config::{
    OptionDefinition, OptionType, RealOptionsConfig, UnderlyingConfig, ValuationMethod,
};
pub use engine::{OptionsResult, RealOptionsEngine};

#[cfg(test)]
mod tests;
