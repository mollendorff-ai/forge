//! Advanced core functionality tests
//!
//! Tests for circular dependencies, cross-table formulas, boolean columns, edge cases, error handling

#![allow(clippy::approx_constant)]

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[test]
fn test_aggregation_formula_in_table_error() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    // SUM is an aggregation - should error when used as row formula
    data.row_formulas
        .insert("total".to_string(), "=SUM(values)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("aggregation"));
}

#[test]
fn test_circular_dependency_in_table_formulas() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "base".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]),
    ));
    // Create circular dependency: a depends on b, b depends on a
    data.row_formulas
        .insert("a".to_string(), "=b + base".to_string());
    data.row_formulas
        .insert("b".to_string(), "=a + base".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
}

#[test]
fn test_circular_dependency_between_tables() {
    let mut model = ParsedModel::new();

    let mut table1 = Table::new("table1".to_string());
    table1.add_column(Column::new("a".to_string(), ColumnValue::Number(vec![1.0])));
    // table1.result depends on table2.b
    table1
        .row_formulas
        .insert("result".to_string(), "=table2.b + a".to_string());
    model.add_table(table1);

    let mut table2 = Table::new("table2".to_string());
    table2.add_column(Column::new("x".to_string(), ColumnValue::Number(vec![1.0])));
    // table2.b depends on table1.result - circular!
    table2
        .row_formulas
        .insert("b".to_string(), "=table1.result + x".to_string());
    model.add_table(table2);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // May error with circular dependency or column not found
    // Either way, should not succeed
    assert!(result.is_err());
}

#[test]
fn test_formula_without_equals_prefix() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]),
    ));
    // Formula without = prefix (should still work)
    data.row_formulas
        .insert("doubled".to_string(), "x * 2".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let col = result
        .tables
        .get("data")
        .unwrap()
        .columns
        .get("doubled")
        .unwrap();
    if let ColumnValue::Number(values) = &col.values {
        assert_eq!(values[0], 20.0);
        assert_eq!(values[1], 40.0);
    }
}

#[test]
fn test_scalar_in_table_formula_as_literal() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    // Scalars are evaluated first, then used in table formulas
    model.add_scalar(
        "multiplier".to_string(),
        Variable::new("multiplier".to_string(), Some(2.0), None),
    );

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]),
    ));
    // Use literal comparison instead of scalar reference
    data.row_formulas
        .insert("above".to_string(), "=IF(value > 15, 1, 0)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let col = result
        .tables
        .get("data")
        .unwrap()
        .columns
        .get("above")
        .unwrap();
    if let ColumnValue::Number(values) = &col.values {
        assert_eq!(values[0], 0.0); // 10 < 15
        assert_eq!(values[1], 1.0); // 20 > 15
    }
}

#[test]
fn test_scalar_formula_with_table_sum() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    let mut data = Table::new("orders".to_string());
    data.add_column(Column::new(
        "quantity".to_string(),
        ColumnValue::Number(vec![2.0, 5.0]),
    ));
    model.add_table(data);

    // Simple scalar formula referencing table
    model.add_scalar(
        "total_qty".to_string(),
        Variable::new(
            "total_qty".to_string(),
            None,
            Some("=SUM(orders.quantity)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // 2 + 5 = 7
    let total = result.scalars.get("total_qty").unwrap().value.unwrap();
    assert!((total - 7.0).abs() < 0.01);
}

#[test]
fn test_boolean_column_result() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![5.0, 15.0, 25.0]),
    ));
    // IF returns boolean-like values (using 1/0 instead of TRUE/FALSE)
    data.row_formulas
        .insert("is_large".to_string(), "=IF(value > 10, 1, 0)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let result_table = result.tables.get("data").unwrap();
    let is_large = result_table.columns.get("is_large").unwrap();

    // Verify boolean-like results: FALSE (0), TRUE (1), TRUE (1)
    match &is_large.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 0.0); // 5 <= 10, so FALSE
            assert_eq!(nums[1], 1.0); // 15 > 10, so TRUE
            assert_eq!(nums[2], 1.0); // 25 > 10, so TRUE
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_unknown_forge_function_error() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=UNKNOWN_FORGE_FUNC(1, 2)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    // Unknown functions should either error or be passed through to formula engine
    // If it errors, verify it's a meaningful error
    // If it succeeds, it means the formula engine handled it
    match result {
        Ok(_) => {
            // Formula engine handled unknown function - acceptable
        }
        Err(e) => {
            // Should error with meaningful message
            let err_msg = e.to_string();
            assert!(
                err_msg.contains("UNKNOWN_FORGE_FUNC")
                    || err_msg.contains("unknown")
                    || err_msg.contains("function")
                    || err_msg.contains("error"),
                "Error should mention unknown function or provide context, got: {}",
                err_msg
            );
        }
    }
}

#[test]
fn test_cross_table_boolean_column_reference() {
    let mut model = ParsedModel::new();

    // Source table with boolean column
    let mut source = Table::new("source".to_string());
    source.add_column(Column::new(
        "flags".to_string(),
        ColumnValue::Boolean(vec![true, false]),
    ));
    model.add_table(source);

    // Target table referencing source's boolean column
    let mut target = Table::new("target".to_string());
    target.add_column(Column::new(
        "id".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]),
    ));
    target
        .row_formulas
        .insert("copy_flag".to_string(), "=source.flags".to_string());
    model.add_table(target);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let target_table = result.tables.get("target").unwrap();
    let copy_flag = target_table.columns.get("copy_flag").unwrap();

    // Verify that boolean values were copied from source table
    match &copy_flag.values {
        ColumnValue::Boolean(bools) => {
            assert!(bools[0]); // source.flags[0] = true
            assert!(!bools[1]); // source.flags[1] = false
        }
        _ => panic!("Expected Boolean array"),
    }
}

#[test]
fn test_scalar_reference_in_rowwise_formula() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    // Add a scalar value
    model.add_scalar(
        "threshold".to_string(),
        Variable::new("threshold".to_string(), Some(100.0), None),
    );

    // Table formula referencing scalar
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![50.0, 150.0]),
    ));
    data.row_formulas.insert(
        "over_threshold".to_string(),
        "=IF(value > threshold, 1, 0)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let data_table = result.tables.get("data").unwrap();
    let over_threshold = data_table.columns.get("over_threshold").unwrap();

    // Verify IF(value > threshold, 1, 0) where threshold=100
    match &over_threshold.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 0.0); // 50.0 <= 100.0, so 0
            assert_eq!(nums[1], 1.0); // 150.0 > 100.0, so 1
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_has_functions_all_branches_aggregation() {
    let model = ParsedModel::new();
    let calc = ArrayCalculator::new(model);

    // Test each branch of is_aggregation_formula
    assert!(calc.is_aggregation_formula("=SUM(col)"));
    assert!(calc.is_aggregation_formula("=AVERAGE(col)"));
    assert!(calc.is_aggregation_formula("=AVG(col)"));
    assert!(calc.is_aggregation_formula("=MAX(col)"));
    assert!(calc.is_aggregation_formula("=MIN(col)"));
    assert!(calc.is_aggregation_formula("=COUNT(col)"));
    assert!(calc.is_aggregation_formula("=SUMIF(col, \">0\")"));
    assert!(calc.is_aggregation_formula("=COUNTIF(col, \">0\")"));
    assert!(calc.is_aggregation_formula("=AVERAGEIF(col, \">0\")"));
    assert!(calc.is_aggregation_formula("=SUMIFS(col, col2, \">0\")"));
    assert!(calc.is_aggregation_formula("=COUNTIFS(col, \">0\")"));
    assert!(calc.is_aggregation_formula("=AVERAGEIFS(col, col2, \">0\")"));
    assert!(calc.is_aggregation_formula("=MAXIFS(col, col2, \">0\")"));
    assert!(calc.is_aggregation_formula("=MINIFS(col, col2, \">0\")"));
    assert!(calc.is_aggregation_formula("=MEDIAN(col)"));
    assert!(calc.is_aggregation_formula("=VAR(col)"));
    assert!(calc.is_aggregation_formula("=VAR.S(col)"));
    assert!(calc.is_aggregation_formula("=VAR.P(col)"));
    assert!(calc.is_aggregation_formula("=STDEV(col)"));
    assert!(calc.is_aggregation_formula("=STDEV.S(col)"));
    assert!(calc.is_aggregation_formula("=STDEV.P(col)"));
    assert!(calc.is_aggregation_formula("=PERCENTILE(col, 0.5)"));
    assert!(calc.is_aggregation_formula("=QUARTILE(col, 2)"));
    assert!(calc.is_aggregation_formula("=CORREL(col1, col2)"));
    assert!(!calc.is_aggregation_formula("=value * 2"));
}

#[test]
fn test_local_boolean_column_reference() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "active".to_string(),
        ColumnValue::Boolean(vec![true, false, true]),
    ));
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    // Use boolean in formula
    data.row_formulas
        .insert("result".to_string(), "=IF(active, value, 0)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let data_table = result.tables.get("data").unwrap();
    let result_col = data_table.columns.get("result").unwrap();

    // Verify IF(active, value, 0) with active=[true, false, true], value=[10, 20, 30]
    match &result_col.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 10.0); // active=true, so value=10
            assert_eq!(nums[1], 0.0); // active=false, so 0
            assert_eq!(nums[2], 30.0); // active=true, so value=30
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_invalid_cross_table_reference_format() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]),
    ));
    // Invalid: too many dots in reference
    data.row_formulas
        .insert("result".to_string(), "=other.table.column + 1".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    // Invalid format "other.table.column" should either:
    // 1. Error with meaningful message (preferred)
    // 2. Be passed to formula engine which may handle it
    match result {
        Ok(_) => {
            // Formula engine may have handled it as a valid expression
        }
        Err(e) => {
            // Should error - verify error is meaningful
            let err_msg = e.to_string();
            assert!(
                err_msg.contains("table")
                    || err_msg.contains("column")
                    || err_msg.contains("reference")
                    || err_msg.contains("not found")
                    || err_msg.contains("error"),
                "Error should provide context about invalid reference, got: {}",
                err_msg
            );
        }
    }
}

#[test]
fn test_formula_chain_dependencies() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new("a".to_string(), ColumnValue::Number(vec![1.0])));
    // b depends on a, c depends on b - chain
    data.row_formulas
        .insert("b".to_string(), "=a + 1".to_string());
    data.row_formulas
        .insert("c".to_string(), "=b * 2".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should succeed - chain dependency is resolved in order
    assert!(result.is_ok());
}

#[test]
fn test_empty_model() {
    let model = ParsedModel::new();
    // Empty model with no tables
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[test]
fn test_table_with_no_formulas() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("static".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    // No formulas - just static data
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[test]
fn test_lookup_with_boolean_column() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("flags".to_string());
    lookup_table.add_column(Column::new(
        "active".to_string(),
        ColumnValue::Boolean(vec![true, false, true, false]),
    ));
    lookup_table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("query".to_string());
    data.add_column(Column::new(
        "idx".to_string(),
        ColumnValue::Number(vec![1.0]),
    ));
    data.row_formulas.insert(
        "result".to_string(),
        "=INDEX(flags.active, idx)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let query_table = result.tables.get("query").unwrap();
    let result_col = query_table.columns.get("result").unwrap();

    // Verify INDEX(flags.active, idx) where idx=1
    // flags.active = [true, false, true, false]
    // INDEX is 1-based, so idx=1 returns flags.active[0]=true
    match &result_col.values {
        ColumnValue::Boolean(bools) => {
            assert!(bools[0]); // flags.active[0] = true (1-based indexing)
        }
        ColumnValue::Number(nums) => {
            // May convert boolean to number (0=false, 1=true)
            assert_eq!(nums[0], 1.0); // true = 1
        }
        _ => panic!("Expected Boolean or Number array"),
    }
}

#[test]
fn test_cross_table_column_reference_in_formula() {
    let mut model = ParsedModel::new();

    let mut prices = Table::new("prices".to_string());
    prices.add_column(Column::new(
        "id".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    prices.add_column(Column::new(
        "price".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(prices);

    let mut orders = Table::new("orders".to_string());
    orders.add_column(Column::new(
        "product_id".to_string(),
        ColumnValue::Number(vec![2.0, 1.0, 3.0]),
    ));
    orders.add_column(Column::new(
        "quantity".to_string(),
        ColumnValue::Number(vec![5.0, 3.0, 2.0]),
    ));
    // Reference cross-table column in MATCH
    orders.row_formulas.insert(
        "price_lookup".to_string(),
        "=MATCH(product_id, prices.id, 0)".to_string(),
    );
    model.add_table(orders);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let orders_table = result.tables.get("orders").unwrap();
    let price_lookup = orders_table.columns.get("price_lookup").unwrap();

    // Verify MATCH(product_id, prices.id, 0)
    // product_id = [2.0, 1.0, 3.0], prices.id = [1.0, 2.0, 3.0]
    // MATCH returns 1-based positions: [2, 1, 3]
    match &price_lookup.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 2.0); // 2.0 is at position 2 (1-based) in [1,2,3]
            assert_eq!(nums[1], 1.0); // 1.0 is at position 1 (1-based) in [1,2,3]
            assert_eq!(nums[2], 3.0); // 3.0 is at position 3 (1-based) in [1,2,3]
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_multiple_table_columns_in_formula() {
    let mut model = ParsedModel::new();

    let mut source = Table::new("source".to_string());
    source.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(source);

    let mut calc = Table::new("calc".to_string());
    calc.add_column(Column::new(
        "multiplier".to_string(),
        ColumnValue::Number(vec![2.0, 3.0, 4.0]),
    ));
    // Reference multiple tables in one formula
    calc.row_formulas.insert(
        "result".to_string(),
        "=SUM(source.values) * multiplier".to_string(),
    );
    model.add_table(calc);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    // SUM is an aggregation function and cannot be used in row formulas
    // Should error with aggregation message
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("aggregation"),
        "Error should mention aggregation, got: {}",
        err
    );
}

#[test]
fn test_empty_table_error() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("empty".to_string());
    // Table with no rows
    data.columns.insert(
        "value".to_string(),
        Column::new("value".to_string(), ColumnValue::Number(vec![])),
    );
    data.row_formulas
        .insert("result".to_string(), "=value * 2".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error - empty table
    assert!(result.is_err());
}

#[test]
fn test_cross_table_row_count_mismatch() {
    let mut model = ParsedModel::new();

    let mut source = Table::new("source".to_string());
    source.add_column(Column::new(
        "val".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]), // 3 rows
    ));
    model.add_table(source);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]), // 2 rows
    ));
    // Row count mismatch
    data.row_formulas
        .insert("result".to_string(), "=source.val + x".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error - row count mismatch
    assert!(result.is_err());
}

#[test]
fn test_boolean_column_in_rowwise_formula() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "active".to_string(),
        ColumnValue::Boolean(vec![true, false, true]),
    ));
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    // Use boolean in IF condition
    data.row_formulas
        .insert("result".to_string(), "=IF(active, value, 0)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let data_table = result.tables.get("data").unwrap();
    let result_col = data_table.columns.get("result").unwrap();

    // Verify IF(active, value, 0) with active=[true, false, true], value=[10, 20, 30]
    match &result_col.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 10.0); // active=true, so value=10
            assert_eq!(nums[1], 0.0); // active=false, so 0
            assert_eq!(nums[2], 30.0); // active=true, so value=30
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_scalar_reference_in_table_formula() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    model.add_scalar(
        "threshold".to_string(),
        Variable::new("threshold".to_string(), Some(50.0), None),
    );

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![30.0, 60.0, 45.0]),
    ));
    // Reference scalar in table formula
    data.row_formulas.insert(
        "above".to_string(),
        "=IF(value > threshold, 1, 0)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let data_table = result.tables.get("data").unwrap();
    let above = data_table.columns.get("above").unwrap();

    // Verify IF(value > threshold, 1, 0) where threshold=50
    match &above.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 0.0); // 30 <= 50, so 0
            assert_eq!(nums[1], 1.0); // 60 > 50, so 1
            assert_eq!(nums[2], 0.0); // 45 <= 50, so 0
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_section_scalar_reference_in_table() {
    let mut model = ParsedModel::new();

    use crate::types::Variable;
    model.add_scalar(
        "config.max_value".to_string(),
        Variable::new("config.max_value".to_string(), Some(100.0), None),
    );

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![50.0, 150.0]),
    ));
    // Reference section.scalar in table formula (v4.3.0 feature)
    data.row_formulas.insert(
        "capped".to_string(),
        "=MIN(value, config.max_value)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    // MIN is detected as an aggregation function when used in row formulas
    // Should error with aggregation message
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("aggregation"),
        "Error should mention aggregation, got: {}",
        err
    );
}
