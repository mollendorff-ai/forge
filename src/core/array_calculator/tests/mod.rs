//! Tests for ArrayCalculator
//!
//! Organized by function category for maintainability.

// Demo tests (always included)
mod aggregation;
mod core;
mod dates;
mod financial;
mod logical;
mod lookup;
mod math;
mod statistical;
mod text;

// Enterprise tests (only in full build)
#[cfg(feature = "full")]
mod advanced;
#[cfg(feature = "full")]
mod array;
#[cfg(feature = "full")]
mod conditional;
#[cfg(feature = "full")]
mod forge;
