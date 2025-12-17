//! Formula edge case tests for 100% coverage
//! Tests date, math, text, lookup, array, conditional aggregation, and FORGE functions
//! Uses programmatic model creation for reliability

// Financial functions are enterprise-only
#![cfg(not(feature = "demo"))]
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
fn test_npv_function() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("cashflows".to_string());
    table.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![-1000.0, 300.0, 400.0, 500.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "finance.npv".to_string(),
        var_formula("finance.npv", "=NPV(0.10, cashflows.amount)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let npv = result.scalars.get("finance.npv").unwrap();
    assert!(npv.value.is_some());
}

#[test]
fn test_irr_function() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("cashflows".to_string());
    table.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![-1000.0, 400.0, 400.0, 400.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "finance.irr".to_string(),
        var_formula("finance.irr", "=IRR(cashflows.amount)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let irr = result.scalars.get("finance.irr").unwrap();
    assert!(irr.value.is_some());
}

#[test]
fn test_pmt_function() {
    let mut model = ParsedModel::new();

    model.scalars.insert(
        "loan.payment".to_string(),
        var_formula("loan.payment", "=PMT(0.05/12, 360, -100000)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let pmt = result.scalars.get("loan.payment").unwrap();
    assert!(pmt.value.is_some());
    // Monthly payment should be around $537 for 100k at 5% for 30 years
    let payment = pmt.value.unwrap();
    assert!(payment > 500.0 && payment < 600.0);
}

#[test]
fn test_fv_function() {
    let mut model = ParsedModel::new();

    model.scalars.insert(
        "savings.fv".to_string(),
        var_formula("savings.fv", "=FV(0.05, 10, -1000, 0)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let fv = result.scalars.get("savings.fv").unwrap();
    assert!(fv.value.is_some());
}

#[test]
fn test_pv_function() {
    let mut model = ParsedModel::new();

    model.scalars.insert(
        "annuity.pv".to_string(),
        var_formula("annuity.pv", "=PV(0.05, 10, -1000, 0)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let pv = result.scalars.get("annuity.pv").unwrap();
    assert!(pv.value.is_some());
}
