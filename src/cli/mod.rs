//! CLI command handlers

pub mod commands;

pub use commands::{
    audit, break_even, calculate, compare, examples, functions, goal_seek, schema, sensitivity,
    validate, variance,
};

// Watch, update, export and import need the `cli` feature
#[cfg(feature = "cli")]
pub use commands::{export, import, update, watch};

pub use commands::upgrade;

pub use commands::{report, ReportFormat, ReportOptions};

pub use commands::{probability_seek, simulate};

pub use commands::{bayesian, bootstrap, decision_tree, real_options, scenarios, tornado};

// Core function re-exports (structured results, no printing)
#[cfg(feature = "cli")]
pub use commands::export_buffer_core;
pub use commands::{
    audit_core, bayesian_core, bootstrap_core, calculate_core, compare_core, decision_tree_core,
    examples_core, functions_core, goal_seek_core, real_options_core, scenarios_core, schema_core,
    sensitivity_core, simulate_core, tornado_core, validate_core, variance_core, SimulateOptions,
};
#[cfg(feature = "excel")]
pub use commands::{
    export_bytes_core, export_core, import_core, import_text_core, ExportBytes, ImportText,
};
pub use commands::{probability_seek_core, SeekGoal, SeekResult};
