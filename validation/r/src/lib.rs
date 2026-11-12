//! forge-e2e-r: E2E validation of forge analytics against R.
//!
//! Validates statistical and analytics functions by comparing forge output
//! against R validators at runtime.

pub mod analytics;
pub mod cli_runner;
pub mod r_validator;
pub mod stats;
pub mod types;
