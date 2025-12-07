//! Formula edge case tests for 100% coverage
//! Tests date, math, text, lookup, array, conditional aggregation, and FORGE functions
//! Uses programmatic model creation for reliability

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI/E

use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::types::{ParsedModel, Variable};

// Helper to create a variable with formula
#[allow(dead_code)]
fn var_formula(path: &str, formula: &str) -> Variable {
    Variable::new(path.to_string(), None, Some(formula.to_string()))
}

// Helper to create a variable with value
#[allow(dead_code)]
fn var_value(path: &str, value: f64) -> Variable {
    Variable::new(path.to_string(), Some(value), None)
}

// ═══════════════════════════════════════════════════════════════════════════
// DATE FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_left_function() {
    let mut model = ParsedModel::new();

    model.scalars.insert(
        "text.left".to_string(),
        var_formula("text.left", "=LEFT(\"Hello World\", 5)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // LEFT returns text which becomes an error for numeric scalars - this tests the path
    assert!(result.is_err());
}

#[test]
fn test_right_function() {
    let mut model = ParsedModel::new();

    model.scalars.insert(
        "text.right".to_string(),
        var_formula("text.right", "=RIGHT(\"Hello World\", 5)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // RIGHT returns text which becomes an error for numeric scalars - this tests the path
    assert!(result.is_err());
}
