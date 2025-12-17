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
#[cfg(not(feature = "demo"))]
mod schema;
mod sensitivity;
#[cfg(not(feature = "demo"))]
mod upgrade;
mod utils;
mod validate;
mod variance;
mod watch;
