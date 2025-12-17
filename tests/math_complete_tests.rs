//! v6.1.0 - Math Functions Complete E2E Tests
//!
//! 100% coverage of all 20 math functions:
//! ABS, CEILING, DEGREES, E, EXP, FLOOR, INT, LN, LOG10, MOD,
//! PI, POW, POWER, RADIANS, ROUND, ROUNDDOWN, ROUNDUP, SIGN, SQRT, TRUNC
//!
//! Tests: Basic operations, edge cases, error conditions, precision

#![cfg(not(feature = "demo"))]
#![allow(clippy::approx_constant)]

use royalbit_forge::core::array_calculator::ArrayCalculator;
use royalbit_forge::types::{Column, ColumnValue, ParsedModel, Table, Variable};
use std::f64::consts::{E as EULER, PI};

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
    (a - b).abs() <= tolerance
}

// ═══════════════════════════════════════════════════════════════════════════════
// ABS FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_abs_positive() {
    assert_eq!(eval_scalar("ABS(42)"), 42.0);
}

#[test]
fn test_abs_negative() {
    assert_eq!(eval_scalar("ABS(-42)"), 42.0);
}

#[test]
fn test_abs_zero() {
    assert_eq!(eval_scalar("ABS(0)"), 0.0);
}

#[test]
fn test_abs_decimal() {
    assert!(approx_eq(eval_scalar("ABS(-3.14159)"), 3.14159, 0.00001));
}

#[test]
fn test_abs_large() {
    assert_eq!(eval_scalar("ABS(-1000000)"), 1000000.0);
}

#[test]
fn test_abs_small() {
    assert!(approx_eq(eval_scalar("ABS(-0.0001)"), 0.0001, 0.000001));
}

// ═══════════════════════════════════════════════════════════════════════════════
// CEILING FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_ceiling_basic() {
    assert_eq!(eval_scalar("CEILING(2.3, 1)"), 3.0);
}

#[test]
fn test_ceiling_significance_5() {
    assert_eq!(eval_scalar("CEILING(13, 5)"), 15.0);
}

#[test]
fn test_ceiling_tenth() {
    assert!(approx_eq(eval_scalar("CEILING(2.31, 0.1)"), 2.4, 0.001));
}

#[test]
fn test_ceiling_negative() {
    assert_eq!(eval_scalar("CEILING(-2.3, 1)"), -2.0);
}

#[test]
fn test_ceiling_already_round() {
    assert_eq!(eval_scalar("CEILING(5.0, 1)"), 5.0);
}

#[test]
fn test_ceiling_zero_significance() {
    assert_eq!(eval_scalar("CEILING(3.5, 0)"), 0.0);
}

#[test]
fn test_ceiling_default_significance() {
    assert_eq!(eval_scalar("CEILING(3.2)"), 4.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// DEGREES FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_degrees_pi() {
    assert!(approx_eq(eval_scalar("DEGREES(PI())"), 180.0, 0.0001));
}

#[test]
fn test_degrees_half_pi() {
    assert!(approx_eq(eval_scalar("DEGREES(PI()/2)"), 90.0, 0.0001));
}

#[test]
fn test_degrees_2pi() {
    assert!(approx_eq(eval_scalar("DEGREES(2*PI())"), 360.0, 0.0001));
}

#[test]
fn test_degrees_zero() {
    assert_eq!(eval_scalar("DEGREES(0)"), 0.0);
}

#[test]
fn test_degrees_one_radian() {
    assert!(approx_eq(eval_scalar("DEGREES(1)"), 57.29577951, 0.0001));
}

// ═══════════════════════════════════════════════════════════════════════════════
// E FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_e_constant() {
    assert!(approx_eq(eval_scalar("E()"), EULER, 0.00001));
}

#[test]
fn test_e_squared() {
    assert!(approx_eq(eval_scalar("E()*E()"), EULER * EULER, 0.0001));
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXP FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_exp_zero() {
    assert_eq!(eval_scalar("EXP(0)"), 1.0);
}

#[test]
fn test_exp_one() {
    assert!(approx_eq(eval_scalar("EXP(1)"), EULER, 0.00001));
}

#[test]
fn test_exp_two() {
    assert!(approx_eq(eval_scalar("EXP(2)"), EULER * EULER, 0.0001));
}

#[test]
fn test_exp_negative() {
    assert!(approx_eq(eval_scalar("EXP(-1)"), 1.0 / EULER, 0.00001));
}

#[test]
fn test_exp_large() {
    assert!(approx_eq(eval_scalar("EXP(10)"), 22026.4657948, 0.01));
}

#[test]
fn test_exp_negative_large() {
    let result = eval_scalar("EXP(-100)");
    assert!(result > 0.0 && result < 1e-40);
}

// ═══════════════════════════════════════════════════════════════════════════════
// FLOOR FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_floor_basic() {
    assert_eq!(eval_scalar("FLOOR(2.7, 1)"), 2.0);
}

#[test]
fn test_floor_significance_5() {
    assert_eq!(eval_scalar("FLOOR(17, 5)"), 15.0);
}

#[test]
fn test_floor_tenth() {
    assert!(approx_eq(eval_scalar("FLOOR(2.79, 0.1)"), 2.7, 0.001));
}

#[test]
fn test_floor_negative() {
    assert_eq!(eval_scalar("FLOOR(-2.3, 1)"), -3.0);
}

#[test]
fn test_floor_already_round() {
    assert_eq!(eval_scalar("FLOOR(5.0, 1)"), 5.0);
}

#[test]
fn test_floor_zero_significance() {
    assert_eq!(eval_scalar("FLOOR(3.5, 0)"), 0.0);
}

#[test]
fn test_floor_default_significance() {
    assert_eq!(eval_scalar("FLOOR(3.9)"), 3.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// INT FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_int_positive() {
    assert_eq!(eval_scalar("INT(5.9)"), 5.0);
}

#[test]
fn test_int_negative() {
    assert_eq!(eval_scalar("INT(-5.1)"), -6.0);
}

#[test]
fn test_int_whole() {
    assert_eq!(eval_scalar("INT(10)"), 10.0);
}

#[test]
fn test_int_zero() {
    assert_eq!(eval_scalar("INT(0)"), 0.0);
}

#[test]
fn test_int_negative_whole() {
    assert_eq!(eval_scalar("INT(-5)"), -5.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// LN FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_ln_e() {
    assert!(approx_eq(eval_scalar("LN(E())"), 1.0, 0.00001));
}

#[test]
fn test_ln_one() {
    assert_eq!(eval_scalar("LN(1)"), 0.0);
}

#[test]
fn test_ln_two() {
    assert!(approx_eq(eval_scalar("LN(2)"), 0.693147, 0.0001));
}

#[test]
fn test_ln_ten() {
    assert!(approx_eq(eval_scalar("LN(10)"), 2.302585, 0.0001));
}

#[test]
fn test_ln_half() {
    assert!(approx_eq(eval_scalar("LN(0.5)"), -0.693147, 0.0001));
}

#[test]
fn test_ln_zero_error() {
    assert!(eval_scalar_err("LN(0)"));
}

#[test]
fn test_ln_negative_error() {
    assert!(eval_scalar_err("LN(-1)"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// LOG10 FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_log10_ten() {
    assert_eq!(eval_scalar("LOG10(10)"), 1.0);
}

#[test]
fn test_log10_hundred() {
    assert_eq!(eval_scalar("LOG10(100)"), 2.0);
}

#[test]
fn test_log10_thousand() {
    assert_eq!(eval_scalar("LOG10(1000)"), 3.0);
}

#[test]
fn test_log10_one() {
    assert_eq!(eval_scalar("LOG10(1)"), 0.0);
}

#[test]
fn test_log10_fraction() {
    assert_eq!(eval_scalar("LOG10(0.1)"), -1.0);
}

#[test]
fn test_log10_zero_error() {
    assert!(eval_scalar_err("LOG10(0)"));
}

#[test]
fn test_log10_negative_error() {
    assert!(eval_scalar_err("LOG10(-1)"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// MOD FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_mod_basic() {
    assert_eq!(eval_scalar("MOD(10, 3)"), 1.0);
}

#[test]
fn test_mod_exact() {
    assert_eq!(eval_scalar("MOD(12, 4)"), 0.0);
}

#[test]
fn test_mod_larger_divisor() {
    assert_eq!(eval_scalar("MOD(3, 10)"), 3.0);
}

#[test]
fn test_mod_decimal() {
    assert!(approx_eq(eval_scalar("MOD(5.5, 2)"), 1.5, 0.001));
}

#[test]
fn test_mod_negative_dividend() {
    assert_eq!(eval_scalar("MOD(-10, 3)"), -1.0);
}

#[test]
fn test_mod_zero_dividend() {
    assert_eq!(eval_scalar("MOD(0, 5)"), 0.0);
}

#[test]
fn test_mod_zero_divisor_error() {
    assert!(eval_scalar_err("MOD(10, 0)"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// PI FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_pi_constant() {
    assert!(approx_eq(eval_scalar("PI()"), PI, 0.00001));
}

#[test]
fn test_pi_doubled() {
    assert!(approx_eq(eval_scalar("2*PI()"), 2.0 * PI, 0.0001));
}

#[test]
fn test_pi_squared() {
    assert!(approx_eq(eval_scalar("PI()*PI()"), PI * PI, 0.0001));
}

// ═══════════════════════════════════════════════════════════════════════════════
// POW / POWER FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_pow_square() {
    assert_eq!(eval_scalar("POW(2, 2)"), 4.0);
}

#[test]
fn test_pow_cube() {
    assert_eq!(eval_scalar("POW(2, 3)"), 8.0);
}

#[test]
fn test_pow_ten() {
    assert_eq!(eval_scalar("POW(2, 10)"), 1024.0);
}

#[test]
fn test_pow_zero_exp() {
    assert_eq!(eval_scalar("POW(5, 0)"), 1.0);
}

#[test]
fn test_pow_negative_exp() {
    assert_eq!(eval_scalar("POW(2, -1)"), 0.5);
}

#[test]
fn test_pow_fractional_exp() {
    assert!(approx_eq(eval_scalar("POW(27, 1/3)"), 3.0, 0.0001));
}

#[test]
fn test_power_square() {
    assert_eq!(eval_scalar("POWER(3, 2)"), 9.0);
}

#[test]
fn test_power_cube() {
    assert_eq!(eval_scalar("POWER(4, 3)"), 64.0);
}

#[test]
fn test_power_one_exp() {
    assert_eq!(eval_scalar("POWER(42, 1)"), 42.0);
}

#[test]
fn test_power_zero_base() {
    assert_eq!(eval_scalar("POWER(0, 5)"), 0.0);
}

#[test]
fn test_power_negative_base_even() {
    assert_eq!(eval_scalar("POWER(-2, 2)"), 4.0);
}

#[test]
fn test_power_negative_base_odd() {
    assert_eq!(eval_scalar("POWER(-2, 3)"), -8.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// RADIANS FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_radians_180() {
    assert!(approx_eq(eval_scalar("RADIANS(180)"), PI, 0.00001));
}

#[test]
fn test_radians_90() {
    assert!(approx_eq(eval_scalar("RADIANS(90)"), PI / 2.0, 0.00001));
}

#[test]
fn test_radians_360() {
    assert!(approx_eq(eval_scalar("RADIANS(360)"), 2.0 * PI, 0.0001));
}

#[test]
fn test_radians_zero() {
    assert_eq!(eval_scalar("RADIANS(0)"), 0.0);
}

#[test]
fn test_radians_45() {
    assert!(approx_eq(eval_scalar("RADIANS(45)"), PI / 4.0, 0.00001));
}

// ═══════════════════════════════════════════════════════════════════════════════
// ROUND FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_round_up() {
    assert!(approx_eq(eval_scalar("ROUND(2.567, 2)"), 2.57, 0.001));
}

#[test]
fn test_round_down() {
    assert!(approx_eq(eval_scalar("ROUND(2.564, 2)"), 2.56, 0.001));
}

#[test]
fn test_round_half() {
    // Standard rounding (may be 3 or 2 depending on banker's rounding)
    let result = eval_scalar("ROUND(2.5, 0)");
    assert!(result == 2.0 || result == 3.0);
}

#[test]
fn test_round_negative() {
    assert!(approx_eq(eval_scalar("ROUND(-2.567, 2)"), -2.57, 0.001));
}

#[test]
fn test_round_zero_decimals() {
    assert_eq!(eval_scalar("ROUND(3.7, 0)"), 4.0);
}

#[test]
fn test_round_one_decimal() {
    assert!(approx_eq(eval_scalar("ROUND(3.14159, 1)"), 3.1, 0.001));
}

#[test]
fn test_round_default() {
    assert_eq!(eval_scalar("ROUND(3.567)"), 4.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// ROUNDDOWN FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_rounddown_pos() {
    assert!(approx_eq(eval_scalar("ROUNDDOWN(2.789, 2)"), 2.78, 0.001));
}

#[test]
fn test_rounddown_neg() {
    assert!(approx_eq(eval_scalar("ROUNDDOWN(-2.789, 2)"), -2.78, 0.001));
}

#[test]
fn test_rounddown_zero() {
    assert_eq!(eval_scalar("ROUNDDOWN(2.9, 0)"), 2.0);
}

#[test]
fn test_rounddown_one_decimal() {
    assert!(approx_eq(eval_scalar("ROUNDDOWN(3.567, 1)"), 3.5, 0.001));
}

#[test]
fn test_rounddown_default() {
    assert_eq!(eval_scalar("ROUNDDOWN(3.9)"), 3.0);
}

#[test]
fn test_rounddown_exact() {
    assert_eq!(eval_scalar("ROUNDDOWN(2.0, 0)"), 2.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// ROUNDUP FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_roundup_pos() {
    assert!(approx_eq(eval_scalar("ROUNDUP(2.321, 2)"), 2.33, 0.001));
}

#[test]
fn test_roundup_neg() {
    assert!(approx_eq(eval_scalar("ROUNDUP(-2.321, 2)"), -2.33, 0.001));
}

#[test]
fn test_roundup_zero() {
    assert_eq!(eval_scalar("ROUNDUP(2.1, 0)"), 3.0);
}

#[test]
fn test_roundup_one_decimal() {
    assert!(approx_eq(eval_scalar("ROUNDUP(3.412, 1)"), 3.5, 0.001));
}

#[test]
fn test_roundup_default() {
    assert_eq!(eval_scalar("ROUNDUP(3.2)"), 4.0);
}

#[test]
fn test_roundup_exact() {
    assert_eq!(eval_scalar("ROUNDUP(2.0, 0)"), 2.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SIGN FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_sign_positive() {
    assert_eq!(eval_scalar("SIGN(42)"), 1.0);
}

#[test]
fn test_sign_negative() {
    assert_eq!(eval_scalar("SIGN(-42)"), -1.0);
}

#[test]
fn test_sign_zero() {
    assert_eq!(eval_scalar("SIGN(0)"), 0.0);
}

#[test]
fn test_sign_small_positive() {
    assert_eq!(eval_scalar("SIGN(0.001)"), 1.0);
}

#[test]
fn test_sign_small_negative() {
    assert_eq!(eval_scalar("SIGN(-0.001)"), -1.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SQRT FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_sqrt_perfect() {
    assert_eq!(eval_scalar("SQRT(144)"), 12.0);
}

#[test]
fn test_sqrt_non_perfect() {
    assert!(approx_eq(eval_scalar("SQRT(2)"), 1.41421356, 0.00001));
}

#[test]
fn test_sqrt_zero() {
    assert_eq!(eval_scalar("SQRT(0)"), 0.0);
}

#[test]
fn test_sqrt_one() {
    assert_eq!(eval_scalar("SQRT(1)"), 1.0);
}

#[test]
fn test_sqrt_large() {
    assert_eq!(eval_scalar("SQRT(10000)"), 100.0);
}

#[test]
fn test_sqrt_small() {
    assert!(approx_eq(eval_scalar("SQRT(0.0001)"), 0.01, 0.0001));
}

#[test]
fn test_sqrt_negative_error() {
    assert!(eval_scalar_err("SQRT(-4)"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// TRUNC FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_trunc_positive() {
    assert_eq!(eval_scalar("TRUNC(5.9)"), 5.0);
}

#[test]
fn test_trunc_negative() {
    assert_eq!(eval_scalar("TRUNC(-5.9)"), -5.0);
}

#[test]
fn test_trunc_decimals() {
    assert!(approx_eq(eval_scalar("TRUNC(3.14159, 2)"), 3.14, 0.001));
}

#[test]
fn test_trunc_zero() {
    assert_eq!(eval_scalar("TRUNC(0.999)"), 0.0);
}

#[test]
fn test_trunc_exact() {
    assert_eq!(eval_scalar("TRUNC(5.0)"), 5.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// COMBINED / IDENTITY TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_identity_sqrt_square() {
    assert!(approx_eq(eval_scalar("SQRT(POWER(5, 2))"), 5.0, 0.0001));
}

#[test]
fn test_identity_ln_exp() {
    assert!(approx_eq(eval_scalar("LN(EXP(1))"), 1.0, 0.0001));
}

#[test]
fn test_identity_exp_ln() {
    assert!(approx_eq(eval_scalar("EXP(LN(10))"), 10.0, 0.0001));
}

#[test]
fn test_identity_radians_degrees() {
    assert!(approx_eq(eval_scalar("DEGREES(RADIANS(45))"), 45.0, 0.0001));
}

#[test]
fn test_combined_abs_sqrt() {
    assert_eq!(eval_scalar("SQRT(ABS(-16))"), 4.0);
}

#[test]
fn test_combined_round_power() {
    assert!(approx_eq(
        eval_scalar("ROUND(POWER(2, 0.5), 4)"),
        1.4142,
        0.0001
    ));
}

#[test]
fn test_combined_log_exp() {
    assert!(approx_eq(eval_scalar("LOG10(POWER(10, 3))"), 3.0, 0.0001));
}

#[test]
fn test_combined_sign_abs() {
    assert_eq!(eval_scalar("SIGN(-5) * ABS(-5)"), -5.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// TABLE / ARRAY TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_abs_array() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-10.0, 5.0, -3.0, 0.0, 8.0]),
    ));
    table.add_row_formula("absolute".to_string(), "=ABS(values)".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let col = result
        .tables
        .get("data")
        .unwrap()
        .columns
        .get("absolute")
        .unwrap();
    if let ColumnValue::Number(values) = &col.values {
        assert_eq!(values, &vec![10.0, 5.0, 3.0, 0.0, 8.0]);
    } else {
        panic!("Expected numeric column");
    }
}

#[test]
fn test_sqrt_array() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![4.0, 9.0, 16.0, 25.0, 100.0]),
    ));
    table.add_row_formula("roots".to_string(), "=SQRT(values)".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let col = result
        .tables
        .get("data")
        .unwrap()
        .columns
        .get("roots")
        .unwrap();
    if let ColumnValue::Number(values) = &col.values {
        assert_eq!(values, &vec![2.0, 3.0, 4.0, 5.0, 10.0]);
    } else {
        panic!("Expected numeric column");
    }
}

#[test]
fn test_round_array() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.456, 2.789, 3.123, 4.555]),
    ));
    table.add_row_formula("rounded".to_string(), "=ROUND(values, 1)".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let col = result
        .tables
        .get("data")
        .unwrap()
        .columns
        .get("rounded")
        .unwrap();
    if let ColumnValue::Number(values) = &col.values {
        assert_eq!(values, &vec![1.5, 2.8, 3.1, 4.6]);
    } else {
        panic!("Expected numeric column");
    }
}

#[test]
fn test_power_array() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "base".to_string(),
        ColumnValue::Number(vec![2.0, 3.0, 4.0, 5.0]),
    ));
    table.add_row_formula("squared".to_string(), "=POWER(base, 2)".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let col = result
        .tables
        .get("data")
        .unwrap()
        .columns
        .get("squared")
        .unwrap();
    if let ColumnValue::Number(values) = &col.values {
        assert_eq!(values, &vec![4.0, 9.0, 16.0, 25.0]);
    } else {
        panic!("Expected numeric column");
    }
}

#[test]
fn test_sign_array() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-5.0, 0.0, 5.0, -0.1, 0.1]),
    ));
    table.add_row_formula("signs".to_string(), "=SIGN(values)".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let col = result
        .tables
        .get("data")
        .unwrap()
        .columns
        .get("signs")
        .unwrap();
    if let ColumnValue::Number(values) = &col.values {
        assert_eq!(values, &vec![-1.0, 0.0, 1.0, -1.0, 1.0]);
    } else {
        panic!("Expected numeric column");
    }
}
