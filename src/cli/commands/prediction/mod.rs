//! Prediction Methods CLI Commands
//!
//! CLI handlers for advanced forecasting methods:
//! - scenarios: Run scenario analysis (Base/Bull/Bear)
//! - `decision_tree`: Analyze decision trees with backward induction
//! - `real_options`: Value managerial flexibility (defer/expand/abandon)
//! - tornado: Generate sensitivity tornado diagrams
//! - bootstrap: Non-parametric confidence intervals via resampling
//! - bayesian: Bayesian network inference and queries

mod bayesian;
mod bootstrap;
mod decision_tree;
mod real_options;
mod scenarios;
mod tornado;

pub use bayesian::{bayesian, bayesian_core};
pub use bootstrap::{bootstrap, bootstrap_core};
pub use decision_tree::{decision_tree, decision_tree_core};
pub use real_options::{real_options, real_options_core};
pub use scenarios::{scenarios, scenarios_core};
pub use tornado::{tornado, tornado_core};

use crate::core::precision::Rounded;
use crate::error::{ForgeError, ForgeResult};
use std::path::Path;

/// Serialises an analytics result for `--output`: JSON for a `.json` path,
/// YAML otherwise, with numbers to 15 significant digits (`core::precision`).
/// (This used to write a Rust debug dump, which no tool can read.)
fn serialize_for_file<T: serde::Serialize>(value: &T, path: &Path) -> ForgeResult<String> {
    if path.extension().and_then(|e| e.to_str()) == Some("json") {
        serde_json::to_string_pretty(&Rounded(value))
            .map_err(|e| ForgeError::Validation(format!("JSON error: {e}")))
    } else {
        yaml_serde::to_string(&Rounded(value))
            .map_err(|e| ForgeError::Validation(format!("YAML error: {e}")))
    }
}
