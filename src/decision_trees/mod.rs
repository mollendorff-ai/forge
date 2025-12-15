//! Decision Trees Module (Enterprise Only)
//!
//! Provides sequential decision modeling for FP&A:
//! - Decision nodes (choices we control)
//! - Chance nodes (outcomes we don't control)
//! - Terminal nodes with values
//! - Backward induction for optimal path
//! - SciPy/NumPy validated calculations
//!
//! # Example
//!
//! ```yaml
//! decision_tree:
//!   name: "R&D Investment Decision"
//!
//!   root:
//!     type: decision
//!     name: "Invest in R&D?"
//!     branches:
//!       invest:
//!         cost: 2000000
//!         next: tech_outcome
//!       dont_invest:
//!         value: 0
//!
//!   nodes:
//!     tech_outcome:
//!       type: chance
//!       name: "Technology works?"
//!       branches:
//!         success:
//!           probability: 0.60
//!           next: commercialize_decision
//!         failure:
//!           probability: 0.40
//!           value: -2000000
//! ```
//!
//! See ADR-019 for architecture decisions.

pub mod config;
pub mod engine;

// Re-exports
pub use config::{Branch, DecisionTreeConfig, Node, NodeType};
pub use engine::{DecisionTreeEngine, NodeResult, TreeResult};

#[cfg(test)]
mod tests;
