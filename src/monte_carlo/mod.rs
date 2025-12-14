//! Monte Carlo Simulation Module (Enterprise Only)
//!
//! Provides probabilistic analysis capabilities for FP&A:
//! - Distribution sampling (Normal, Triangular, Uniform, PERT, Lognormal)
//! - Latin Hypercube and Monte Carlo sampling methods
//! - Statistical output (percentiles, probability thresholds)
//! - Sensitivity analysis via correlation coefficients
//!
//! # Example
//!
//! ```yaml
//! monte_carlo:
//!   enabled: true
//!   iterations: 10000
//!   sampling: latin_hypercube
//!   seed: 12345
//!   outputs:
//!     - variable: valuation.npv
//!       percentiles: [10, 50, 90]
//!       threshold: "> 0"
//!
//! assumptions:
//!   revenue_growth: =MC.Normal(0.15, 0.05)
//!   initial_cost: =MC.Triangular(80000, 100000, 150000)
//! ```
//!
//! See ADR-016 for architecture decisions.

pub mod config;
pub mod distributions;
pub mod engine;
pub mod sampler;
pub mod statistics;

// Re-exports
pub use config::MonteCarloConfig;
pub use distributions::{Distribution, DistributionType};
pub use engine::{MonteCarloEngine, SimulationResult};
pub use sampler::{Sampler, SamplingMethod};
pub use statistics::Statistics;

#[cfg(test)]
mod tests;
