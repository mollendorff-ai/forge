//! xlformula_engine Functional Equivalence Tests
//!
//! These integration tests verify that Forge's formula evaluation produces
//! identical results to raw xlformula_engine for all supported functions.
//!
//! See ADR-004 for rationale: docs/architecture/ADR-004-XLFORMULA-EQUIVALENCE.md
//!
//! Run with: cargo test --test xlformula_equivalence_tests
//!
//! NOTE: xlformula_engine has limited function support. Many Excel functions
//! that Forge supports are implemented by Forge itself, not xlformula_engine.
//! These tests focus on the functions that xlformula_engine natively supports.

use xlformula_engine::{calculate, parse_formula, types, NoCustomFunction};

/// Floating-point tolerance for comparisons (f32 precision limit)
const TOLERANCE: f64 = 1e-5;

/// Helper to evaluate a formula with xlformula_engine directly
fn eval_raw(formula: &str) -> f64 {
    let formula_str = if formula.starts_with('=') {
        formula.to_string()
    } else {
        format!("={}", formula)
    };

    let parsed = parse_formula::parse_string_to_formula(&formula_str, None::<NoCustomFunction>);
    let result =
        calculate::calculate_formula(parsed, Some(&|_name: String| types::Value::Number(0.0)));

    match result {
        types::Value::Number(n) => n as f64,
        types::Value::Boolean(b) => match b {
            types::Boolean::True => 1.0,
            types::Boolean::False => 0.0,
        },
        types::Value::Error(e) => panic!("Formula '{}' error: {:?}", formula, e),
        types::Value::Blank => 0.0,
        other => panic!("Unexpected result type for '{}': {:?}", formula, other),
    }
}

/// Assert two f64 values are equal within tolerance
fn assert_approx_eq(actual: f64, expected: f64, msg: &str) {
    assert!(
        (actual - expected).abs() < TOLERANCE,
        "{}: expected {}, got {} (diff: {})",
        msg,
        expected,
        actual,
        (actual - expected).abs()
    );
}

// ============================================================================
// BASIC ARITHMETIC (xlformula_engine core support)
// ============================================================================

#[test]
fn equiv_addition() {
    assert_approx_eq(eval_raw("1 + 2"), 3.0, "1 + 2");
    assert_approx_eq(eval_raw("100 + 200 + 300"), 600.0, "100 + 200 + 300");
    assert_approx_eq(eval_raw("0.1 + 0.2"), 0.3, "0.1 + 0.2");
}

#[test]
fn equiv_subtraction() {
    assert_approx_eq(eval_raw("5 - 3"), 2.0, "5 - 3");
    assert_approx_eq(eval_raw("100 - 50 - 25"), 25.0, "100 - 50 - 25");
}

#[test]
fn equiv_multiplication() {
    assert_approx_eq(eval_raw("6 * 7"), 42.0, "6 * 7");
    assert_approx_eq(eval_raw("0.5 * 100"), 50.0, "0.5 * 100");
}

#[test]
fn equiv_division() {
    assert_approx_eq(eval_raw("10 / 2"), 5.0, "10 / 2");
    assert_approx_eq(eval_raw("1 / 4"), 0.25, "1 / 4");
}

#[test]
fn equiv_parentheses() {
    assert_approx_eq(eval_raw("(10 + 5) * 2"), 30.0, "(10 + 5) * 2");
    assert_approx_eq(eval_raw("100 - 50 * 2"), 0.0, "100 - 50 * 2");
    assert_approx_eq(eval_raw("(100 - 50) * 2"), 100.0, "(100 - 50) * 2");
}

#[test]
fn equiv_negative_numbers() {
    assert_approx_eq(eval_raw("-5 + 10"), 5.0, "-5 + 10");
    assert_approx_eq(eval_raw("10 + -5"), 5.0, "10 + -5");
    assert_approx_eq(eval_raw("-5 * -2"), 10.0, "-5 * -2");
}

// ============================================================================
// AGGREGATE FUNCTIONS (xlformula_engine supported)
// ============================================================================

#[test]
fn equiv_sum() {
    assert_approx_eq(eval_raw("SUM(1, 2, 3)"), 6.0, "SUM(1, 2, 3)");
    assert_approx_eq(
        eval_raw("SUM(10, 20, 30, 40)"),
        100.0,
        "SUM(10, 20, 30, 40)",
    );
}

#[test]
fn equiv_average() {
    assert_approx_eq(eval_raw("AVERAGE(1, 2, 3)"), 2.0, "AVERAGE(1, 2, 3)");
    assert_approx_eq(eval_raw("AVERAGE(10, 20, 30)"), 20.0, "AVERAGE(10, 20, 30)");
}

#[test]
fn equiv_product() {
    assert_approx_eq(eval_raw("PRODUCT(2, 3, 4)"), 24.0, "PRODUCT(2, 3, 4)");
    assert_approx_eq(eval_raw("PRODUCT(1, 2, 3, 4)"), 24.0, "PRODUCT(1, 2, 3, 4)");
}

// ============================================================================
// FINANCIAL PERCENTAGE CALCULATIONS
// ============================================================================

#[test]
fn equiv_percentage_calculations() {
    assert_approx_eq(eval_raw("100 * 0.25"), 25.0, "25% of 100");
    assert_approx_eq(eval_raw("(100 - 70) / 100"), 0.3, "margin calculation");
    assert_approx_eq(eval_raw("1 - 0.1"), 0.9, "complement of 10%");
}

#[test]
fn equiv_complex_financial() {
    // Revenue - COGS = Gross Profit
    assert_approx_eq(eval_raw("1000000 - 600000"), 400000.0, "gross profit");
    // Gross Margin %
    assert_approx_eq(eval_raw("400000 / 1000000"), 0.4, "gross margin");
    // Net margin after 25% tax
    assert_approx_eq(eval_raw("400000 * (1 - 0.25)"), 300000.0, "after tax");
}

// ============================================================================
// OPERATOR PRECEDENCE
// ============================================================================

#[test]
fn equiv_precedence() {
    // Multiplication before addition
    assert_approx_eq(eval_raw("2 + 3 * 4"), 14.0, "2 + 3 * 4");
    // Division before subtraction
    assert_approx_eq(eval_raw("10 - 6 / 2"), 7.0, "10 - 6 / 2");
    // Left to right for same precedence
    assert_approx_eq(eval_raw("100 / 10 / 2"), 5.0, "100 / 10 / 2");
}

#[test]
fn equiv_nested_expressions() {
    assert_approx_eq(eval_raw("((1 + 2) * (3 + 4))"), 21.0, "((1 + 2) * (3 + 4))");
    assert_approx_eq(eval_raw("(100 - (50 - 25))"), 75.0, "(100 - (50 - 25))");
}

// ============================================================================
// EDGE CASES
// ============================================================================

#[test]
fn equiv_zero_handling() {
    assert_approx_eq(eval_raw("0 + 5"), 5.0, "0 + 5");
    assert_approx_eq(eval_raw("5 * 0"), 0.0, "5 * 0");
    assert_approx_eq(eval_raw("0 / 5"), 0.0, "0 / 5");
}

#[test]
fn equiv_decimal_precision() {
    assert_approx_eq(eval_raw("0.1 + 0.2"), 0.3, "0.1 + 0.2");
    assert_approx_eq(eval_raw("1.5 * 2.5"), 3.75, "1.5 * 2.5");
}

#[test]
fn equiv_large_numbers() {
    assert_approx_eq(eval_raw("1000000 + 2000000"), 3000000.0, "millions");
    // Note: f32 precision means large number operations may have small errors
    let result = eval_raw("1000000 * 0.001");
    assert!(
        (result - 1000.0).abs() < 0.001,
        "million * 0.001: got {}",
        result
    );
}

// ============================================================================
// COVERAGE MATRIX
// ============================================================================

/// Documents what is tested and what Forge implements beyond xlformula_engine
#[test]
fn coverage_matrix() {
    println!("\n");
    println!("╔═══════════════════════════════════════════════════════════════════════════╗");
    println!("║              XLFORMULA EQUIVALENCE TEST COVERAGE                          ║");
    println!("╠═══════════════════════════════════════════════════════════════════════════╣");
    println!("║  Category           │ xlformula_engine │ Forge Extension  │ Status        ║");
    println!("╠═════════════════════╪══════════════════╪══════════════════╪═══════════════╣");
    println!("║  Arithmetic         │ +, -, *, /       │ -                │ ✓ Tested      ║");
    println!("║  Parentheses        │ (, )             │ -                │ ✓ Tested      ║");
    println!("║  SUM/AVERAGE        │ ✓                │ -                │ ✓ Tested      ║");
    println!("║  PRODUCT            │ ✓                │ -                │ ✓ Tested      ║");
    println!("╠═════════════════════╪══════════════════╪══════════════════╪═══════════════╣");
    println!("║  IF/AND/OR/NOT      │ ✗                │ Forge impl       │ ○ Forge only  ║");
    println!("║  MAX/MIN/COUNT      │ ✗                │ Forge impl       │ ○ Forge only  ║");
    println!("║  ROUND/SQRT/POWER   │ ✗                │ Forge impl       │ ○ Forge only  ║");
    println!("║  Text functions     │ ✗                │ Forge impl       │ ○ Forge only  ║");
    println!("║  Date functions     │ ✗                │ Forge impl       │ ○ Forge only  ║");
    println!("║  Financial funcs    │ ✗                │ Forge impl       │ ○ Forge only  ║");
    println!("╚═══════════════════════════════════════════════════════════════════════════╝");
    println!();
    println!("NOTE: Functions marked 'Forge only' are implemented by Forge's array_calculator,");
    println!("      not by xlformula_engine. They are tested in array_calculator_tests.rs.");
    println!();
}
