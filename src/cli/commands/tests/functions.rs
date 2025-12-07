//! Functions tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_function_category_struct() {
    let cat = FunctionCategory {
        name: "Financial",
        functions: vec![
            ("NPV", "Net Present Value"),
            ("IRR", "Internal Rate of Return"),
        ],
    };

    assert_eq!(cat.name, "Financial");
    assert_eq!(cat.functions.len(), 2);
    assert_eq!(cat.functions[0].0, "NPV");
}
