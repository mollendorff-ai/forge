//! Conditional Function Edge Case Tests
//!
//! Tests for SUMIF, COUNTIF, AVERAGEIF, SUMIFS, COUNTIFS, MAXIFS, MINIFS edge cases

#![allow(clippy::approx_constant)]
#![allow(clippy::float_cmp)]

use crate::core::array_calculator::ArrayCalculator;
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

// ============================================================================
// SUMIF Tests
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_sumif_greater() {
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
            Some("=SUMIF(data.values, \">3\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(9.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_sumif_less() {
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
            Some("=SUMIF(data.values, \"<3\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_sumif_equal() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 2.0, 1.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SUMIF(data.values, \"=2\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(4.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_sumif_not_equal() {
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
            Some("=SUMIF(data.values, \"<>3\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(12.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_sumif_all_match() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SUMIF(data.values, \">0\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(6.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_sumif_none_match() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SUMIF(data.values, \">10\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

// ============================================================================
// COUNTIF Tests
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_countif_greater() {
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
            Some("=COUNTIF(data.values, \">3\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_countif_less() {
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
            Some("=COUNTIF(data.values, \"<3\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_countif_equal() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 2.0, 1.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=COUNTIF(data.values, \"=2\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_countif_all_match() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=COUNTIF(data.values, \">0\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_countif_none_match() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=COUNTIF(data.values, \">10\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

// ============================================================================
// AVERAGEIF Tests
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_averageif_greater() {
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
            Some("=AVERAGEIF(data.values, \">3\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(4.5));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_averageif_less() {
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
            Some("=AVERAGEIF(data.values, \"<3\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1.5));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_averageif_all_match() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 4.0, 6.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=AVERAGEIF(data.values, \">0\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(4.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_averageif_equal() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 10.0, 30.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=AVERAGEIF(data.values, \"=10\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
}

// ============================================================================
// SUMIFS Tests
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_sumifs_two_criteria() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    table.add_column(Column::new(
        "cat1".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 1.0, 2.0]),
    ));
    table.add_column(Column::new(
        "cat2".to_string(),
        ColumnValue::Number(vec![5.0, 5.0, 6.0, 6.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SUMIFS(data.values, data.cat1, \"=1\", data.cat2, \">5\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(30.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_sumifs_all_match() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    table.add_column(Column::new(
        "cat".to_string(),
        ColumnValue::Number(vec![1.0, 1.0, 1.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SUMIFS(data.values, data.cat, \"=1\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(6.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_sumifs_none_match() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    table.add_column(Column::new(
        "cat".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SUMIFS(data.values, data.cat, \">5\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

// ============================================================================
// COUNTIFS Tests
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_countifs_two_criteria() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "cat1".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 1.0, 2.0]),
    ));
    table.add_column(Column::new(
        "cat2".to_string(),
        ColumnValue::Number(vec![5.0, 6.0, 5.0, 6.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=COUNTIFS(data.cat1, \"=1\", data.cat2, \">5\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    // cat1=1 at idx 0,2; cat2>5 at idx 1,3. Intersection: none that match both
    // Actually wait - cat1=1 at positions 0,2 and cat2>5 at positions 1,3
    // So intersection is empty. Let me fix this...
    // Actually let me fix the test data: cat1=[1,2,1,2], cat2=[5,6,5,6]
    // cat1=1: indices 0,2
    // cat2>5: indices 1,3
    // No overlap! So count=0, not 1
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_countifs_all_match() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "cat".to_string(),
        ColumnValue::Number(vec![1.0, 1.0, 1.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=COUNTIFS(data.cat, \"=1\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_countifs_none_match() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "cat".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=COUNTIFS(data.cat, \">5\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
}

// ============================================================================
// MAXIFS/MINIFS Tests
// ============================================================================

#[cfg(not(feature = "demo"))]
#[test]
fn test_maxifs_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    table.add_column(Column::new(
        "cat".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 1.0, 2.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=MAXIFS(data.values, data.cat, \"=1\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(30.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_minifs_basic() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    table.add_column(Column::new(
        "cat".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 1.0, 2.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=MINIFS(data.values, data.cat, \"=1\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_maxifs_all() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    table.add_column(Column::new(
        "cat".to_string(),
        ColumnValue::Number(vec![1.0, 1.0, 1.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=MAXIFS(data.values, data.cat, \"=1\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_minifs_all() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    table.add_column(Column::new(
        "cat".to_string(),
        ColumnValue::Number(vec![1.0, 1.0, 1.0]),
    ));
    model.add_table(table);
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=MINIFS(data.values, data.cat, \"=1\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
}
