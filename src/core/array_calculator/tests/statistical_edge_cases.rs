//! Statistical Function Edge Case Tests
//!
//! Tests for PERCENTILE, QUARTILE, VAR.S, VAR.P, STDEV.S, STDEV.P, LARGE, SMALL, CORREL

#![allow(clippy::approx_constant)]
#![allow(clippy::float_cmp)]

use crate::core::array_calculator::ArrayCalculator;
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

// ============================================================================
// PERCENTILE Tests
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_percentile_0() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=PERCENTILE(data.values, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_percentile_100() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=PERCENTILE(data.values, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_percentile_50() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=PERCENTILE(data.values, 0.5)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_percentile_25() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=PERCENTILE(data.values, 0.25)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_percentile_75() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=PERCENTILE(data.values, 0.75)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(4.0));
}

// ============================================================================
// QUARTILE Tests
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_quartile_0() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=QUARTILE(data.values, 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_quartile_1() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=QUARTILE(data.values, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_quartile_2() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=QUARTILE(data.values, 2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_quartile_4() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=QUARTILE(data.values, 4)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
}

// ============================================================================
// VAR.S / STDEV.S Tests (Sample Variance/StdDev)
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_var_s_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 6.0, 8.0, 10.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=VAR.S(data.values)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_var_s_identical() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![5.0, 5.0, 5.0, 5.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=VAR.S(data.values)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_stdev_s_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 6.0, 8.0, 10.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=STDEV.S(data.values)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let value = result.scalars.get("result").unwrap().value.unwrap();
    assert!((value - 3.162277660168379).abs() < 1e-10);
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_stdev_s_identical() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![5.0, 5.0, 5.0, 5.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=STDEV.S(data.values)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

// ============================================================================
// VAR.P / STDEV.P Tests (Population Variance/StdDev)
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_var_p_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 6.0, 8.0, 10.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=VAR.P(data.values)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(8.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_var_p_identical() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![5.0, 5.0, 5.0, 5.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=VAR.P(data.values)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_stdev_p_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 6.0, 8.0, 10.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=STDEV.P(data.values)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let value = result.scalars.get("result").unwrap().value.unwrap();
    assert!((value - 2.8284271247461903).abs() < 1e-10);
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_stdev_p_identical() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![5.0, 5.0, 5.0, 5.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=STDEV.P(data.values)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

// ============================================================================
// LARGE / SMALL Tests
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_large_1() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LARGE(data.values, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(9.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_large_3() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LARGE(data.values, 3)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_small_1() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SMALL(data.values, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_small_3() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SMALL(data.values, 3)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_large_equals_max() {
    // LARGE with k=1 should equal MAX
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LARGE(data.values, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_small_equals_min() {
    // SMALL with k=1 should equal MIN
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SMALL(data.values, 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
}

// ============================================================================
// CORREL Tests (Correlation)
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_correl_perfect_positive() {
    // Perfect positive correlation: y = 2x
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    table.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 6.0, 8.0, 10.0]),
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
    let result = calculator.calculate_all().expect("Should calculate");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - 1.0).abs() < 1e-10, "Expected ~1.0, got {val}");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_correl_perfect_negative() {
    // Perfect negative correlation: opposite direction
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    table.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![5.0, 4.0, 3.0, 2.0, 1.0]),
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
    let result = calculator.calculate_all().expect("Should calculate");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert!((val - (-1.0)).abs() < 1e-10, "Expected ~-1.0, got {val}");
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_correl_no_correlation() {
    // Near-zero correlation: uncorrelated data
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    table.add_column(Column::new(
        "y".to_string(),
        ColumnValue::Number(vec![3.0, 1.0, 5.0, 2.0, 4.0]),
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
    let result = calculator.calculate_all().expect("Should calculate");
    let value = result.scalars.get("result").unwrap().value.unwrap();
    // This won't be exactly 0, but should be close to 0
    assert!(value.abs() < 0.5);
}
