//! Function Registry - Single Source of Truth for all Forge functions
//!
//! This module defines all supported functions with metadata including:
//! - Function name
//! - Category
//! - Description
//! - Syntax
//! - Demo availability (demo: true = included in demo build)
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
            Category::Math => write!(f, "Math"),
            Category::Aggregation => write!(f, "Aggregation"),
            Category::Logical => write!(f, "Logical"),
            Category::Text => write!(f, "Text"),
            Category::Date => write!(f, "Date"),
            Category::Lookup => write!(f, "Lookup"),
            Category::Financial => write!(f, "Financial"),
            Category::Statistical => write!(f, "Statistical"),
            Category::Trigonometric => write!(f, "Trigonometric"),
            Category::Information => write!(f, "Information"),
            Category::Conditional => write!(f, "Conditional"),
            Category::Array => write!(f, "Array"),
            Category::Advanced => write!(f, "Advanced"),
            Category::ForgeNative => write!(f, "Forge Native"),
            Category::MonteCarlo => write!(f, "Monte Carlo"),
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
    /// Available in demo build (false = enterprise only)
    pub demo: bool,
    /// Scalar compatible (true = works with v1.0.0 schema without tables/arrays)
    /// scalar=true: single value in, single value out (e.g., ABS, SQRT, IF)
    /// scalar=false: requires table/array context (e.g., UNIQUE, FILTER, SUMIF)
    pub scalar: bool,
}

/// Get demo functions only
pub fn demo_functions() -> impl Iterator<Item = &'static FunctionDef> {
    FUNCTIONS.iter().filter(|f| f.demo)
}

/// Get enterprise functions (all)
pub fn enterprise_functions() -> impl Iterator<Item = &'static FunctionDef> {
    FUNCTIONS.iter()
}

/// Count demo functions
pub fn count_demo() -> usize {
    FUNCTIONS.iter().filter(|f| f.demo).count()
}

/// Count enterprise functions (total)
pub fn count_enterprise() -> usize {
    FUNCTIONS.len()
}

/// Get functions by category
pub fn by_category(category: Category) -> impl Iterator<Item = &'static FunctionDef> {
    FUNCTIONS.iter().filter(move |f| f.category == category)
}

/// Get demo functions by category
pub fn demo_by_category(category: Category) -> impl Iterator<Item = &'static FunctionDef> {
    FUNCTIONS
        .iter()
        .filter(move |f| f.demo && f.category == category)
}

/// Check if function is available in demo build
pub fn is_demo_function(name: &str) -> bool {
    FUNCTIONS.iter().any(|f| f.name == name && f.demo)
}

/// Find function by name
pub fn find_function(name: &str) -> Option<&'static FunctionDef> {
    FUNCTIONS.iter().find(|f| f.name == name)
}

/// Count scalar functions (v1.0.0 compatible)
pub fn count_scalar() -> usize {
    FUNCTIONS.iter().filter(|f| f.scalar).count()
}

/// Count array-only functions (v5.0.0 only)
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
