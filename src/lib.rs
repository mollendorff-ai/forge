//! Mouvify Forge - YAML formula calculator with Excel-style cross-file references
//!
//! This library provides functionality to parse YAML files containing formulas,
//! calculate them in dependency order, and update values across multiple files.
//!
//! # Features
//!
//! - Excel-style formulas in YAML files
//! - Cross-file references using `@alias.variable` syntax
//! - Automatic dependency resolution
//! - Multi-file updates (Excel-style behavior)
//! - Fuzzy variable name matching
//!
//! # Example
//!
//! ```no_run
//! use mouvify_forge::parser;
//! use mouvify_forge::core::Calculator;
//! use std::path::Path;
//!
//! let path = Path::new("model.yaml");
//! let parsed = parser::parse_yaml_with_includes(path)?;
//! let mut calculator = Calculator::new(parsed.variables.clone());
//! let results = calculator.calculate_all()?;
//! # Ok::<(), mouvify_forge::error::ForgeError>(())
//! ```

pub mod cli;
pub mod core;
pub mod error;
pub mod parser;
pub mod types;
pub mod writer;

// Re-export commonly used types
pub use error::{ForgeError, ForgeResult};
pub use types::{Include, ParsedYaml, Variable};
