//! Bayesian Networks Module (Enterprise Only)
//!
//! Probabilistic graphical models for causal reasoning:
//! - Directed Acyclic Graph (DAG) structure
//! - Conditional probability tables (CPTs)
//! - Belief propagation inference
//! - pgmpy validated calculations
//!
//! # Example
//!
//! ```yaml
//! bayesian_network:
//!   name: "Credit Risk Model"
//!
//!   nodes:
//!     economic_conditions:
//!       type: discrete
//!       states: [good, neutral, bad]
//!       prior: [0.3, 0.5, 0.2]
//!
//!     company_revenue:
//!       type: discrete
//!       states: [high, medium, low]
//!       parents: [economic_conditions]
//!       cpt:
//!         good: [0.6, 0.3, 0.1]
//!         neutral: [0.3, 0.5, 0.2]
//!         bad: [0.1, 0.3, 0.6]
//!
//!     default_probability:
//!       type: discrete
//!       states: [low, medium, high]
//!       parents: [company_revenue]
//!       cpt:
//!         high: [0.8, 0.15, 0.05]
//!         medium: [0.4, 0.4, 0.2]
//!         low: [0.1, 0.3, 0.6]
//! ```
//!
//! See ADR for architecture decisions.

pub mod config;
pub mod engine;
pub mod inference;

// Re-exports
pub use config::{BayesianConfig, BayesianNode, NodeType as BayesianNodeType};
pub use engine::{BayesianEngine, BayesianResult};
pub use inference::BeliefPropagation;

#[cfg(test)]
mod tests;
