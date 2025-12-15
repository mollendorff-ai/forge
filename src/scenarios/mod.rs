//! Scenario Analysis Module (Enterprise Only)
//!
//! Provides discrete scenario modeling for FP&A:
//! - Probability-weighted scenarios (Base/Bull/Bear)
//! - Expected value calculation across scenarios
//! - Integration with Monte Carlo for continuous uncertainty within scenarios
//! - R-validated weighted mean calculations
//!
//! # Example
//!
//! ```yaml
//! scenarios:
//!   base_case:
//!     probability: 0.50
//!     description: "Market grows 5%, we maintain share"
//!     scalars:
//!       revenue_growth: 0.05
//!       market_share: 0.15
//!
//!   bull_case:
//!     probability: 0.30
//!     description: "Competitor exits, we capture share"
//!     scalars:
//!       revenue_growth: 0.15
//!       market_share: 0.25
//!
//!   bear_case:
//!     probability: 0.20
//!     description: "Recession hits, market contracts"
//!     scalars:
//!       revenue_growth: -0.10
//!       market_share: 0.12
//! ```
//!
//! See ADR-018 for architecture decisions.

pub mod config;
pub mod engine;

// Re-exports
pub use config::{ScenarioConfig, ScenarioDefinition};
pub use engine::{ScenarioEngine, ScenarioResult, ScenarioResults};

#[cfg(test)]
mod tests;
