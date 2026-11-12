//! forge-e2e-gnumeric: E2E validation of forge against Gnumeric.
//!
//! Validates Excel-compatible functions by comparing forge output
//! against Gnumeric (via ssconvert) at runtime.

pub mod engine;
pub mod excel;
pub mod runner;
pub mod types;
