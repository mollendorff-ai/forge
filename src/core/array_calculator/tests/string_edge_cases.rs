//! String Operations Edge Case Tests - Ported from forge-e2e
//!
//! Tests for string operation edge cases including:
//! - Concatenation with spaces
//! - Empty string handling
//! - LEFT/RIGHT/MID edge cases
//! - TRIM behavior
//! - UPPER/LOWER
//! - REPT function

#![allow(clippy::approx_constant)]
#![allow(clippy::float_cmp)]

use crate::core::array_calculator::ArrayCalculator;
use crate::types::{ParsedModel, Variable};

// ══════════════════════════════════════════════════════════════════════════════
// CONCATENATION
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_concat_with_spaces() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(CONCAT(\"Hello\", \" \", \"World\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(11.0));
}

#[test]
fn test_concat_function() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(CONCAT(\"ab\", \"cd\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(4.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// EMPTY STRING HANDLING
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_empty_string() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=LEN(\"\")".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

#[test]
fn test_single_space() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some("=LEN(\" \")".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// LEFT/RIGHT EDGE CASES
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_left_empty_string() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(LEFT(\"\", 5))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

#[test]
fn test_right_empty_string() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(RIGHT(\"\", 5))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

#[test]
fn test_left_partial() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(LEFT(\"Hello\", 3))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
}

#[test]
fn test_right_partial() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(RIGHT(\"Hello\", 3))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// MID EDGE CASES
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_mid_from_start() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(MID(\"test\", 1, 2))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
}

#[test]
fn test_mid_from_middle() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(MID(\"test\", 2, 2))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// TRIM BEHAVIOR
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_trim_internal_spaces() {
    // Excel TRIM collapses multiple internal spaces to single space
    // "  a  b  " -> "a b" (length 3)
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(TRIM(\"  a  b  \"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
}

#[test]
fn test_trim_only_spaces() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(TRIM(\"   \"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// UPPER/LOWER
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_upper_length() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(UPPER(\"abc\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
}

#[test]
fn test_lower_length() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(LOWER(\"ABC\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// REPT FUNCTION
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_rept_single_char() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(REPT(\"x\", 5))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
}

#[test]
fn test_rept_multi_char() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(REPT(\"ab\", 3))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(6.0));
}
