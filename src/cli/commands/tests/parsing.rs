//! Parsing tests for CLI commands

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI

use super::super::*;

// =========================================================================
// format_number Tests
// =========================================================================

#[test]
fn test_parse_range_basic() {
    // Format is "start,end,step"
    let result = parse_range("0,10,2").unwrap();
    assert_eq!(result, vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
}

#[test]
fn test_parse_range_decimal_step() {
    let result = parse_range("0,1,0.25").unwrap();
    assert_eq!(result.len(), 5);
    assert!((result[0] - 0.0).abs() < 0.0001);
    assert!((result[1] - 0.25).abs() < 0.0001);
    assert!((result[4] - 1.0).abs() < 0.0001);
}

#[test]
fn test_parse_range_negative_values() {
    let result = parse_range("-5,-1,1").unwrap();
    assert_eq!(result, vec![-5.0, -4.0, -3.0, -2.0, -1.0]);
}

#[test]
fn test_parse_range_financial() {
    // Typical rate sensitivity: 1% to 15% in 2% steps
    let result = parse_range("0.01,0.15,0.02").unwrap();
    assert_eq!(result.len(), 8); // 0.01, 0.03, 0.05, 0.07, 0.09, 0.11, 0.13, 0.15
}

#[test]
fn test_parse_range_invalid_format_too_few_parts() {
    let result = parse_range("1,5");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Invalid range format"));
}

#[test]
fn test_parse_range_invalid_format_too_many_parts() {
    let result = parse_range("1,2,3,4");
    assert!(result.is_err());
}

#[test]
fn test_parse_range_invalid_start_value() {
    let result = parse_range("abc,10,1");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Invalid start value"));
}

#[test]
fn test_parse_range_invalid_end_value() {
    let result = parse_range("0,xyz,1");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Invalid end value"));
}

#[test]
fn test_parse_range_invalid_step_value() {
    let result = parse_range("0,10,bad");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Invalid step value"));
}

#[test]
fn test_parse_range_zero_step() {
    let result = parse_range("0,10,0");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Step must be positive"));
}

#[test]
fn test_parse_range_negative_step() {
    let result = parse_range("0,10,-1");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Step must be positive"));
}

#[test]
fn test_parse_range_start_greater_than_end() {
    let result = parse_range("10,0,1");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Start must be less than or equal to end"));
}

#[test]
fn test_parse_range_start_equals_end() {
    let result = parse_range("5,5,1").unwrap();
    assert_eq!(result, vec![5.0]);
}

#[test]
fn test_parse_range_with_spaces() {
    let result = parse_range(" 0 , 10 , 2 ").unwrap();
    assert_eq!(result, vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
}

#[test]
fn test_extract_references_simple() {
    let refs = extract_references_from_formula("=a + b");
    assert!(refs.contains(&"a".to_string()));
    assert!(refs.contains(&"b".to_string()));
}

#[test]
fn test_extract_references_with_underscores() {
    let refs = extract_references_from_formula("=total_revenue - total_costs");
    assert!(refs.contains(&"total_revenue".to_string()));
    assert!(refs.contains(&"total_costs".to_string()));
}

#[test]
fn test_extract_references_filters_functions() {
    let refs = extract_references_from_formula("=SUM(revenue) + MAX(costs)");
    assert!(refs.contains(&"revenue".to_string()));
    assert!(refs.contains(&"costs".to_string()));
    assert!(!refs.contains(&"SUM".to_string()));
    assert!(!refs.contains(&"MAX".to_string()));
}

#[test]
fn test_extract_references_filters_all_known_functions() {
    let refs = extract_references_from_formula("=IF(AND(x, OR(y, z)), AVERAGE(data), MIN(values))");
    assert!(refs.contains(&"x".to_string()));
    assert!(refs.contains(&"y".to_string()));
    assert!(refs.contains(&"z".to_string()));
    assert!(refs.contains(&"data".to_string()));
    assert!(refs.contains(&"values".to_string()));
    assert!(!refs.contains(&"IF".to_string()));
    assert!(!refs.contains(&"AND".to_string()));
    assert!(!refs.contains(&"OR".to_string()));
    assert!(!refs.contains(&"AVERAGE".to_string()));
    assert!(!refs.contains(&"MIN".to_string()));
}

#[test]
fn test_extract_references_filters_numbers() {
    let refs = extract_references_from_formula("=revenue * 0.15 + 100");
    assert!(refs.contains(&"revenue".to_string()));
    assert!(!refs.iter().any(|r| r.starts_with('0')));
    assert!(!refs.iter().any(|r| r.starts_with('1')));
}

#[test]
fn test_extract_references_empty_formula() {
    let refs = extract_references_from_formula("");
    assert!(refs.is_empty());
}

#[test]
fn test_extract_references_literal_only() {
    let refs = extract_references_from_formula("=100");
    assert!(refs.is_empty());
}

#[test]
fn test_extract_references_no_duplicates() {
    let refs = extract_references_from_formula("=a + a + a");
    assert_eq!(refs.len(), 1);
    assert!(refs.contains(&"a".to_string()));
}

#[test]
fn test_extract_references_strips_equals() {
    let refs = extract_references_from_formula("=price");
    assert!(refs.contains(&"price".to_string()));
    assert!(!refs.contains(&"=price".to_string()));
}

#[test]
fn test_extract_references_complex_formula() {
    let refs = extract_references_from_formula(
        "=SUMIF(categories, expenses) + IFERROR(overhead / months, 0)",
    );
    assert!(refs.contains(&"categories".to_string()));
    assert!(refs.contains(&"expenses".to_string()));
    assert!(refs.contains(&"overhead".to_string()));
    assert!(refs.contains(&"months".to_string()));
    // Numbers in formulas should not be extracted
    assert!(!refs.iter().any(|r| r == "0"));
}

#[test]
fn test_extract_references_case_insensitive_functions() {
    // Functions should be filtered regardless of case
    let refs = extract_references_from_formula("=sum(data) + Sum(more) + SUM(again)");
    assert!(refs.contains(&"data".to_string()));
    assert!(refs.contains(&"more".to_string()));
    assert!(refs.contains(&"again".to_string()));
    assert!(!refs.iter().any(|r| r.to_uppercase() == "SUM"));
}
