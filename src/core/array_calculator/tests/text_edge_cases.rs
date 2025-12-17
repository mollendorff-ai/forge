//! Edge case tests for TEXT functions - 100% COVERAGE MANDATORY
//!
//! These tests cover all edge cases including:
//! - Empty string handling
//! - Position out of bounds
//! - Not found scenarios
//! - Special characters
//! - Case sensitivity
//! - Unicode handling

#![allow(clippy::approx_constant)]

use crate::core::array_calculator::ArrayCalculator;
use crate::types::{ParsedModel, Variable};

// ══════════════════════════════════════════════════════════════════════════════
// EMPTY STRING TESTS
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
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(0.0));
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
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(0.0));
}

#[test]
fn test_mid_empty_string() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(MID(\"\", 1, 5))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(0.0));
}

#[test]
fn test_upper_empty_string_verified() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(UPPER(\"\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(0.0));
}

#[test]
fn test_lower_empty_string_verified() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(LOWER(\"\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(0.0));
}

#[test]
fn test_trim_empty_string() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(TRIM(\"\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(0.0));
}

#[test]
fn test_concat_empty_strings() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(CONCAT(\"\", \"\", \"\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(0.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// OUT OF BOUNDS TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_left_count_larger_than_string() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(LEFT(\"abc\", 100))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    // Should return entire string when count exceeds length
    assert_eq!(var.value, Some(3.0));
}

#[test]
fn test_right_count_larger_than_string() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(RIGHT(\"abc\", 100))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    // Should return entire string when count exceeds length
    assert_eq!(var.value, Some(3.0));
}

#[test]
fn test_mid_start_beyond_string_length() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(MID(\"abc\", 10, 5))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    // Should return empty string when start is beyond length
    assert_eq!(var.value, Some(0.0));
}

#[test]
fn test_mid_length_exceeds_remaining() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(MID(\"hello\", 3, 100))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    // Should return "llo" (from position 3 to end) = 3 chars
    assert_eq!(var.value, Some(3.0));
}

#[test]
fn test_left_zero_count() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(LEFT(\"hello\", 0))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    // LEFT with 0 count should return empty string
    assert_eq!(var.value, Some(0.0));
}

#[test]
fn test_right_zero_count() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(RIGHT(\"hello\", 0))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    // RIGHT with 0 count should return empty string
    assert_eq!(var.value, Some(0.0));
}

#[test]
fn test_mid_zero_length() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(MID(\"hello\", 2, 0))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    // MID with 0 length should return empty string
    assert_eq!(var.value, Some(0.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// NOT FOUND TESTS (ENTERPRISE ONLY)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
#[cfg(not(feature = "demo"))]
fn test_find_character_not_in_string() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "pos".to_string(),
        Variable::new(
            "pos".to_string(),
            None,
            Some("=FIND(\"x\", \"abc\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should return error when character not found
    assert!(result.is_err());
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_search_substring_not_found() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "pos".to_string(),
        Variable::new(
            "pos".to_string(),
            None,
            Some("=SEARCH(\"xyz\", \"abc\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should return error when substring not found
    assert!(result.is_err());
}

// ══════════════════════════════════════════════════════════════════════════════
// CASE SENSITIVITY TESTS (ENTERPRISE ONLY)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
#[cfg(not(feature = "demo"))]
fn test_find_case_sensitive() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "pos".to_string(),
        Variable::new(
            "pos".to_string(),
            None,
            Some("=FIND(\"H\", \"hello\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // FIND is case-sensitive, so "H" should not be found in "hello"
    assert!(result.is_err());
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_search_case_insensitive_verified() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "pos".to_string(),
        Variable::new(
            "pos".to_string(),
            None,
            Some("=SEARCH(\"HELLO\", \"hello world\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("pos").unwrap();
    // SEARCH is case-insensitive, should find at position 1
    assert_eq!(var.value, Some(1.0));
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_find_with_start_position() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "pos".to_string(),
        Variable::new(
            "pos".to_string(),
            None,
            Some("=FIND(\"o\", \"hello world\", 6)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("pos").unwrap();
    // Should find the second "o" at position 8
    assert_eq!(var.value, Some(8.0));
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_search_with_start_position() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "pos".to_string(),
        Variable::new(
            "pos".to_string(),
            None,
            Some("=SEARCH(\"L\", \"hello world\", 4)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("pos").unwrap();
    // Should find "l" at position 4 (case insensitive)
    assert_eq!(var.value, Some(4.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// SPECIAL CHARACTERS TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_trim_multiple_spaces() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(TRIM(\"     hello     \"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(5.0));
}

#[test]
fn test_trim_tabs_and_newlines() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(TRIM(\"\t\nhello\t\n\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(5.0));
}

#[test]
fn test_trim_only_spaces() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(TRIM(\"     \"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    // All spaces should be trimmed, leaving empty string
    assert_eq!(var.value, Some(0.0));
}

#[test]
fn test_len_special_characters() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(\"!@#$%^&*()\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(10.0));
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_substitute_with_special_chars() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(SUBSTITUTE(\"a-b-c\", \"-\", \"_\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(5.0));
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_substitute_empty_old_text() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(SUBSTITUTE(\"hello\", \"\", \"x\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    // Substituting empty string should return original
    assert_eq!(var.value, Some(5.0));
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_substitute_no_replacement_needed() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(SUBSTITUTE(\"abc\", \"xyz\", \"123\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    // No match, should return original string
    assert_eq!(var.value, Some(3.0));
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_replace_entire_string() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(REPLACE(\"hello\", 1, 5, \"world\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    // Replace all characters
    assert_eq!(var.value, Some(5.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// UNICODE TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_len_unicode_characters() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(\"hello世界\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    // "hello" = 5 chars + "世界" = 2 chars = 7 total
    assert_eq!(var.value, Some(7.0));
}

#[test]
fn test_left_with_unicode() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(LEFT(\"hello世界\", 6))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(6.0));
}

#[test]
fn test_right_with_unicode() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(RIGHT(\"hello世界\", 3))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(3.0));
}

#[test]
fn test_mid_with_unicode() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(MID(\"hello世界\", 5, 3))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    // Starting at position 5 ("o"), take 3 chars: "o世界"
    assert_eq!(var.value, Some(3.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// MIXED TYPES TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_concat_mixed_types() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "num".to_string(),
        Variable::new("num".to_string(), Some(42.0), None),
    );
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(CONCAT(\"The answer is \", num))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(16.0));
}

#[test]
fn test_upper_with_numbers() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(UPPER(\"test123\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    // Numbers should remain unchanged
    assert_eq!(var.value, Some(7.0));
}

#[test]
fn test_lower_with_numbers() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(LOWER(\"TEST123\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    // Numbers should remain unchanged
    assert_eq!(var.value, Some(7.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// VALUE FUNCTION TESTS (ENTERPRISE ONLY)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
#[cfg(not(feature = "demo"))]
fn test_value_with_thousand_separators() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=VALUE(\"1,234,567\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(1234567.0));
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_value_with_whitespace() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=VALUE(\"   100.5   \")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(100.5));
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_value_invalid_text() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=VALUE(\"not a number\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should return error for invalid number text
    assert!(result.is_err());
}

// ══════════════════════════════════════════════════════════════════════════════
// TEXT FUNCTION TESTS (ENTERPRISE ONLY)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
#[cfg(not(feature = "demo"))]
fn test_text_percentage_format() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(TEXT(0.5, \"0%\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(3.0)); // "50%"
}

#[test]
#[cfg(not(feature = "demo"))]
fn test_text_decimal_format() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(TEXT(123.456, \"0.00\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(6.0)); // "123.46"
}

// ══════════════════════════════════════════════════════════════════════════════
// REPT FUNCTION TEST (DEMO)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_rept_multiple_times() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(REPT(\"ab\", 5))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(10.0)); // "ab" * 5 = "ababababab" = 10 chars
}

#[test]
fn test_rept_zero_times() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(REPT(\"hello\", 0))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(0.0)); // Empty string
}

#[test]
fn test_rept_once() {
    let mut model = ParsedModel::new();
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEN(REPT(\"test\", 1))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(4.0)); // "test" = 4 chars
}
