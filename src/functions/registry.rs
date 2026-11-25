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

#[cfg(test)]
mod tests {

    mod registry {
        //! Tests for the function registry
        //! Extracted from registry.rs to keep file under 1500 lines

        #[cfg(test)]
        mod tests {
            use crate::functions::registry::*;

            #[test]
            fn test_function_count() {
                assert_eq!(count_all(), 203, "Registry should have 203 functions");
            }

            #[test]
            fn test_find_function() {
                let sum = find_function("SUM");
                assert!(sum.is_some());

                let npv = find_function("NPV");
                assert!(npv.is_some());
            }

            #[test]
            fn test_scalar_count() {
                // Scalar functions work without array context (v1.0.0 compatible)
                let scalar = count_scalar();
                let array_only = count_array_only();
                assert_eq!(
                    scalar + array_only,
                    203,
                    "Scalar + array-only should equal total"
                );
                // 24 array-only functions:
                // Array (5): UNIQUE, FILTER, SORT, SEQUENCE, RANDARRAY
                // Conditional (6): SUMIF, SUMIFS, COUNTIF, COUNTIFS, AVERAGEIF, AVERAGEIFS
                // Aggregation (5): MAXIFS, MINIFS, RANK.EQ, LARGE, SMALL
                // Statistical (3): PERCENTILE, QUARTILE, CORREL
                // Lookup (5): INDEX, MATCH, VLOOKUP, HLOOKUP, XLOOKUP, OFFSET, INDIRECT
                assert!(
                    array_only >= 20,
                    "Should have at least 20 array-only functions"
                );
            }

            #[test]
            fn test_array_only_functions() {
                // Verify key array-only functions are correctly marked
                let array_funcs = ["UNIQUE", "FILTER", "SORT", "SUMIF", "VLOOKUP"];
                for name in array_funcs {
                    let scalar = find_function(name).map(|f| f.scalar);
                    assert_eq!(scalar, Some(false), "{name} should be scalar=false");
                }
            }

            /// The lists by kind hold each function of that kind once, and
            /// together all of them
            #[test]
            fn the_lists_by_kind_split_the_registry() {
                let scalar: Vec<&str> = scalar_functions().map(|f| f.name).collect();
                let array: Vec<&str> = array_only_functions().map(|f| f.name).collect();
                assert_eq!(scalar.len(), count_scalar());
                assert_eq!(array.len(), count_array_only());
                assert!(scalar.contains(&"ABS") && !scalar.contains(&"SORT"));
                assert!(array.contains(&"SORT") && !array.contains(&"ABS"));
                let financial: Vec<&str> =
                    by_category(Category::Financial).map(|f| f.name).collect();
                assert!(financial.contains(&"NPV"), "{financial:?}");
                assert!(!financial.contains(&"SUM"), "{financial:?}");
                let per_category: usize = [
                    Category::Math,
                    Category::Aggregation,
                    Category::Logical,
                    Category::Text,
                    Category::Date,
                    Category::Lookup,
                    Category::Financial,
                    Category::Statistical,
                    Category::Trigonometric,
                    Category::Information,
                    Category::Conditional,
                    Category::Array,
                    Category::Advanced,
                    Category::ForgeNative,
                    Category::MonteCarlo,
                ]
                .into_iter()
                .map(|category| by_category(category).count())
                .sum();
                assert_eq!(per_category, count_all());
            }
        }
    }

    mod function_list {
        //! `forge functions` lists only what a formula can call (roadmap
        //! T1-FUNCTION-LIST). It listed Y, M, D, YD, YM and MD, which are units of
        //! `DATEDIF`; `=YD(a, b)` was an unknown function.

        use crate::cli::calculate_core;
        use crate::functions::registry::all_functions;

        #[test]
        fn every_listed_function_is_known_to_the_calculator() {
            let dir = tempfile::tempdir().unwrap();
            // Calling with no arguments is usually an error; only "unknown"
            // matters (and none may panic, which fails the test)
            let unknown: Vec<&str> = all_functions()
                .filter_map(|f| {
                    let path = dir
                        .path()
                        .join(format!("{}.yaml", f.name.replace('.', "_")));
                    std::fs::write(
                        &path,
                        format!(
                            "_forge_version: \"5.2.0\"\nx:\n  a: {{formula: \"={}()\"}}\n",
                            f.name
                        ),
                    )
                    .unwrap();
                    let unknown = calculate_core(&path, true, None)
                        .err()
                        .is_some_and(|e| e.to_string().contains("Unknown function"));
                    unknown.then_some(f.name)
                })
                .collect();
            assert_eq!(unknown, Vec::<&str>::new(), "listed but not callable");
        }

        #[test]
        fn the_datedif_units_are_not_listed() {
            let names: Vec<&str> = all_functions().map(|f| f.name).collect();
            for unit in ["Y", "M", "D", "YD", "YM", "MD"] {
                assert!(!names.contains(&unit), "{unit} is a DATEDIF unit");
            }
            assert!(names.contains(&"DATEDIF"));
        }

        /// A reader of `forge functions` is told the sign Excel gives, and the
        /// calculator gives it: a loan received of 100,000 is paid with
        /// negative payments, interest and principal.
        #[test]
        fn the_loan_payment_functions_say_their_sign_and_give_it() {
            let text = "_forge_version: \"5.1.0\"\nloan:\n  \
                pmt: {formula: \"=PMT(0.005, 60, 100000)\"}\n  \
                ipmt: {formula: \"=IPMT(0.005, 1, 60, 100000)\"}\n  \
                ppmt: {formula: \"=PPMT(0.005, 1, 60, 100000)\"}\n";
            let model = crate::parser::parse_model_text(
                "loan.yaml",
                text,
                &std::collections::HashMap::new(),
            )
            .unwrap();
            let done = crate::core::ArrayCalculator::new(model)
                .calculate_all()
                .unwrap();
            for (name, key, value) in [
                ("PMT", "loan.pmt", -1933.28),
                ("IPMT", "loan.ipmt", -500.0),
                ("PPMT", "loan.ppmt", -1433.28),
            ] {
                let got = done.scalars[key].value.unwrap();
                assert!((got - value).abs() < 0.005, "{name} gave {got}");
                let said = crate::functions::registry::find_function(name)
                    .unwrap()
                    .description;
                assert!(said.contains("negative for a loan received"), "{said}");
            }
        }
    }
}
