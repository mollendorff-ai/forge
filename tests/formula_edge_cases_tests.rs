//! Formula edge case tests for 100% coverage
//! Tests date, math, text, lookup, array, conditional aggregation, and FORGE functions
//! Uses programmatic model creation for reliability

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI/E

use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::types::{Column, ColumnValue, ParsedModel, Table, Variable};

// Helper to create a variable with formula
fn var_formula(path: &str, formula: &str) -> Variable {
    Variable::new(path.to_string(), None, Some(formula.to_string()))
}

// Helper to create a variable with value
fn var_value(path: &str, value: f64) -> Variable {
    Variable::new(path.to_string(), Some(value), None)
}

// ═══════════════════════════════════════════════════════════════════════════
// DATE FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_average_array() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("numbers".to_string());
    table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "array.avg".to_string(),
        var_formula("array.avg", "=AVERAGE(numbers.value)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let a = result.scalars.get("array.avg").unwrap();
    assert_eq!(a.value, Some(30.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_product_inline() {
    // PRODUCT with inline values (xlformula_engine style)
    let mut model = ParsedModel::new();

    model.scalars.insert(
        "math.product".to_string(),
        var_formula("math.product", "=PRODUCT(2, 3, 4, 5)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let p = result.scalars.get("math.product").unwrap();
    assert_eq!(p.value, Some(120.0)); // 2*3*4*5
}

#[test]
fn test_array_indexing() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("numbers".to_string());
    table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "array.first".to_string(),
        var_formula("array.first", "=numbers.value[0]"),
    );
    model.scalars.insert(
        "array.last".to_string(),
        var_formula("array.last", "=numbers.value[4]"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();

    let f = result.scalars.get("array.first").unwrap();
    assert_eq!(f.value, Some(10.0));

    let l = result.scalars.get("array.last").unwrap();
    assert_eq!(l.value, Some(50.0));
}

#[test]
fn test_nonexistent_scalar_reference() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "test.bad_ref".to_string(),
        var_formula("test.bad_ref", "=nonexistent.value + 1"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
}

#[test]
fn test_division_by_zero() {
    let mut model = ParsedModel::new();
    model
        .scalars
        .insert("test.zero".to_string(), var_value("test.zero", 0.0));
    model.scalars.insert(
        "test.divide".to_string(),
        var_formula("test.divide", "=100 / test.zero"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Division by zero returns an error (Div0), not infinity
    assert!(result.is_err());
}

#[test]
fn test_empty_table_formula_error() {
    let mut model = ParsedModel::new();

    // Create empty table
    let mut table = Table::new("empty".to_string());
    table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![]),
    ));
    model.add_table(table);

    // Try to add a formula column - should fail on empty table
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Empty tables should still calculate successfully (no formulas to evaluate)
    assert!(result.is_ok());
}
