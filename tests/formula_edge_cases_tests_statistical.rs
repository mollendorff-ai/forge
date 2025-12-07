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
fn test_percentile_function() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "stats.percentile".to_string(),
        var_formula("stats.percentile", "=PERCENTILE(data.values, 0.5)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let p = result.scalars.get("stats.percentile").unwrap();
    assert_eq!(p.value, Some(30.0)); // Median
}

#[test]
fn test_quartile_function() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "stats.q2".to_string(),
        var_formula("stats.q2", "=QUARTILE(data.values, 2)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let q = result.scalars.get("stats.q2").unwrap();
    assert_eq!(q.value, Some(30.0)); // Q2 = median
}

#[test]
fn test_correl_function() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
    ));
    table.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![20.0, 40.0, 60.0, 80.0, 100.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "stats.correl".to_string(),
        var_formula("stats.correl", "=CORREL(data.x, data.y)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let c = result.scalars.get("stats.correl").unwrap();
    assert!(c.value.unwrap() > 0.99); // Perfect correlation
}

#[test]
fn test_median_array() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("numbers".to_string());
    table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "array.median".to_string(),
        var_formula("array.median", "=MEDIAN(numbers.value)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let m = result.scalars.get("array.median").unwrap();
    assert_eq!(m.value, Some(30.0));
}

#[test]
fn test_var_sample_function() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "stats.var".to_string(),
        var_formula("stats.var", "=VAR(data.value)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let v = result.scalars.get("stats.var").unwrap();
    assert!(v.value.is_some());
}

#[test]
fn test_stdev_sample_function() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "stats.stdev".to_string(),
        var_formula("stats.stdev", "=STDEV(data.value)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let s = result.scalars.get("stats.stdev").unwrap();
    assert!(s.value.is_some());
    // Stdev should be around 2.0
    let stdev = s.value.unwrap();
    assert!(stdev > 1.5 && stdev < 2.5);
}

#[test]
fn test_correl_perfect_correlation() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    table.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 6.0, 8.0, 10.0]), // Perfect correlation
    ));
    model.add_table(table);

    model.scalars.insert(
        "stats.correl".to_string(),
        var_formula("stats.correl", "=CORREL(data.x, data.y)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let c = result.scalars.get("stats.correl").unwrap();
    assert!(c.value.is_some());
    // Perfect positive correlation
    let correl = c.value.unwrap();
    assert!(correl > 0.99);
}
