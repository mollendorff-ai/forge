//! Forge - YAML formula calculator with Excel-style cross-file references
//!
//! This library provides functionality to parse YAML files containing formulas,
//! calculate them in dependency order, and update values across multiple files.
//!
//! # Features
//!
//! - Excel-style formulas in YAML files (SUM, AVERAGE, IF, etc.)
//! - Array model (v1.0.0) for Excel-compatible column-based data
//! - Scalar model (v0.2.0) for backwards compatibility
//! - Cross-file references using `@alias.variable` syntax
//! - Automatic dependency resolution
//! - Multi-file updates (Excel-style behavior)
//! - JSON Schema validation (v1.0.0)
//! - Type-safe homogeneous arrays (Number, Text, Date, Boolean)
//!
//! # Model Versions
//!
//! ## v1.0.0 - Array Model (Excel-compatible)
//! Column arrays that map directly to Excel columns for trivial export.
//! Supports row-wise formulas, aggregations, and type-safe arrays.
//!
//! ## v0.2.0 - Scalar Model (Backwards compatible)
//! Individual variables with {value, formula} pattern.
//!
//! # Example (v1.0.0)
//!
//! ```no_run
//! use royalbit_forge::parser::parse_model;
//! use std::path::Path;
//!
//! let path = Path::new("model.yaml");
//! let model = parse_model(path)?;
//!
//! println!("Version: {:?}", model.version);
//! println!("Tables: {}", model.tables.len());
//! println!("Scalars: {}", model.scalars.len());
//! # Ok::<(), royalbit_forge::error::ForgeError>(())
//! ```
//!
//! # Example (v0.2.0)
//!
//! ```no_run
//! use royalbit_forge::parser::parse_yaml_with_includes;
//! use royalbit_forge::core::Calculator;
//! use std::path::Path;
//!
//! let path = Path::new("model.yaml");
//! let parsed = parse_yaml_with_includes(path)?;
//! let mut calculator = Calculator::new(parsed.variables.clone());
//! let results = calculator.calculate_all()?;
//! # Ok::<(), royalbit_forge::error::ForgeError>(())
//! ```

pub mod cli;
pub mod core;
pub mod error;
pub mod parser;
pub mod types;
pub mod writer;

// Re-export commonly used types
pub use error::{ForgeError, ForgeResult};
pub use types::{
    Column, ColumnValue, ForgeVersion, Include, ParsedModel, ParsedYaml, Table, Variable,
};
