//! E2E Roundtrip test modules for Gnumeric validation
//!
//! This module contains all E2E tests organized by function category.

// Harness and infrastructure
pub mod harness;

// Test modules by category
pub mod aggregation;
pub mod conditional;
pub mod financial;
pub mod lookup;
pub mod math;
pub mod statistical;
pub mod text_date;
