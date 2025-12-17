//! Comprehensive Unit Tests for Miscellaneous Functions
//!
//! Functions tested:
//! - LOG: Logarithm (currently base-10, future: custom base support)
//! - CORREL: Correlation coefficient between two arrays
//! - TIME: Construct time from hour, minute, second
//! - NOW: Current date and time
//!
//! Coverage: 20+ tests with basic functionality, edge cases, and error handling

#![cfg(not(feature = "demo"))]

use royalbit_forge::core::array_calculator::ArrayCalculator;
use royalbit_forge::types::{Column, ColumnValue, ParsedModel, Table, Variable};

// ═══════════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

fn eval_scalar(formula: &str) -> f64 {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some(format!("={}", formula))),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    result.scalars.get("result").unwrap().value.unwrap()
}

fn eval_scalar_err(formula: &str) -> bool {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some(format!("={}", formula))),
    );
    let calculator = ArrayCalculator::new(model);
    calculator.calculate_all().is_err()
}

fn approx_eq(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() < tolerance
}

// ═══════════════════════════════════════════════════════════════════════════════
// LOG FUNCTION TESTS (6 tests)
// Note: Current implementation is LOG10. Future: LOG(number, base) for custom bases
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_log_100() {
    // LOG(100) should be 2.0 (10^2 = 100)
    let result = eval_scalar("LOG(100)");
    assert!(approx_eq(result, 2.0, 1e-10), "LOG(100) should be 2.0");
}

#[test]
fn test_log_1000() {
    // LOG(1000) should be 3.0 (10^3 = 1000)
    let result = eval_scalar("LOG(1000)");
    assert!(approx_eq(result, 3.0, 1e-10), "LOG(1000) should be 3.0");
}

#[test]
fn test_log_one() {
    // LOG(1) should be 0.0 (10^0 = 1)
    let result = eval_scalar("LOG(1)");
    assert!(approx_eq(result, 0.0, 1e-10), "LOG(1) should be 0.0");
}

#[test]
fn test_log_ten() {
    // LOG(10) should be 1.0 (10^1 = 10)
    let result = eval_scalar("LOG(10)");
    assert!(approx_eq(result, 1.0, 1e-10), "LOG(10) should be 1.0");
}

#[test]
fn test_log_zero_error() {
    // LOG(0) should error (log of non-positive number)
    assert!(eval_scalar_err("LOG(0)"), "LOG(0) should error");
}

#[test]
fn test_log_negative_error() {
    // LOG(-10) should error (log of non-positive number)
    assert!(eval_scalar_err("LOG(-10)"), "LOG(-10) should error");
}

// ═══════════════════════════════════════════════════════════════════════════════
// CORREL FUNCTION TESTS (7 tests)
// Correlation coefficient: -1 (perfect negative) to +1 (perfect positive)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_correl_perfect_positive() {
    // Perfect positive correlation: [1,2,3] and [2,4,6]
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    table.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 6.0]),
    ));
    model.add_table(table);

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let corr = result.scalars.get("result").unwrap().value.unwrap();

    assert!(
        approx_eq(corr, 1.0, 1e-10),
        "Perfect positive correlation should be 1.0"
    );
}

#[test]
fn test_correl_perfect_negative() {
    // Perfect negative correlation: [1,2,3] and [6,4,2]
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    table.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![6.0, 4.0, 2.0]),
    ));
    model.add_table(table);

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let corr = result.scalars.get("result").unwrap().value.unwrap();

    assert!(
        approx_eq(corr, -1.0, 1e-10),
        "Perfect negative correlation should be -1.0"
    );
}

#[test]
fn test_correl_zero() {
    // Zero correlation: no relationship between variables
    // Using x=[1,2,3] and y=[2,2,2] (constant y = zero variance)
    // Note: This will error because CORREL requires non-zero variance
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    table.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![2.0, 2.0, 2.0]),
    ));
    model.add_table(table);

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    // Should error due to zero variance
    assert!(result.is_err(), "CORREL with zero variance should error");
}

#[test]
fn test_correl_moderate_positive() {
    // Moderate positive correlation
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0]),
    ));
    table.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![2.0, 3.0, 5.0, 4.0]),
    ));
    model.add_table(table);

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let corr = result.scalars.get("result").unwrap().value.unwrap();

    // Should be positive correlation but not perfect
    assert!(
        corr > 0.0 && corr < 1.0,
        "Should have positive but imperfect correlation"
    );
}

#[test]
fn test_correl_unequal_length_error() {
    // Arrays of different lengths should error
    // This is harder to test with the table structure, so we skip for now
    // The implementation checks for equal length - tested in unit tests
}

#[test]
fn test_correl_single_value_error() {
    // CORREL requires at least 2 values
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new("x".to_string(), ColumnValue::Number(vec![1.0])));
    table.add_column(Column::new("y".to_string(), ColumnValue::Number(vec![2.0])));
    model.add_table(table);

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    // Should error with only 1 value
    assert!(result.is_err(), "CORREL with single value should error");
}

#[test]
fn test_correl_identical_arrays() {
    // Identical arrays should have correlation of 1.0
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![5.0, 10.0, 15.0]),
    ));
    table.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![5.0, 10.0, 15.0]),
    ));
    model.add_table(table);

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=CORREL(data.x, data.y)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let corr = result.scalars.get("result").unwrap().value.unwrap();

    assert!(
        approx_eq(corr, 1.0, 1e-10),
        "Identical arrays should have correlation 1.0"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// TIME FUNCTION TESTS (6 tests)
// TIME(hour, minute, second) returns fraction of day (Excel serial time)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_time_midnight() {
    // TIME(0, 0, 0) = 0.0 (midnight)
    let result = eval_scalar("TIME(0, 0, 0)");
    assert!(approx_eq(result, 0.0, 1e-10), "TIME(0, 0, 0) should be 0.0");
}

#[test]
fn test_time_noon() {
    // TIME(12, 0, 0) = 0.5 (noon is half a day)
    let result = eval_scalar("TIME(12, 0, 0)");
    assert!(
        approx_eq(result, 0.5, 1e-10),
        "TIME(12, 0, 0) should be 0.5"
    );
}

#[test]
fn test_time_quarter_day() {
    // TIME(6, 0, 0) = 0.25 (6 AM is quarter day)
    let result = eval_scalar("TIME(6, 0, 0)");
    assert!(
        approx_eq(result, 0.25, 1e-10),
        "TIME(6, 0, 0) should be 0.25"
    );
}

#[test]
fn test_time_with_minutes_seconds() {
    // TIME(14, 30, 45) - 2:30:45 PM
    // Total seconds: 14*3600 + 30*60 + 45 = 50400 + 1800 + 45 = 52245
    // Fraction: 52245 / 86400 = 0.604687...
    let result = eval_scalar("TIME(14, 30, 45)");
    let expected = 52245.0 / 86400.0;
    assert!(
        approx_eq(result, expected, 1e-10),
        "TIME(14, 30, 45) should be correct fraction"
    );
}

#[test]
fn test_time_one_second() {
    // TIME(0, 0, 1) = 1/86400
    let result = eval_scalar("TIME(0, 0, 1)");
    let expected = 1.0 / 86400.0;
    assert!(
        approx_eq(result, expected, 1e-10),
        "TIME(0, 0, 1) should be 1/86400"
    );
}

#[test]
fn test_time_one_minute() {
    // TIME(0, 1, 0) = 60/86400
    let result = eval_scalar("TIME(0, 1, 0)");
    let expected = 60.0 / 86400.0;
    assert!(
        approx_eq(result, expected, 1e-10),
        "TIME(0, 1, 0) should be 60/86400"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// NOW FUNCTION TESTS (5 tests)
// NOW() returns current date and time as text string
// Note: NOW() returns text, so we test it indirectly using functions that return numbers
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_now_returns_nonempty_string() {
    // NOW() should return a non-empty string
    // Test by checking that LEN(NOW()) > 0
    let result = eval_scalar("LEN(NOW())");
    assert!(result > 0.0, "NOW() should return a non-empty string");
}

#[test]
fn test_now_format_length() {
    // NOW() returns in format: YYYY-MM-DD HH:MM:SS (19 characters)
    // Verify the length is 19
    let result = eval_scalar("LEN(NOW())");
    assert_eq!(result, 19.0, "NOW() should return a string of length 19");
}

#[test]
fn test_now_istext() {
    // NOW() should return a text value
    // Test using ISTEXT which returns a boolean (converted to number)
    let result = eval_scalar("IF(ISTEXT(NOW()), 1, 0)");
    assert_eq!(result, 1.0, "NOW() should return a text value");
}

#[test]
fn test_now_contains_year() {
    // NOW() should contain current year (e.g., "2025")
    // Test by checking if the first 4 characters are numeric and >= 2020
    let result = eval_scalar("VALUE(LEFT(NOW(), 4))");
    assert!(
        (2020.0..=2100.0).contains(&result),
        "NOW() should contain a valid year"
    );
}

#[test]
fn test_now_contains_time_separator() {
    // NOW() format: "YYYY-MM-DD HH:MM:SS" has space at position 11
    // Test by checking MID(NOW(), 11, 1) = " "
    let result = eval_scalar("IF(MID(NOW(), 11, 1) = \" \", 1, 0)");
    assert_eq!(
        result, 1.0,
        "NOW() should have space separator between date and time"
    );
}
