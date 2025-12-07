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
fn test_round_function() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "math.round".to_string(),
        var_formula("math.round", "=ROUND(3.14159, 2)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let r = result.scalars.get("math.round").unwrap();
    assert_eq!(r.value, Some(3.14));
}

#[test]
fn test_roundup_function() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "math.roundup".to_string(),
        var_formula("math.roundup", "=ROUNDUP(3.14159, 2)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let r = result.scalars.get("math.roundup").unwrap();
    assert_eq!(r.value, Some(3.15));
}

#[test]
fn test_rounddown_function() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "math.rounddown".to_string(),
        var_formula("math.rounddown", "=ROUNDDOWN(3.14159, 2)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let r = result.scalars.get("math.rounddown").unwrap();
    assert_eq!(r.value, Some(3.14));
}

#[test]
fn test_ceiling_function() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "math.ceiling".to_string(),
        var_formula("math.ceiling", "=CEILING(7.3, 2)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let c = result.scalars.get("math.ceiling").unwrap();
    assert_eq!(c.value, Some(8.0));
}

#[test]
fn test_floor_function() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "math.floor".to_string(),
        var_formula("math.floor", "=FLOOR(7.9, 2)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let f = result.scalars.get("math.floor").unwrap();
    assert_eq!(f.value, Some(6.0));
}

#[test]
fn test_mod_function() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "math.mod".to_string(),
        var_formula("math.mod", "=MOD(17, 5)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let m = result.scalars.get("math.mod").unwrap();
    assert_eq!(m.value, Some(2.0));
}

#[test]
fn test_mod_by_zero_error() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "math.mod_zero".to_string(),
        var_formula("math.mod_zero", "=MOD(10, 0)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
}

#[test]
fn test_sqrt_function() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "math.sqrt".to_string(),
        var_formula("math.sqrt", "=SQRT(144)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let s = result.scalars.get("math.sqrt").unwrap();
    assert_eq!(s.value, Some(12.0));
}

#[test]
fn test_sqrt_negative_error() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "math.sqrt_neg".to_string(),
        var_formula("math.sqrt_neg", "=SQRT(-1)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
}

#[test]
fn test_power_function() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "math.power".to_string(),
        var_formula("math.power", "=POWER(2, 10)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let p = result.scalars.get("math.power").unwrap();
    assert_eq!(p.value, Some(1024.0));
}

#[test]
fn test_abs_function() {
    let mut model = ParsedModel::new();
    model
        .scalars
        .insert("math.abs".to_string(), var_formula("math.abs", "=ABS(-42)"));

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let a = result.scalars.get("math.abs").unwrap();
    assert_eq!(a.value, Some(42.0));
}

#[test]
fn test_abs_with_scalar_ref() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "input.negative".to_string(),
        var_value("input.negative", -42.0),
    );

    model.scalars.insert(
        "test.abs".to_string(),
        var_formula("test.abs", "=ABS(input.negative)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let v = result.scalars.get("test.abs").unwrap();
    assert_eq!(v.value, Some(42.0));
}
