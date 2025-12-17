//! CLI command handlers

pub mod commands;

pub use commands::{
    audit, break_even, calculate, compare, export, functions, goal_seek, import, sensitivity,
    validate, variance, watch,
};

#[cfg(not(feature = "demo"))]
pub use commands::upgrade;

#[cfg(not(feature = "demo"))]
pub use commands::simulate;

#[cfg(not(feature = "demo"))]
pub use commands::{bayesian, bootstrap, decision_tree, real_options, scenarios, tornado};
