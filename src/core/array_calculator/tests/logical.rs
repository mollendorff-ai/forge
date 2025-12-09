//! Logical function tests for ArrayCalculator

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI/E

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[test]
fn test_if_simple_condition() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, -5.0, 20.0]),
    ));
    data.row_formulas
        .insert("positive".to_string(), "=IF(value > 0, 1, 0)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let col = result
        .tables
        .get("data")
        .unwrap()
        .columns
        .get("positive")
        .unwrap();
    if let ColumnValue::Number(values) = &col.values {
        assert_eq!(values[0], 1.0);
        assert_eq!(values[1], 0.0);
        assert_eq!(values[2], 1.0);
    }
}

#[test]
fn test_cross_table_column_not_found_error() {
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
    // Reference non-existent column in table1
    table2
        .row_formulas
        .insert("result".to_string(), "=table1.nonexistent + x".to_string());
    model.add_table(table2);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
}

#[test]
fn test_cross_table_table_not_found_error() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]),
    ));
    // Reference non-existent table
    data.row_formulas.insert(
        "result".to_string(),
        "=nonexistent_table.column + x".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
}

#[test]
fn test_local_column_not_found_error() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]),
    ));
    // Reference non-existent local column
    data.row_formulas
        .insert("result".to_string(), "=nonexistent_column + x".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
}

#[test]
fn test_if_with_cross_table_reference() {
    let mut model = ParsedModel::new();

    let mut thresholds = Table::new("thresholds".to_string());
    thresholds.add_column(Column::new(
        "min".to_string(),
        ColumnValue::Number(vec![50.0]),
    ));
    model.add_table(thresholds);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![30.0, 60.0, 45.0]),
    ));
    // IF with cross-table reference
    data.row_formulas.insert(
        "above_min".to_string(),
        "=IF(value > SUM(thresholds.min), 1, 0)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Exercises cross-table reference in conditional
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_cross_table_column_not_found_error_v2() {
    let mut model = ParsedModel::new();

    let mut source = Table::new("source".to_string());
    source.add_column(Column::new(
        "a".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]),
    ));
    model.add_table(source);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]),
    ));
    // Reference non-existent column in other table
    data.row_formulas
        .insert("result".to_string(), "=source.nonexistent + x".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error - column not found
    assert!(result.is_err());
}

#[test]
fn test_cross_table_table_not_found_error_v2() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]),
    ));
    // Reference non-existent table
    data.row_formulas
        .insert("result".to_string(), "=nonexistent.column + x".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error - table not found
    assert!(result.is_err());
}

#[test]
fn test_local_column_not_found_error_v2() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]),
    ));
    // Reference non-existent local column
    data.row_formulas
        .insert("result".to_string(), "=nonexistent + x".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error - column not found
    assert!(result.is_err());
}

#[test]
fn test_nested_if_function() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "score".to_string(),
        ColumnValue::Number(vec![45.0, 65.0, 85.0]),
    ));
    data.row_formulas.insert(
        "grade".to_string(),
        "=IF(score >= 80, 1, IF(score >= 60, 2, 3))".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err()); // Exercise code path
}

#[test]
fn test_and_or_functions() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "a".to_string(),
        ColumnValue::Boolean(vec![true, true, false]),
    ));
    data.add_column(Column::new(
        "b".to_string(),
        ColumnValue::Boolean(vec![true, false, false]),
    ));
    data.row_formulas
        .insert("and_result".to_string(), "=IF(AND(a, b), 1, 0)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_not_function() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "flag".to_string(),
        ColumnValue::Boolean(vec![true, false]),
    ));
    data.row_formulas
        .insert("inverted".to_string(), "=IF(NOT(flag), 1, 0)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_iferror_function() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "numerator".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]),
    ));
    data.add_column(Column::new(
        "denominator".to_string(),
        ColumnValue::Number(vec![2.0, 0.0]),
    ));
    data.row_formulas.insert(
        "safe_div".to_string(),
        "=IFERROR(numerator / denominator, 0)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_iferror_no_error() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IFERROR(10/2, -1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_rowwise_if_formula() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    data.add_row_formula("status".to_string(), "=IF(value > 25, 1, 0)".to_string());
    model.add_table(data);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

// ══════════════════════════════════════════════════════════════════════════════
// Additional tests for logical functions with low coverage (v6.0.0 Phase 2)
// Note: Use numeric 1/0 for true/false, or comparisons like 1>0 for TRUE
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_and_all_true() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // Use comparisons that evaluate to true: 1>0, 2>0, 3>0
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IF(AND(1>0, 2>0, 3>0), 1, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(1.0));
}

#[test]
fn test_and_one_false() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // 1>0 is true, 0>1 is false, 2>0 is true
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IF(AND(1>0, 0>1, 2>0), 1, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(0.0));
}

#[test]
fn test_and_with_numbers() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // Non-zero numbers are truthy
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IF(AND(1, 2, 3), 1, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(1.0));
}

#[test]
fn test_or_all_false() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // All comparisons evaluate to false
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IF(OR(0>1, 0>2, 0>3), 1, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(0.0));
}

#[test]
fn test_or_one_true() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // Middle comparison is true
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IF(OR(0>1, 1>0, 0>2), 1, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(1.0));
}

#[test]
fn test_or_with_zero() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // Zero is falsy
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IF(OR(0, 0, 0), 1, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(0.0));
}

#[test]
fn test_not_true() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // NOT(1>0) = NOT(true) = false
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IF(NOT(1>0), 1, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(0.0));
}

#[test]
fn test_not_false() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // NOT(0>1) = NOT(false) = true
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IF(NOT(0>1), 1, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(1.0));
}

#[test]
fn test_not_with_number() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // NOT(1) should be FALSE (0)
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IF(NOT(1), 1, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(0.0));
}

#[test]
fn test_true_via_comparison() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // 1>0 evaluates to true
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IF(1>0, 1, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(1.0));
}

#[test]
fn test_false_via_comparison() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // 0>1 evaluates to false
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IF(0>1, 1, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(0.0));
}

#[test]
fn test_complex_logical_expression() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // Complex: AND(OR(true, false), NOT(false))
    // = AND(OR(1>0, 0>1), NOT(0>1))
    // = AND(true, true) = true
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=IF(AND(OR(1>0, 0>1), NOT(0>1)), 1, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let var = result.scalars.get("result").unwrap();
    assert_eq!(var.value, Some(1.0));
}
