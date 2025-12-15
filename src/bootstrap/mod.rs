//! Bootstrap Resampling Module (Enterprise Only)
//!
//! Non-parametric uncertainty quantification via resampling:
//! - Resample from historical data with replacement
//! - No distribution assumptions required
//! - Confidence intervals from empirical distribution
//! - R boot package validated calculations
//!
//! # Example
//!
//! ```yaml
//! bootstrap:
//!   iterations: 10000
//!   confidence_levels: [0.90, 0.95, 0.99]
//!   seed: 12345
//!
//!   data:
//!     historical_returns: [0.05, -0.02, 0.08, 0.03, -0.05, 0.12, 0.01, -0.01, 0.06, 0.04]
//!
//!   statistic: mean  # or median, std, var, percentile
//! ```
//!
//! See ADR for architecture decisions.

pub mod config;
pub mod engine;

// Re-exports
pub use config::{BootstrapConfig, BootstrapStatistic};
pub use engine::{BootstrapEngine, BootstrapResult, ConfidenceInterval};

#[cfg(test)]
mod tests;
