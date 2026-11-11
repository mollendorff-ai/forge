//! Forge - YAML formula calculator with Excel-style arrays
//!
//! This library provides functionality to parse YAML files containing formulas,
//! calculate them in dependency order, and update values.
//!
//! # Features
//!
//! - Excel-style formulas in YAML files (SUM, AVERAGE, IF, etc.)
//! - Array model for Excel-compatible column-based data
//! - JSON Schema validation
//! - Type-safe homogeneous arrays (Number, Text, Date, Boolean)
//! - Excel import/export
//! - Multi-document YAML support
//!
//! # Example
//!
//! ```no_run
//! use mollendorff_forge::parser::parse_model;
//! use mollendorff_forge::core::ArrayCalculator;
//! use std::path::Path;
//!
//! let path = Path::new("model.yaml");
//! let model = parse_model(path)?;
//!
//! println!("Tables: {}", model.tables.len());
//! println!("Scalars: {}", model.scalars.len());
//!
//! let calculator = ArrayCalculator::new(model);
//! let result = calculator.calculate_all()?;
//! # Ok::<(), mollendorff_forge::error::ForgeError>(())
//! ```

pub mod core;
pub mod error;
pub mod functions;
pub mod parser;
pub mod types;
pub mod writer;

// Analysis modules
pub mod bayesian;
pub mod bootstrap;
pub mod decision_trees;
pub mod monte_carlo;
pub mod real_options;
pub mod scenarios;
pub mod tornado;

// Excel export and import (feature `excel`, part of the default `cli`)
#[cfg(feature = "excel")]
pub mod excel;

// The commands. Their core functions (`calculate_core`, `simulate_core`, ...)
// build without the `cli` feature, for a browser (ADR-055); watch, update,
// export and import need it.
pub mod cli;

// The MCP server and the REST API (feature `cli`, the default)
#[cfg(feature = "cli")]
pub mod api;
#[cfg(feature = "cli")]
pub mod inline_model;
#[cfg(feature = "cli")]
pub mod mcp;

#[cfg(test)]
mod config_strictness_tests;

/// The version of Forge, as in Cargo.toml (`10.0.0-rc.13`)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Re-export commonly used types
pub use error::{ForgeError, ForgeResult};
pub use types::{Column, ColumnValue, ParsedModel, Table, Variable};
