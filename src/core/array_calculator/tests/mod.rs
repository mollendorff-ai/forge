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
mod errors;
mod logical;
mod lookup;
mod math;
mod text;
mod text_edge_cases;
mod trig;

// Enterprise-only test modules
#[cfg(not(feature = "demo"))]
mod advanced;
#[cfg(not(feature = "demo"))]
mod array;
#[cfg(not(feature = "demo"))]
mod conditional;
#[cfg(not(feature = "demo"))]
mod financial;
#[cfg(not(feature = "demo"))]
mod forge;
#[cfg(not(feature = "demo"))]
mod statistical;
