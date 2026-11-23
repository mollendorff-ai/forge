//! CLI command handlers

#[cfg(feature = "cli")]
pub mod args;
pub mod commands;
pub mod console;
#[cfg(feature = "cli")]
mod run;

pub use console::Console;
#[cfg(all(test, feature = "cli"))]
pub use run::{forge, forge_at_width, Ran};
#[cfg(feature = "cli")]
pub use run::{run_from, starts_a_server};

pub use commands::{
    audit, break_even, calculate, compare, functions, game, goal_seek, schema, sensitivity,
    sensitivity_in_format, validate, variance,
};

// Watch, update, export and import need the `cli` feature
#[cfg(feature = "cli")]
pub use commands::{examples, export, import, update, watch};

pub use commands::upgrade;

pub use commands::{report, ReportFormat, ReportOptions};

pub use commands::{probability_seek, simulate};

pub use commands::{
    bayesian, bootstrap, decision_tree, real_options, scenarios, templates, tornado,
};

// Core function re-exports (structured results, no printing)
pub use commands::calculate_plan_core;
#[cfg(feature = "cli")]
pub use commands::export_buffer_core;
pub use commands::{
    audit_core, bayesian_core, bootstrap_core, calculate_core, compare_core, comparison_json,
    decision_tree_core, examples_core, functions_core, game_core, goal_seek_core,
    pct_is_meaningful, real_options_core, scenarios_core, schema_core, sensitivity_core,
    simulate_core, templates_core, tornado_core, validate_core, validate_core_with, variance_core,
    SimulateOptions,
};
#[cfg(feature = "excel")]
pub use commands::{
    export_bytes_core, export_core, import_core, import_text_core, ExportBytes, ImportText,
};
pub use commands::{probability_seek_core, SeekGoal, SeekResult};
