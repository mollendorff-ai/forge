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
//! use royalbit_forge::parser::parse_model;
//! use royalbit_forge::core::ArrayCalculator;
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
//! # Ok::<(), royalbit_forge::error::ForgeError>(())
//! ```

// Demo modules (always included)
pub mod cli;
pub mod core;
pub mod error;
pub mod excel;
pub mod functions;
pub mod parser;
pub mod types;
pub mod writer;

// Enterprise-only modules (excluded from demo builds)
#[cfg(not(feature = "demo"))]
pub mod api;
#[cfg(not(feature = "demo"))]
pub mod bayesian;
#[cfg(not(feature = "demo"))]
pub mod bootstrap;
#[cfg(not(feature = "demo"))]
pub mod decision_trees;
#[cfg(not(feature = "demo"))]
pub mod mcp;
#[cfg(not(feature = "demo"))]
pub mod monte_carlo;
#[cfg(not(feature = "demo"))]
pub mod real_options;
#[cfg(not(feature = "demo"))]
pub mod scenarios;
#[cfg(not(feature = "demo"))]
pub mod tornado;

// Re-export commonly used types
pub use error::{ForgeError, ForgeResult};
pub use types::{Column, ColumnValue, ParsedModel, Table, Variable};
