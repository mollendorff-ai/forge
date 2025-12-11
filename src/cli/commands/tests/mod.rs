//! Tests for CLI commands
//!
//! Organized by command/functionality for maintainability.

mod common;

mod audit;
mod break_even;
mod calculate;
mod compare;
mod export;
mod functions;
mod goal_seek;
mod import;
mod integration;
mod parsing;
mod scenario;
#[cfg(feature = "full")]
mod schema;
mod sensitivity;
#[cfg(feature = "full")]
mod upgrade;
mod utils;
mod validate;
mod variance;
mod watch;
