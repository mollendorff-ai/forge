//! CLI command handlers

pub mod commands;

pub use commands::{
    audit, break_even, calculate, compare, examples, export, functions, goal_seek, import, schema,
    sensitivity, update, validate, variance, watch,
};

pub use commands::upgrade;

pub use commands::simulate;

pub use commands::{bayesian, bootstrap, decision_tree, real_options, scenarios, tornado};

// Core function re-exports (structured results, no printing)
pub use commands::{
    audit_core, bayesian_core, bootstrap_core, calculate_core, compare_core, decision_tree_core,
    examples_core, export_core, functions_core, goal_seek_core, import_core, real_options_core,
    scenarios_core, schema_core, sensitivity_core, simulate_core, tornado_core, validate_core,
    variance_core,
};
