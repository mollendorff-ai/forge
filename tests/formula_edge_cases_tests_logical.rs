//! Formula edge case tests for 100% coverage
//! Tests date, math, text, lookup, array, conditional aggregation, and FORGE functions
//! Uses programmatic model creation for reliability

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI/E

use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::types::{Column, ColumnValue, ParsedModel, Table, Variable};

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
fn test_cross_table_reference_table_not_found() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("items".to_string());
    table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    // Add formula as row_formula
    table.row_formulas.insert(
        "computed".to_string(),
        "=nonexistent.price + 10".to_string(),
    );
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
}

#[test]
fn test_cross_table_column_not_found() {
    let mut model = ParsedModel::new();

    let mut table1 = Table::new("source".to_string());
    table1.add_column(Column::new(
        "existing".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(table1);

    let mut table2 = Table::new("target".to_string());
    table2.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    // Add formula as row_formula
    table2.row_formulas.insert(
        "computed".to_string(),
        "=source.nonexistent + 10".to_string(),
    );
    model.add_table(table2);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
}

#[test]
fn test_local_column_not_found() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("items".to_string());
    table.add_column(Column::new(
        "price".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    // Add formula as row_formula
    table
        .row_formulas
        .insert("total".to_string(), "=nonexistent_column * 2".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
}
