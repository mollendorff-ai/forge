//! Date Edge Case Tests - Ported from forge-e2e
//!
//! Tests for date edge cases including:
//! - Leap year handling
//! - Month overflow/underflow
//! - Day overflow/underflow
//! - Date arithmetic

#![allow(clippy::approx_constant)]
#![allow(clippy::float_cmp)]

use crate::core::array_calculator::ArrayCalculator;
use crate::types::{ParsedModel, Variable};

// ══════════════════════════════════════════════════════════════════════════════
// LEAP YEAR TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_leap_year_valid_year() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=YEAR(DATE(2024, 2, 29))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2024.0));
}

#[test]
fn test_leap_year_valid_month() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=MONTH(DATE(2024, 2, 29))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
}

#[test]
fn test_leap_year_valid_day() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=DAY(DATE(2024, 2, 29))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(29.0));
}

#[test]
fn test_non_leap_year_rollover_year() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=YEAR(DATE(2023, 2, 29))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2023.0));
}

#[test]
fn test_non_leap_year_rollover_month() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=MONTH(DATE(2023, 2, 29))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
}

#[test]
fn test_non_leap_year_rollover_day() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=DAY(DATE(2023, 2, 29))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// MONTH OVERFLOW/UNDERFLOW TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_month_zero_year() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=YEAR(DATE(2024, 0, 1))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2023.0));
}

#[test]
fn test_month_zero_month() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=MONTH(DATE(2024, 0, 1))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(12.0));
}

#[test]
fn test_month_thirteen_year() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=YEAR(DATE(2024, 13, 1))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2025.0));
}

#[test]
fn test_month_thirteen_month() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=MONTH(DATE(2024, 13, 1))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// DAY OVERFLOW/UNDERFLOW TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_day_zero_day() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=DAY(DATE(2024, 1, 0))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(31.0));
}

#[test]
fn test_day_zero_month() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=MONTH(DATE(2024, 1, 0))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(12.0));
}

#[test]
fn test_day_overflow_day() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=DAY(DATE(2024, 1, 32))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
}

#[test]
fn test_day_overflow_month() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=MONTH(DATE(2024, 1, 32))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// DATE ARITHMETIC TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_date_subtraction_leap_year() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=DATE(2024, 12, 31) - DATE(2024, 1, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(365.0));
}

#[test]
fn test_date_subtraction_non_leap_year() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=DATE(2023, 12, 31) - DATE(2023, 1, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(364.0));
}

#[test]
fn test_today_subtraction() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=TODAY() - TODAY()".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

#[test]
fn test_eomonth_subtraction() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=EOMONTH(DATE(2024, 1, 15), 0) - DATE(2024, 1, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(30.0));
}
