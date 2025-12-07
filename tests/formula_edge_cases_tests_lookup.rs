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
fn test_xlookup_exact_match() {
    // XLOOKUP is preferred over VLOOKUP - use modern lookup syntax
    let mut model = ParsedModel::new();

    let mut table = Table::new("products".to_string());
    table.add_column(Column::new(
        "name".to_string(),
        ColumnValue::Text(vec![
            "Widget".to_string(),
            "Gadget".to_string(),
            "Gizmo".to_string(),
        ]),
    ));
    table.add_column(Column::new(
        "price".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 150.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "lookup.result".to_string(),
        var_formula(
            "lookup.result",
            "=XLOOKUP(\"Widget\", products.name, products.price)",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let v = result.scalars.get("lookup.result").unwrap();
    assert_eq!(v.value, Some(100.0));
}

#[test]
fn test_index_function() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("products".to_string());
    table.add_column(Column::new(
        "price".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 150.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "lookup.index".to_string(),
        var_formula("lookup.index", "=INDEX(products.price, 2)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let i = result.scalars.get("lookup.index").unwrap();
    assert_eq!(i.value, Some(200.0));
}

#[test]
fn test_match_function() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("products".to_string());
    table.add_column(Column::new(
        "name".to_string(),
        ColumnValue::Text(vec![
            "Widget".to_string(),
            "Gadget".to_string(),
            "Gizmo".to_string(),
        ]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "lookup.match".to_string(),
        var_formula("lookup.match", "=MATCH(\"Widget\", products.name)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let m = result.scalars.get("lookup.match").unwrap();
    assert_eq!(m.value, Some(1.0)); // 1-indexed
}

#[test]
fn test_choose_function() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "lookup.choose".to_string(),
        var_formula("lookup.choose", "=CHOOSE(2, 10, 20, 30)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let c = result.scalars.get("lookup.choose").unwrap();
    assert_eq!(c.value, Some(20.0));
}

#[test]
fn test_xlookup_not_found() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("products".to_string());
    table.add_column(Column::new(
        "name".to_string(),
        ColumnValue::Text(vec!["Widget".to_string(), "Gadget".to_string()]),
    ));
    table.add_column(Column::new(
        "price".to_string(),
        ColumnValue::Number(vec![100.0, 200.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "lookup.notfound".to_string(),
        var_formula(
            "lookup.notfound",
            "=XLOOKUP(\"NonExistent\", products.name, products.price, -1)",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let v = result.scalars.get("lookup.notfound").unwrap();
    assert_eq!(v.value, Some(-1.0)); // Returns if_not_found value
}

#[test]
fn test_match_exact_mode() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("items".to_string());
    table.add_column(Column::new(
        "id".to_string(),
        ColumnValue::Number(vec![101.0, 102.0, 103.0, 104.0, 105.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "lookup.match".to_string(),
        var_formula("lookup.match", "=MATCH(103, items.id, 0)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let m = result.scalars.get("lookup.match").unwrap();
    assert_eq!(m.value, Some(3.0)); // 1-based index
}
