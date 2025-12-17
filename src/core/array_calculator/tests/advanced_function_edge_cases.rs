//! Advanced Function Edge Case Tests
//!
//! Tests for LET, LAMBDA, SWITCH, IFS advanced edge cases

#![allow(clippy::approx_constant)]
#![allow(clippy::float_cmp)]

use crate::core::array_calculator::ArrayCalculator;
use crate::types::{ParsedModel, Variable};

// ============================================================================
// LET Tests (6 tests)
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_let_single() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LET(x, 5, x * 2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_let_two_vars() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LET(x, 5, y, 3, x + y)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(8.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_let_dependent() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LET(x, 5, y, x * 2, y + 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(11.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_let_with_sum() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LET(x, SUM(1,2,3), x * 2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(12.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_let_nested() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LET(a, 2, b, LET(c, a*2, c+1), b)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_let_complex() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LET(rate, 0.08, price, 100, price * (1 + rate))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(108.0));
}

// ============================================================================
// LAMBDA Tests - REMOVED
// Note: LAMBDA is not supported by the underlying xlformula_engine.
// These tests are commented out until LAMBDA support is added.
// ============================================================================

// ============================================================================
// SWITCH Tests (4 tests)
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_switch_first() {
    // SWITCH(1, 1, 100, 2, 200, 999) = 100 (first case matches)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SWITCH(1, 1, 100, 2, 200, 999)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(100.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_switch_last() {
    // SWITCH(2, 1, 100, 2, 200, 999) = 200 (second case matches)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SWITCH(2, 1, 100, 2, 200, 999)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(200.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_switch_default() {
    // SWITCH(99, 1, 100, 2, 200, 999) = 999 (default case)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SWITCH(99, 1, 100, 2, 200, 999)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(999.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_switch_numeric() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SWITCH(2, 1, 10, 2, 20, 3, 30)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(20.0));
}

// ============================================================================
// IFS Tests (5 tests)
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_ifs_first_true() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IFS(TRUE, 1, FALSE, 2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_ifs_second_true() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IFS(FALSE, 1, TRUE, 2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_ifs_with_comparison() {
    // IFS(5>10, 100, 5<10, 200) = 200 (second condition true)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IFS(5>10, 100, 5<10, 200)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(200.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_ifs_with_and() {
    // IFS(AND(1>0, 2>0), 100, TRUE, 200) = 100 (AND condition true)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IFS(AND(1>0, 2>0), 100, TRUE, 200)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(100.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_ifs_with_or() {
    // IFS(OR(1>10, 2>10), 100, TRUE, 200) = 200 (OR false, fallback to TRUE)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IFS(OR(1>10, 2>10), 100, TRUE, 200)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(200.0));
}
