//! Tests for ArrayCalculator
//!
//! Organized by function category for maintainability.
//!
//! DEMO tests: core, logical, math (partial), aggregation (partial),
//!             text (partial), dates (partial), lookup (partial)
//! ENTERPRISE tests: financial, statistical, advanced, array, conditional, forge

// Demo tests (always included)
mod aggregation;
mod core;
mod dates;
mod logical;
mod lookup;
mod math;
mod text;

// Enterprise-only test modules
#[cfg(feature = "full")]
mod advanced;
#[cfg(feature = "full")]
mod array;
#[cfg(feature = "full")]
mod conditional;
#[cfg(feature = "full")]
mod financial;
#[cfg(feature = "full")]
mod forge;
#[cfg(feature = "full")]
mod statistical;
