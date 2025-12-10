//! Error Propagation Tests - 100% Coverage
//!
//! Tests for all error types and error propagation behavior:
//! - Division by zero (#DIV/0!)
//! - Type mismatch (#VALUE!)
//! - Invalid reference (#REF!)
//! - Invalid numeric (#NUM!)
//! - Not available (#N/A)
//! - Error handling functions (IFERROR, ISERROR, ISNA)
//! - Error propagation in formulas

#![allow(clippy::approx_constant)]

use crate::core::array_calculator::ArrayCalculator;
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

// ═══════════════════════════════════════════════════════════════════════════
// DIVISION BY ZERO (#DIV/0!)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_division_by_zero_direct() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new("x".to_string(), ColumnValue::Number(vec![1.0])));
    table.add_row_formula("result".to_string(), "=1 / 0".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Division by zero"));
}

#[test]
fn test_division_by_zero_in_formula() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new(
        "a".to_string(),
        ColumnValue::Number(vec![10.0, 5.0]),
    ));
    table.add_column(Column::new(
        "b".to_string(),
        ColumnValue::Number(vec![2.0, 0.0]),
    ));
    table.add_row_formula("result".to_string(), "=a / b".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Second row has division by zero
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Division by zero"));
}

#[test]
fn test_division_by_zero_scalar() {
    let mut model = ParsedModel::new();
    let var = Variable::new("result".to_string(), None, Some("=10 / 0".to_string()));
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Division by zero"));
}

#[test]
fn test_division_by_zero_in_nested_formula() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new("x".to_string(), ColumnValue::Number(vec![1.0])));
    table.add_row_formula("result".to_string(), "=(10 + 5) / (2 - 2)".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Division by zero"));
}

// ═══════════════════════════════════════════════════════════════════════════
// TYPE MISMATCH (#VALUE!)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_type_mismatch_text_plus_number() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new(
        "text".to_string(),
        ColumnValue::Text(vec!["abc".to_string()]),
    ));
    table.add_row_formula("result".to_string(), "=text - 1".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must be a number"));
}

#[test]
fn test_type_mismatch_invalid_sqrt_argument() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new(
        "text".to_string(),
        ColumnValue::Text(vec!["hello".to_string()]),
    ));
    table.add_row_formula("result".to_string(), "=SQRT(text)".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("requires a number"));
}

#[test]
fn test_type_mismatch_abs_with_text() {
    let mut model = ParsedModel::new();
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=ABS(\"text\")".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("requires a number"));
}

// ═══════════════════════════════════════════════════════════════════════════
// INVALID REFERENCE (#REF!)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_invalid_reference_nonexistent_column() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new(
        "a".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]),
    ));
    table.add_row_formula("result".to_string(), "=nonexistent + a".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown variable"));
}

#[test]
fn test_invalid_reference_nonexistent_table() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new("x".to_string(), ColumnValue::Number(vec![1.0])));
    table.add_row_formula(
        "result".to_string(),
        "=nonexistent_table.column".to_string(),
    );
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown"));
}

#[test]
fn test_invalid_reference_nonexistent_table_column() {
    let mut model = ParsedModel::new();

    let mut table1 = Table::new("table1".to_string());
    table1.add_column(Column::new(
        "a".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]),
    ));
    model.add_table(table1);

    let mut table2 = Table::new("table2".to_string());
    table2.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]),
    ));
    table2.add_row_formula("result".to_string(), "=table1.nonexistent".to_string());
    model.add_table(table2);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown column"));
}

#[test]
fn test_invalid_reference_scalar() {
    let mut model = ParsedModel::new();
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=nonexistent_var".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown variable"));
}

// ═══════════════════════════════════════════════════════════════════════════
// INVALID NUMERIC (#NUM!)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_invalid_numeric_sqrt_negative() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new("x".to_string(), ColumnValue::Number(vec![1.0])));
    table.add_row_formula("result".to_string(), "=SQRT(-1)".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("SQRT of negative"));
}

#[test]
fn test_invalid_numeric_sqrt_negative_variable() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![-4.0, 9.0]),
    ));
    table.add_row_formula("result".to_string(), "=SQRT(value)".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // First row has negative value
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("SQRT of negative"));
}

#[test]
fn test_invalid_numeric_ln_zero() {
    let mut model = ParsedModel::new();
    let var = Variable::new("result".to_string(), None, Some("=LN(0)".to_string()));
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    // LN(0) should produce an error (result is -inf which may cause issues)
}

#[test]
fn test_invalid_numeric_ln_negative() {
    let mut model = ParsedModel::new();
    let var = Variable::new("result".to_string(), None, Some("=LN(-5)".to_string()));
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    // LN of negative should produce an error
}

#[test]
fn test_invalid_numeric_log_zero() {
    let mut model = ParsedModel::new();
    let var = Variable::new("result".to_string(), None, Some("=LOG(0)".to_string()));
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════════
// NOT AVAILABLE (#N/A)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_na_match_not_found() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("lookup".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    model.add_table(table);

    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=MATCH(99, lookup.values, 0)".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_na_match_approximate_not_found() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("lookup".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(table);

    // Looking for 5 in [10, 20, 30] with match_type=1 (less than or equal)
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=MATCH(5, lookup.values, 1)".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_na_index_out_of_bounds() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    model.add_table(table);

    // INDEX is 1-based, accessing row 10 when only 3 rows exist
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=INDEX(data.values, 10)".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("out of bounds"));
}

#[test]
fn test_na_index_zero() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    model.add_table(table);

    // INDEX is 1-based, row_num must be >= 1
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=INDEX(data.values, 0)".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must be >= 1"));
}

// ═══════════════════════════════════════════════════════════════════════════
// ERROR HANDLING FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_iferror_catches_division_by_zero() {
    let mut model = ParsedModel::new();
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=IFERROR(1/0, 999)".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Should handle error with IFERROR");

    let value = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(value, 999.0);
}

#[test]
fn test_iferror_returns_value_when_no_error() {
    let mut model = ParsedModel::new();
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=IFERROR(10/2, 999)".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Should calculate successfully");

    let value = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(value, 5.0);
}

#[test]
fn test_iferror_with_sqrt_negative() {
    let mut model = ParsedModel::new();
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=IFERROR(SQRT(-1), -1)".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Should handle error with IFERROR");

    let value = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(value, -1.0);
}

#[test]
fn test_iferror_with_invalid_reference() {
    let mut model = ParsedModel::new();
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=IFERROR(nonexistent, 0)".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Should handle error with IFERROR");

    let value = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(value, 0.0);
}

#[test]
fn test_iferror_nested() {
    let mut model = ParsedModel::new();
    // IFERROR inside IFERROR
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=IFERROR(IFERROR(1/0, 2/0), 100)".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Should handle nested IFERROR");

    let value = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(value, 100.0);
}

#[test]
#[cfg(feature = "full")]
fn test_iserror_detects_division_by_zero() {
    let mut model = ParsedModel::new();
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=IF(ISERROR(1/0), 1, 0)".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Should detect error with ISERROR");

    let value = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(value, 1.0);
}

#[test]
#[cfg(feature = "full")]
fn test_iserror_returns_false_for_valid_value() {
    let mut model = ParsedModel::new();
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=IF(ISERROR(5), 1, 0)".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Should calculate successfully");

    let value = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(value, 0.0);
}

#[test]
#[cfg(feature = "full")]
fn test_iserror_detects_sqrt_negative() {
    let mut model = ParsedModel::new();
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=IF(ISERROR(SQRT(-1)), 1, 0)".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Should detect error with ISERROR");

    let value = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(value, 1.0);
}

// ═══════════════════════════════════════════════════════════════════════════
// ERROR PROPAGATION IN NESTED FORMULAS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_error_propagation_in_arithmetic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new("x".to_string(), ColumnValue::Number(vec![1.0])));
    // Error in part of formula should propagate
    table.add_row_formula("result".to_string(), "=5 + (1/0) + 3".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Division by zero"));
}

#[test]
fn test_error_propagation_sqrt_in_sum() {
    let mut model = ParsedModel::new();
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=1 + SQRT(-1) + 3".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("SQRT of negative"));
}

#[test]
fn test_error_propagation_if_true_branch() {
    let mut model = ParsedModel::new();
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=IF(1, 1/0, 5)".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Division by zero"));
}

#[test]
fn test_error_no_propagation_if_false_branch() {
    let mut model = ParsedModel::new();
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=IF(0, 1/0, 5)".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("False branch should not be evaluated");

    let value = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(value, 5.0);
}

#[test]
fn test_error_propagation_nested_functions() {
    let mut model = ParsedModel::new();
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=ABS(SQRT(-4))".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("SQRT of negative"));
}

#[test]
fn test_error_propagation_in_multiplication() {
    let mut model = ParsedModel::new();
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=10 * (5 / 0)".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Division by zero"));
}

#[test]
fn test_error_propagation_power_with_error() {
    let mut model = ParsedModel::new();
    let var = Variable::new("result".to_string(), None, Some("=(1/0) ^ 2".to_string()));
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Division by zero"));
}

#[test]
fn test_error_iferror_with_text_fallback() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new("x".to_string(), ColumnValue::Number(vec![1.0])));
    // IFERROR should work with text fallback (though result will be text)
    table.add_row_formula("result".to_string(), "=IFERROR(1/0, \"error\")".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("IFERROR should handle error");

    let col = result
        .tables
        .get("test")
        .unwrap()
        .columns
        .get("result")
        .unwrap();
    if let ColumnValue::Text(values) = &col.values {
        assert_eq!(values[0], "error");
    } else {
        panic!("Expected text result from IFERROR");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// COMPLEX ERROR SCENARIOS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_error_in_dependent_formula() {
    let mut model = ParsedModel::new();

    let var1 = Variable::new("a".to_string(), None, Some("=1/0".to_string()));
    model.add_scalar("a".to_string(), var1);

    let var2 = Variable::new("b".to_string(), None, Some("=a + 5".to_string()));
    model.add_scalar("b".to_string(), var2);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Error in 'a' should prevent calculation of 'b'
    assert!(result.is_err());
}

#[test]
fn test_type_error_comparison() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new(
        "text".to_string(),
        ColumnValue::Text(vec!["abc".to_string()]),
    ));
    table.add_row_formula("result".to_string(), "=text > 5".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must be a number"));
}

#[test]
fn test_array_index_out_of_bounds_large() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    model.add_table(table);

    // Large index should error
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=data.values[999]".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("out of bounds"));
}

#[test]
fn test_multiple_errors_first_one_reported() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new("x".to_string(), ColumnValue::Number(vec![1.0])));
    // Multiple errors in one formula - first error should be reported
    table.add_row_formula("result".to_string(), "=(1/0) + SQRT(-1)".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    // First error (division by zero) should be reported
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Division by zero"));
}

#[test]
fn test_iferror_protects_from_multiple_errors() {
    let mut model = ParsedModel::new();
    let var = Variable::new(
        "result".to_string(),
        None,
        Some("=IFERROR((1/0) + SQRT(-1), 42)".to_string()),
    );
    model.add_scalar("result".to_string(), var);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("IFERROR should catch all errors");

    let value = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(value, 42.0);
}
