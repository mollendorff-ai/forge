//! Function definitions and registry
//!
//! This module provides the single source of truth for all Forge functions.
//! See ADR-013 for design details.

mod definitions;
pub mod registry;
#[cfg(test)]
mod registry_tests;

pub use registry::*;
