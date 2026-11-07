//! Function Registry - Single Source of Truth for all Forge functions
//!
//! This module defines all supported functions with metadata including:
//! - Function name
//! - Category
//! - Description
//! - Syntax
//! - Scalar compatibility (scalar: true = works with v1.0.0 schema, no tables needed)
//!
//! See ADR-013 for the design rationale.
//! See ADR-014 for scalar/array classification.

pub use super::definitions::FUNCTIONS;

/// Function category for grouping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Math,
    Aggregation,
    Logical,
    Text,
    Date,
    Lookup,
    Financial,
    Statistical,
    Trigonometric,
    Information,
    Conditional,
    Array,
    Advanced,
    ForgeNative,
    MonteCarlo,
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Math => write!(f, "Math"),
            Self::Aggregation => write!(f, "Aggregation"),
            Self::Logical => write!(f, "Logical"),
            Self::Text => write!(f, "Text"),
            Self::Date => write!(f, "Date"),
            Self::Lookup => write!(f, "Lookup"),
            Self::Financial => write!(f, "Financial"),
            Self::Statistical => write!(f, "Statistical"),
            Self::Trigonometric => write!(f, "Trigonometric"),
            Self::Information => write!(f, "Information"),
            Self::Conditional => write!(f, "Conditional"),
            Self::Array => write!(f, "Array"),
            Self::Advanced => write!(f, "Advanced"),
            Self::ForgeNative => write!(f, "Forge Native"),
            Self::MonteCarlo => write!(f, "Monte Carlo"),
        }
    }
}

/// Function definition with all metadata
#[derive(Debug, Clone)]
pub struct FunctionDef {
    /// Function name (e.g., "SUM")
    pub name: &'static str,
    /// Category for grouping
    pub category: Category,
    /// Short description
    pub description: &'static str,
    /// Usage syntax (e.g., "=SUM(value1, value2, ...)")
    pub syntax: &'static str,
    /// Scalar compatible (true = works with v1.0.0 schema without tables/arrays)
    /// scalar=true: single value in, single value out (e.g., ABS, SQRT, IF)
    /// scalar=false: requires table/array context (e.g., UNIQUE, FILTER, SUMIF)
    pub scalar: bool,
}

/// Get all functions
pub fn all_functions() -> impl Iterator<Item = &'static FunctionDef> {
    FUNCTIONS.iter()
}

/// Count all functions
#[must_use]
pub fn count_all() -> usize {
    FUNCTIONS.len()
}

/// Get functions by category
pub fn by_category(category: Category) -> impl Iterator<Item = &'static FunctionDef> {
    FUNCTIONS.iter().filter(move |f| f.category == category)
}

/// Find function by name
#[must_use]
pub fn find_function(name: &str) -> Option<&'static FunctionDef> {
    FUNCTIONS.iter().find(|f| f.name == name)
}

/// Count scalar functions (v1.0.0 compatible)
#[must_use]
pub fn count_scalar() -> usize {
    FUNCTIONS.iter().filter(|f| f.scalar).count()
}

/// Count array-only functions (v5.0.0 only)
#[must_use]
pub fn count_array_only() -> usize {
    FUNCTIONS.iter().filter(|f| !f.scalar).count()
}

/// Get scalar functions only
pub fn scalar_functions() -> impl Iterator<Item = &'static FunctionDef> {
    FUNCTIONS.iter().filter(|f| f.scalar)
}

/// Get array-only functions
pub fn array_only_functions() -> impl Iterator<Item = &'static FunctionDef> {
    FUNCTIONS.iter().filter(|f| !f.scalar)
}

// Tests moved to registry_tests.rs to keep this file under 1500 lines
