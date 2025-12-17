//! Type Coercion Edge Case Tests - Ported from forge-e2e
//!
//! Tests for type coercion edge cases including:
//! - Boolean to number coercion (TRUE + 1 = 2)
//! - Date arithmetic
//! - IF result type coercion

#![allow(clippy::approx_constant)]
#![allow(clippy::float_cmp)]

use crate::core::array_calculator::ArrayCalculator;
use crate::types::{ParsedModel, Variable};

// ══════════════════════════════════════════════════════════════════════════════
// BOOLEAN ARITHMETIC
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_boolean_true_plus_one() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=TRUE + 1".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
}

#[test]
fn test_boolean_false_plus_one() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=FALSE + 1".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
}

#[test]
fn test_boolean_true_multiply() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=TRUE * 5".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
}

#[test]
fn test_boolean_false_multiply() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=FALSE * 5".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// IF FUNCTION COERCION
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_if_true_branch() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IF(TRUE, 1, 0) + 1".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
}

#[test]
fn test_if_false_branch() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IF(FALSE, 1, 0) + 1".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// DATE ARITHMETIC
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_date_subtraction_leap_year() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=DATE(2024,12,31) - DATE(2024,1,1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(365.0));
}

#[test]
fn test_date_add_days() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=DATE(2024,6,15) + 10".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(45468.0));
}
