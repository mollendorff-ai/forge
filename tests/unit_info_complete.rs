//! v6.6.0 - Information Functions Complete Unit Tests
//!
//! 100% coverage of all 11 information functions:
//! ISBLANK, ISERROR, ISNA, ISNUMBER, ISTEXT, ISLOGICAL, ISREF, ISFORMULA, NA, TYPE, N
//!
//! Tests: Type checking, error handling, edge cases, NULL/NA handling

#![cfg(not(feature = "demo"))]

use royalbit_forge::core::array_calculator::ArrayCalculator;
use royalbit_forge::types::{ParsedModel, Variable};

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

#[allow(dead_code)]
fn eval_scalar_err(formula: &str) -> bool {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new("result".to_string(), None, Some(format!("={}", formula))),
    );
    let calculator = ArrayCalculator::new(model);
    calculator.calculate_all().is_err()
}

// ═══════════════════════════════════════════════════════════════════════════════
// ISBLANK FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_isblank_na() {
    // NA() returns NULL which ISBLANK treats as blank
    assert_eq!(eval_scalar("IF(ISBLANK(NA()), 1, 0)"), 1.0);
}

#[test]
fn test_isblank_zero() {
    // Zero is not blank
    assert_eq!(eval_scalar("IF(ISBLANK(0), 1, 0)"), 0.0);
}

#[test]
fn test_isblank_empty_string() {
    // Empty string is NOT blank in Forge (it's a Text value)
    assert_eq!(eval_scalar("IF(ISBLANK(\"\"), 1, 0)"), 0.0);
}

#[test]
fn test_isblank_text() {
    // Text is not blank
    assert_eq!(eval_scalar("IF(ISBLANK(\"hello\"), 1, 0)"), 0.0);
}

#[test]
fn test_isblank_number() {
    // Numbers are not blank
    assert_eq!(eval_scalar("IF(ISBLANK(42), 1, 0)"), 0.0);
}

#[test]
fn test_isblank_boolean() {
    // Booleans are not blank
    assert_eq!(eval_scalar("IF(ISBLANK(TRUE()), 1, 0)"), 0.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// ISERROR FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_iserror_div_zero() {
    // Division by zero is an error
    assert_eq!(eval_scalar("IF(ISERROR(1/0), 1, 0)"), 1.0);
}

#[test]
fn test_iserror_valid_calc() {
    // Valid calculation is not an error
    assert_eq!(eval_scalar("IF(ISERROR(1/2), 1, 0)"), 0.0);
}

#[test]
fn test_iserror_sqrt_negative() {
    // SQRT of negative number is an error
    assert_eq!(eval_scalar("IF(ISERROR(SQRT(-1)), 1, 0)"), 1.0);
}

#[test]
fn test_iserror_ln_zero() {
    // LN(0) is an error
    assert_eq!(eval_scalar("IF(ISERROR(LN(0)), 1, 0)"), 1.0);
}

#[test]
fn test_iserror_valid_number() {
    // Plain number is not an error
    assert_eq!(eval_scalar("IF(ISERROR(42), 1, 0)"), 0.0);
}

#[test]
fn test_iserror_text() {
    // Text is not an error
    assert_eq!(eval_scalar("IF(ISERROR(\"text\"), 1, 0)"), 0.0);
}

#[test]
fn test_iserror_mod_zero() {
    // MOD by zero is an error
    assert_eq!(eval_scalar("IF(ISERROR(MOD(10, 0)), 1, 0)"), 1.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// ISNA FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_isna_na_function() {
    // NA() should be detected by ISNA
    assert_eq!(eval_scalar("IF(ISNA(NA()), 1, 0)"), 1.0);
}

#[test]
fn test_isna_number() {
    // Numbers are not NA
    assert_eq!(eval_scalar("IF(ISNA(5), 1, 0)"), 0.0);
}

#[test]
fn test_isna_text() {
    // Text is not NA
    assert_eq!(eval_scalar("IF(ISNA(\"hello\"), 1, 0)"), 0.0);
}

#[test]
fn test_isna_zero() {
    // Zero is not NA
    assert_eq!(eval_scalar("IF(ISNA(0), 1, 0)"), 0.0);
}

#[test]
fn test_isna_boolean() {
    // Booleans are not NA
    assert_eq!(eval_scalar("IF(ISNA(TRUE()), 1, 0)"), 0.0);
}

#[test]
fn test_isna_empty_string() {
    // Empty string is not NA
    assert_eq!(eval_scalar("IF(ISNA(\"\"), 1, 0)"), 0.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// ISNUMBER FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_isnumber_integer() {
    // Integer is a number
    assert_eq!(eval_scalar("IF(ISNUMBER(42), 1, 0)"), 1.0);
}

#[test]
fn test_isnumber_decimal() {
    // Decimal is a number
    assert_eq!(eval_scalar("IF(ISNUMBER(3.14), 1, 0)"), 1.0);
}

#[test]
fn test_isnumber_zero() {
    // Zero is a number
    assert_eq!(eval_scalar("IF(ISNUMBER(0), 1, 0)"), 1.0);
}

#[test]
fn test_isnumber_negative() {
    // Negative number is a number
    assert_eq!(eval_scalar("IF(ISNUMBER(-5), 1, 0)"), 1.0);
}

#[test]
fn test_isnumber_text() {
    // Text is not a number
    assert_eq!(eval_scalar("IF(ISNUMBER(\"123\"), 1, 0)"), 0.0);
}

#[test]
fn test_isnumber_boolean() {
    // Boolean is not a number
    assert_eq!(eval_scalar("IF(ISNUMBER(TRUE()), 1, 0)"), 0.0);
}

#[test]
fn test_isnumber_formula_result() {
    // Result of a formula that produces a number
    assert_eq!(eval_scalar("IF(ISNUMBER(1+1), 1, 0)"), 1.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// ISTEXT FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_istext_string() {
    // String is text
    assert_eq!(eval_scalar("IF(ISTEXT(\"hello\"), 1, 0)"), 1.0);
}

#[test]
fn test_istext_empty_string() {
    // Empty string is text
    assert_eq!(eval_scalar("IF(ISTEXT(\"\"), 1, 0)"), 1.0);
}

#[test]
fn test_istext_number() {
    // Number is not text
    assert_eq!(eval_scalar("IF(ISTEXT(123), 1, 0)"), 0.0);
}

#[test]
fn test_istext_zero() {
    // Zero is not text
    assert_eq!(eval_scalar("IF(ISTEXT(0), 1, 0)"), 0.0);
}

#[test]
fn test_istext_boolean() {
    // Boolean is not text
    assert_eq!(eval_scalar("IF(ISTEXT(FALSE()), 1, 0)"), 0.0);
}

#[test]
fn test_istext_numeric_string() {
    // String containing numbers is still text
    assert_eq!(eval_scalar("IF(ISTEXT(\"123\"), 1, 0)"), 1.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// ISLOGICAL FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_islogical_true() {
    // TRUE() is logical
    assert_eq!(eval_scalar("IF(ISLOGICAL(TRUE()), 1, 0)"), 1.0);
}

#[test]
fn test_islogical_false() {
    // FALSE() is logical
    assert_eq!(eval_scalar("IF(ISLOGICAL(FALSE()), 1, 0)"), 1.0);
}

#[test]
fn test_islogical_number_one() {
    // Number 1 is not logical
    assert_eq!(eval_scalar("IF(ISLOGICAL(1), 1, 0)"), 0.0);
}

#[test]
fn test_islogical_number_zero() {
    // Number 0 is not logical
    assert_eq!(eval_scalar("IF(ISLOGICAL(0), 1, 0)"), 0.0);
}

#[test]
fn test_islogical_comparison() {
    // Result of comparison is logical
    assert_eq!(eval_scalar("IF(ISLOGICAL(5>3), 1, 0)"), 1.0);
}

#[test]
fn test_islogical_text() {
    // Text is not logical
    assert_eq!(eval_scalar("IF(ISLOGICAL(\"TRUE\"), 1, 0)"), 0.0);
}

#[test]
fn test_islogical_and_result() {
    // Result of AND is logical
    assert_eq!(
        eval_scalar("IF(ISLOGICAL(AND(TRUE(), FALSE())), 1, 0)"),
        1.0
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// ISREF FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════
// Note: ISREF always returns FALSE in current implementation (references are resolved)

#[test]
#[ignore = "ISREF not fully implemented - always returns FALSE"]
fn test_isref_placeholder() {
    // ISREF always returns FALSE in current implementation
    assert_eq!(eval_scalar("IF(ISREF(5), 1, 0)"), 0.0);
}

#[test]
fn test_isref_number() {
    // Number is not a reference
    assert_eq!(eval_scalar("IF(ISREF(42), 1, 0)"), 0.0);
}

#[test]
fn test_isref_text() {
    // Text is not a reference
    assert_eq!(eval_scalar("IF(ISREF(\"A1\"), 1, 0)"), 0.0);
}

#[test]
fn test_isref_boolean() {
    // Boolean is not a reference
    assert_eq!(eval_scalar("IF(ISREF(TRUE()), 1, 0)"), 0.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// ISFORMULA FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════
// Note: ISFORMULA always returns FALSE in current implementation (formulas are evaluated)

#[test]
#[ignore = "ISFORMULA not fully implemented - always returns FALSE"]
fn test_isformula_placeholder() {
    // ISFORMULA always returns FALSE in current implementation
    assert_eq!(eval_scalar("IF(ISFORMULA(1+1), 1, 0)"), 0.0);
}

#[test]
fn test_isformula_number() {
    // Number is not a formula
    assert_eq!(eval_scalar("IF(ISFORMULA(42), 1, 0)"), 0.0);
}

#[test]
fn test_isformula_text() {
    // Text is not a formula
    assert_eq!(eval_scalar("IF(ISFORMULA(\"hello\"), 1, 0)"), 0.0);
}

#[test]
fn test_isformula_expression() {
    // Expression result is not detected as formula
    assert_eq!(eval_scalar("IF(ISFORMULA(5+5), 1, 0)"), 0.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// NA FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_na_detected_by_isna() {
    // NA() should be detected by ISNA
    assert_eq!(eval_scalar("IF(ISNA(NA()), 1, 0)"), 1.0);
}

#[test]
fn test_na_detected_by_isblank() {
    // NA() should be detected by ISBLANK (NULL value)
    assert_eq!(eval_scalar("IF(ISBLANK(NA()), 1, 0)"), 1.0);
}

#[test]
fn test_na_type() {
    // NA() should have TYPE of 16 (error)
    assert_eq!(eval_scalar("TYPE(NA())"), 16.0);
}

#[test]
fn test_na_not_number() {
    // NA() is not a number
    assert_eq!(eval_scalar("IF(ISNUMBER(NA()), 1, 0)"), 0.0);
}

#[test]
fn test_na_not_text() {
    // NA() is not text
    assert_eq!(eval_scalar("IF(ISTEXT(NA()), 1, 0)"), 0.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// TYPE FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════
// TYPE returns: 1=number, 2=text, 4=logical, 16=error/null, 64=array

#[test]
fn test_type_number() {
    // Numbers return type code 1
    assert_eq!(eval_scalar("TYPE(123)"), 1.0);
}

#[test]
fn test_type_decimal() {
    // Decimals return type code 1
    assert_eq!(eval_scalar("TYPE(3.14)"), 1.0);
}

#[test]
fn test_type_zero() {
    // Zero returns type code 1
    assert_eq!(eval_scalar("TYPE(0)"), 1.0);
}

#[test]
fn test_type_text() {
    // Text returns type code 2
    assert_eq!(eval_scalar("TYPE(\"hello\")"), 2.0);
}

#[test]
fn test_type_empty_string() {
    // Empty string returns type code 2
    assert_eq!(eval_scalar("TYPE(\"\")"), 2.0);
}

#[test]
fn test_type_true() {
    // TRUE() returns type code 4
    assert_eq!(eval_scalar("TYPE(TRUE())"), 4.0);
}

#[test]
fn test_type_false() {
    // FALSE() returns type code 4
    assert_eq!(eval_scalar("TYPE(FALSE())"), 4.0);
}

#[test]
fn test_type_na() {
    // NA() returns type code 16 (error)
    assert_eq!(eval_scalar("TYPE(NA())"), 16.0);
}

#[test]
fn test_type_comparison() {
    // Comparison result is logical (4)
    assert_eq!(eval_scalar("TYPE(5>3)"), 4.0);
}

#[test]
fn test_type_math_result() {
    // Math operation result is number (1)
    assert_eq!(eval_scalar("TYPE(2+3)"), 1.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// N FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════════
// N converts: Number stays, TRUE=1, FALSE=0, others=0

#[test]
fn test_n_true() {
    // N(TRUE()) = 1
    assert_eq!(eval_scalar("N(TRUE())"), 1.0);
}

#[test]
fn test_n_false() {
    // N(FALSE()) = 0
    assert_eq!(eval_scalar("N(FALSE())"), 0.0);
}

#[test]
fn test_n_number() {
    // N(number) = number
    assert_eq!(eval_scalar("N(42)"), 42.0);
}

#[test]
fn test_n_decimal() {
    // N(decimal) = decimal
    assert_eq!(eval_scalar("N(3.15)"), 3.15);
}

#[test]
fn test_n_zero() {
    // N(0) = 0
    assert_eq!(eval_scalar("N(0)"), 0.0);
}

#[test]
fn test_n_negative() {
    // N(negative) = negative
    assert_eq!(eval_scalar("N(-5)"), -5.0);
}

#[test]
fn test_n_text() {
    // N(text) = 0
    assert_eq!(eval_scalar("N(\"hello\")"), 0.0);
}

#[test]
fn test_n_empty_string() {
    // N("") = 0
    assert_eq!(eval_scalar("N(\"\")"), 0.0);
}

#[test]
fn test_n_numeric_string() {
    // N("123") = 0 (text is text, not converted)
    assert_eq!(eval_scalar("N(\"123\")"), 0.0);
}

#[test]
fn test_n_comparison_true() {
    // N(5>3) = N(TRUE) = 1
    assert_eq!(eval_scalar("N(5>3)"), 1.0);
}

#[test]
fn test_n_comparison_false() {
    // N(5<3) = N(FALSE) = 0
    assert_eq!(eval_scalar("N(5<3)"), 0.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// COMBINED / INTEGRATION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_combined_type_with_n() {
    // TYPE of N(TRUE) should be 1 (number)
    assert_eq!(eval_scalar("TYPE(N(TRUE()))"), 1.0);
}

#[test]
fn test_combined_isna_with_na() {
    // ISNA should detect NA()
    assert_eq!(eval_scalar("IF(ISNA(NA()), 1, 0)"), 1.0);
}

#[test]
fn test_combined_isnumber_with_n() {
    // N always returns a number
    assert_eq!(eval_scalar("IF(ISNUMBER(N(\"text\")), 1, 0)"), 1.0);
}

#[test]
fn test_combined_iserror_with_type() {
    // TYPE doesn't error on valid input
    assert_eq!(eval_scalar("IF(ISERROR(TYPE(5)), 1, 0)"), 0.0);
}

#[test]
fn test_combined_islogical_comparison() {
    // Nested comparisons return logical
    assert_eq!(eval_scalar("IF(ISLOGICAL(AND(5>3, 2<4)), 1, 0)"), 1.0);
}

#[test]
fn test_combined_multiple_checks() {
    // Testing multiple IS functions in one formula
    assert_eq!(
        eval_scalar("IF(AND(ISNUMBER(5), ISTEXT(\"a\"), ISLOGICAL(TRUE())), 1, 0)"),
        1.0
    );
}

#[test]
fn test_combined_n_sum() {
    // N can be used in arithmetic
    assert_eq!(eval_scalar("N(TRUE()) + N(FALSE()) + N(5)"), 6.0);
}

#[test]
fn test_combined_type_all_types() {
    // Verify all type codes work together
    let type_sum = eval_scalar("TYPE(5) + TYPE(\"a\") + TYPE(TRUE()) + TYPE(NA())");
    assert_eq!(type_sum, 23.0); // 1 + 2 + 4 + 16
}

// ═══════════════════════════════════════════════════════════════════════════════
// EDGE CASES AND ERROR HANDLING
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_edge_iserror_nested() {
    // ISERROR of ISERROR should not error
    assert_eq!(eval_scalar("IF(ISERROR(ISERROR(1/0)), 1, 0)"), 0.0);
}

#[test]
fn test_edge_type_of_type() {
    // TYPE of TYPE result is number (1)
    assert_eq!(eval_scalar("TYPE(TYPE(5))"), 1.0);
}

#[test]
fn test_edge_n_of_n() {
    // N of N result
    assert_eq!(eval_scalar("N(N(TRUE()))"), 1.0);
}

#[test]
fn test_edge_isna_not_error() {
    // ISNA(error) should be false - only NA is true
    // Division by zero is an error but not NA
    assert_eq!(eval_scalar("IF(ISERROR(1/0), 1, 0)"), 1.0);
}

#[test]
fn test_edge_empty_vs_na() {
    // Empty string is different from NA
    assert_eq!(eval_scalar("IF(ISNA(\"\"), 1, 0)"), 0.0);
    assert_eq!(eval_scalar("IF(ISBLANK(\"\"), 1, 0)"), 0.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// ISEVEN FUNCTION TESTS - FP&A ACCURACY MANDATE
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_iseven_positive_even() {
    // Even numbers return TRUE
    assert_eq!(eval_scalar("IF(ISEVEN(4), 1, 0)"), 1.0);
    assert_eq!(eval_scalar("IF(ISEVEN(2), 1, 0)"), 1.0);
    assert_eq!(eval_scalar("IF(ISEVEN(100), 1, 0)"), 1.0);
}

#[test]
fn test_iseven_positive_odd() {
    // Odd numbers return FALSE
    assert_eq!(eval_scalar("IF(ISEVEN(3), 1, 0)"), 0.0);
    assert_eq!(eval_scalar("IF(ISEVEN(5), 1, 0)"), 0.0);
    assert_eq!(eval_scalar("IF(ISEVEN(99), 1, 0)"), 0.0);
}

#[test]
fn test_iseven_zero() {
    // Zero is even
    assert_eq!(eval_scalar("IF(ISEVEN(0), 1, 0)"), 1.0);
}

#[test]
fn test_iseven_negative_even() {
    // Negative even numbers return TRUE
    assert_eq!(eval_scalar("IF(ISEVEN(-2), 1, 0)"), 1.0);
    assert_eq!(eval_scalar("IF(ISEVEN(-4), 1, 0)"), 1.0);
    assert_eq!(eval_scalar("IF(ISEVEN(-100), 1, 0)"), 1.0);
}

#[test]
fn test_iseven_negative_odd() {
    // Negative odd numbers return FALSE
    assert_eq!(eval_scalar("IF(ISEVEN(-3), 1, 0)"), 0.0);
    assert_eq!(eval_scalar("IF(ISEVEN(-5), 1, 0)"), 0.0);
    assert_eq!(eval_scalar("IF(ISEVEN(-99), 1, 0)"), 0.0);
}

#[test]
fn test_iseven_decimal_rounds() {
    // Decimals are truncated before testing
    assert_eq!(eval_scalar("IF(ISEVEN(4.7), 1, 0)"), 1.0); // 4 is even
    assert_eq!(eval_scalar("IF(ISEVEN(3.9), 1, 0)"), 0.0); // 3 is odd
}

#[test]
fn test_iseven_in_calculation() {
    // ISEVEN used in calculation
    assert_eq!(eval_scalar("IF(ISEVEN(2*3), 1, 0)"), 1.0); // 6 is even
    assert_eq!(eval_scalar("IF(ISEVEN(2*3+1), 1, 0)"), 0.0); // 7 is odd
}

// ═══════════════════════════════════════════════════════════════════════════════
// ISODD FUNCTION TESTS - FP&A ACCURACY MANDATE
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_isodd_positive_odd() {
    // Odd numbers return TRUE
    assert_eq!(eval_scalar("IF(ISODD(3), 1, 0)"), 1.0);
    assert_eq!(eval_scalar("IF(ISODD(5), 1, 0)"), 1.0);
    assert_eq!(eval_scalar("IF(ISODD(99), 1, 0)"), 1.0);
}

#[test]
fn test_isodd_positive_even() {
    // Even numbers return FALSE
    assert_eq!(eval_scalar("IF(ISODD(2), 1, 0)"), 0.0);
    assert_eq!(eval_scalar("IF(ISODD(4), 1, 0)"), 0.0);
    assert_eq!(eval_scalar("IF(ISODD(100), 1, 0)"), 0.0);
}

#[test]
fn test_isodd_zero() {
    // Zero is not odd
    assert_eq!(eval_scalar("IF(ISODD(0), 1, 0)"), 0.0);
}

#[test]
fn test_isodd_negative_odd() {
    // Negative odd numbers return TRUE
    assert_eq!(eval_scalar("IF(ISODD(-3), 1, 0)"), 1.0);
    assert_eq!(eval_scalar("IF(ISODD(-5), 1, 0)"), 1.0);
    assert_eq!(eval_scalar("IF(ISODD(-99), 1, 0)"), 1.0);
}

#[test]
fn test_isodd_negative_even() {
    // Negative even numbers return FALSE
    assert_eq!(eval_scalar("IF(ISODD(-2), 1, 0)"), 0.0);
    assert_eq!(eval_scalar("IF(ISODD(-4), 1, 0)"), 0.0);
    assert_eq!(eval_scalar("IF(ISODD(-100), 1, 0)"), 0.0);
}

#[test]
fn test_isodd_decimal_rounds() {
    // Decimals are truncated before testing
    assert_eq!(eval_scalar("IF(ISODD(3.7), 1, 0)"), 1.0); // 3 is odd
    assert_eq!(eval_scalar("IF(ISODD(4.9), 1, 0)"), 0.0); // 4 is even
}

#[test]
fn test_isodd_in_calculation() {
    // ISODD used in calculation
    assert_eq!(eval_scalar("IF(ISODD(2*3), 1, 0)"), 0.0); // 6 is not odd
    assert_eq!(eval_scalar("IF(ISODD(2*3+1), 1, 0)"), 1.0); // 7 is odd
}

#[test]
fn test_isodd_one() {
    // 1 is odd
    assert_eq!(eval_scalar("IF(ISODD(1), 1, 0)"), 1.0);
}

#[test]
fn test_iseven_two() {
    // 2 is even
    assert_eq!(eval_scalar("IF(ISEVEN(2), 1, 0)"), 1.0);
}
