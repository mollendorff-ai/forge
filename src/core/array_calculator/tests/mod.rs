//! Tests for ArrayCalculator
//!
//! Organized by function category for maintainability.
//!
//! DEMO tests: core, logical, math (partial), aggregation (partial),
//!             text (partial), dates (partial), lookup (partial)
//! ENTERPRISE tests: financial, statistical, advanced, array, conditional, forge
//!
//! Edge case tests: Ported from forge-e2e for comprehensive coverage

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

// Edge case tests (ported from forge-e2e)
mod comparison_edge_cases;
mod date_edge_cases;
mod error_edge_cases;
mod logical_edge_cases;
mod math_edge_cases;
mod numeric_edge_cases;
mod string_edge_cases;
mod type_coercion_edge_cases;

// Enterprise-only test modules
#[cfg(not(feature = "demo"))]
mod advanced;
#[cfg(not(feature = "demo"))]
mod advanced_function_edge_cases;
#[cfg(not(feature = "demo"))]
mod array;
#[cfg(not(feature = "demo"))]
mod array_function_edge_cases;
#[cfg(not(feature = "demo"))]
mod conditional;
#[cfg(not(feature = "demo"))]
mod conditional_function_edge_cases;
#[cfg(not(feature = "demo"))]
mod financial;
#[cfg(not(feature = "demo"))]
mod financial_edge_cases;
#[cfg(not(feature = "demo"))]
mod forge;
#[cfg(not(feature = "demo"))]
mod lookup_edge_cases;
#[cfg(not(feature = "demo"))]
mod statistical;
#[cfg(not(feature = "demo"))]
mod statistical_edge_cases;
