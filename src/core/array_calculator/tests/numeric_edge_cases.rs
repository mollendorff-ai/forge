//! Numeric Function Edge Case Tests - Ported from forge-e2e
//!
//! Tests for numeric function edge cases including:
//! - INT vs TRUNC behavior with negatives
//! - ROUND with half values and negative precision
//! - CEILING/FLOOR with negatives
//! - SQRT, ABS, SIGN edge cases

#![allow(clippy::approx_constant)]
#![allow(clippy::float_cmp)]

use crate::core::array_calculator::ArrayCalculator;
use crate::types::{ParsedModel, Variable};

// ══════════════════════════════════════════════════════════════════════════════
// INT FUNCTION - ENTERPRISE ONLY
// ══════════════════════════════════════════════════════════════════════════════
// INT rounds DOWN to the nearest integer (toward negative infinity)
// INT(5.9) = 5, INT(-5.9) = -6 (rounds DOWN to -6, not UP to -5)

#[cfg(not(feature = "demo"))]
#[test]
fn test_int_positive_fraction() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=INT(5.9)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_int_negative_large_fraction() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=INT(-5.9)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(-6.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_int_negative_small_fraction() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=INT(-5.1)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(-6.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// TRUNC FUNCTION - ENTERPRISE ONLY
// ══════════════════════════════════════════════════════════════════════════════
// TRUNC removes the decimal part (toward zero), different from INT for negatives
// TRUNC(5.9) = 5, TRUNC(-5.9) = -5 (removes fraction, toward zero)

#[cfg(not(feature = "demo"))]
#[test]
fn test_trunc_positive_fraction() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=TRUNC(5.9)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_trunc_negative_large_fraction() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=TRUNC(-5.9)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(-5.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_trunc_negative_small_fraction() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=TRUNC(-5.1)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(-5.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// ROUND FUNCTION
// ══════════════════════════════════════════════════════════════════════════════
// ROUND uses "round half away from zero" (banker's rounding in some contexts)
// ROUND(2.5) = 3, ROUND(-2.5) = -3 (away from zero)
// Negative precision rounds to the left of decimal: ROUND(1234.5, -2) = 1200

#[test]
fn test_round_half_even_low() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=ROUND(2.5, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
}

#[test]
fn test_round_half_odd_low() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=ROUND(3.5, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(4.0));
}

#[test]
fn test_round_half_negative() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=ROUND(-2.5, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(-3.0));
}

#[test]
fn test_round_negative_precision_decimal() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=ROUND(1234.5, -2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1200.0));
}

#[test]
fn test_round_negative_precision_exact() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=ROUND(1250, -2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1300.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// CEILING FUNCTION
// ══════════════════════════════════════════════════════════════════════════════
// CEILING rounds UP to the nearest multiple of significance
// CEILING(2.1, 1) = 3, CEILING(-2.1, 1) = -2 (toward positive infinity)

#[test]
fn test_ceiling_positive_fraction() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=CEILING(2.1, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
}

#[test]
fn test_ceiling_negative_fraction() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=CEILING(-2.1, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(-2.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// FLOOR FUNCTION
// ══════════════════════════════════════════════════════════════════════════════
// FLOOR rounds DOWN to the nearest multiple of significance
// FLOOR(2.9, 1) = 2, FLOOR(-2.9, 1) = -3 (toward negative infinity)

#[test]
fn test_floor_positive_fraction() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=FLOOR(2.9, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
}

#[test]
fn test_floor_negative_fraction() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=FLOOR(-2.9, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(-3.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// SQRT FUNCTION
// ══════════════════════════════════════════════════════════════════════════════
// SQRT edge case: zero should return zero

#[test]
fn test_sqrt_zero() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=SQRT(0)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// ABS FUNCTION
// ══════════════════════════════════════════════════════════════════════════════
// ABS edge case: negative zero should return positive zero

#[test]
fn test_abs_negative_zero() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=ABS(-0)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// SIGN FUNCTION
// ══════════════════════════════════════════════════════════════════════════════
// SIGN returns -1 for negative, 0 for zero, 1 for positive

#[test]
fn test_sign_zero() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=SIGN(0)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

#[test]
fn test_sign_positive() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=SIGN(100)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
}

#[test]
fn test_sign_negative() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=SIGN(-100)".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(-1.0));
}
