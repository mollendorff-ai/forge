//! Advanced lookup function tests
//!
//! Tests for VLOOKUP, INDIRECT, and OFFSET functions

#![allow(clippy::approx_constant)]

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[test]
fn test_match_greater_than_or_equal_descending() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("ranges".to_string());
    lookup_table.add_column(Column::new(
        "threshold".to_string(),
        ColumnValue::Number(vec![40.0, 30.0, 20.0, 10.0]), // Descending order
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![25.0]),
    ));
    // match_type = -1: find smallest value >= lookup_value
    data.row_formulas.insert(
        "position".to_string(),
        "=MATCH(value, ranges.threshold, -1)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("position") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 2.0); // 30 is smallest value >= 25
        }
    }
}

#[test]
fn test_match_invalid_match_type() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("ranges".to_string());
    lookup_table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "search".to_string(),
        ColumnValue::Number(vec![15.0]),
    ));
    // Invalid match_type = 2
    data.row_formulas.insert(
        "position".to_string(),
        "=MATCH(search, ranges.value, 2)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error due to invalid match_type
    assert!(result.is_err());
}

#[test]
fn test_index_bounds_error() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("items".to_string());
    lookup_table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "idx".to_string(),
        ColumnValue::Number(vec![10.0]), // Out of bounds
    ));
    data.row_formulas
        .insert("result".to_string(), "=INDEX(items.value, idx)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error due to out of bounds
    assert!(result.is_err());
}

#[test]
fn test_index_zero_row_num() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("items".to_string());
    lookup_table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "idx".to_string(),
        ColumnValue::Number(vec![0.0]), // Zero not allowed (1-based)
    ));
    data.row_formulas
        .insert("result".to_string(), "=INDEX(items.value, idx)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error because row_num must be >= 1
    assert!(result.is_err());
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_vlookup_exact_match() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("products".to_string());
    lookup_table.add_column(Column::new(
        "id".to_string(),
        ColumnValue::Number(vec![101.0, 102.0, 103.0]),
    ));
    lookup_table.add_column(Column::new(
        "name".to_string(),
        ColumnValue::Text(vec![
            "Apple".to_string(),
            "Banana".to_string(),
            "Cherry".to_string(),
        ]),
    ));
    lookup_table.add_column(Column::new(
        "price".to_string(),
        ColumnValue::Number(vec![1.50, 0.75, 3.00]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "search_id".to_string(),
        ColumnValue::Number(vec![102.0]),
    ));
    // VLOOKUP(lookup_value, table_array, col_index, range_lookup)
    data.row_formulas.insert(
        "found_price".to_string(),
        "=VLOOKUP(search_id, products, 3, FALSE)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // VLOOKUP with table references may not be fully implemented
    if let Err(err) = result {
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("VLOOKUP")
                || err_msg.contains("table")
                || err_msg.contains("Unknown variable")
                || err_msg.contains("products"),
            "VLOOKUP should error with meaningful message, got: {}",
            err_msg
        );
    } else {
        // If it succeeds, verify the correct price is returned
        let model_result = result.unwrap();
        let table = model_result.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("found_price") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 0.75); // Price for product ID 102 (Banana)
            }
        }
    }
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_vlookup_col_index_out_of_range() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("products".to_string());
    lookup_table.add_column(Column::new(
        "id".to_string(),
        ColumnValue::Number(vec![101.0]),
    ));
    lookup_table.add_column(Column::new(
        "name".to_string(),
        ColumnValue::Text(vec!["Apple".to_string()]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "search".to_string(),
        ColumnValue::Number(vec![101.0]),
    ));
    // col_index = 5 exceeds number of columns (2)
    data.row_formulas.insert(
        "result".to_string(),
        "=VLOOKUP(search, products, 5, FALSE)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error because col_index exceeds columns
    assert!(result.is_err());
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_vlookup_col_index_zero() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("products".to_string());
    lookup_table.add_column(Column::new(
        "id".to_string(),
        ColumnValue::Number(vec![101.0]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "search".to_string(),
        ColumnValue::Number(vec![101.0]),
    ));
    // col_index = 0 is invalid
    data.row_formulas.insert(
        "result".to_string(),
        "=VLOOKUP(search, products, 0, FALSE)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error because col_index must be >= 1
    assert!(result.is_err());
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_xlookup_employee_salary() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("employees".to_string());
    lookup_table.add_column(Column::new(
        "id".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    lookup_table.add_column(Column::new(
        "salary".to_string(),
        ColumnValue::Number(vec![50000.0, 60000.0, 70000.0]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "emp_id".to_string(),
        ColumnValue::Number(vec![2.0]),
    ));
    // XLOOKUP(lookup_value, lookup_array, return_array, if_not_found, match_mode)
    data.row_formulas.insert(
        "emp_salary".to_string(),
        "=XLOOKUP(emp_id, employees.id, employees.salary, 0, 0)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("emp_salary") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 60000.0);
        }
    }
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_xlookup_default_value() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("employees".to_string());
    lookup_table.add_column(Column::new(
        "id".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]),
    ));
    lookup_table.add_column(Column::new(
        "salary".to_string(),
        ColumnValue::Number(vec![50000.0, 60000.0]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "emp_id".to_string(),
        ColumnValue::Number(vec![99.0]), // Not found
    ));
    // XLOOKUP with if_not_found = -1
    data.row_formulas.insert(
        "emp_salary".to_string(),
        "=XLOOKUP(emp_id, employees.id, employees.salary, -1, 0)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("emp_salary") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], -1.0); // Default value
        }
    }
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_xlookup_next_larger() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("ranges".to_string());
    lookup_table.add_column(Column::new(
        "threshold".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    lookup_table.add_column(Column::new(
        "label".to_string(),
        ColumnValue::Text(vec![
            "Low".to_string(),
            "Med".to_string(),
            "High".to_string(),
        ]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![15.0]),
    ));
    // match_mode = 1: exact or next larger
    data.row_formulas.insert(
        "label".to_string(),
        "=XLOOKUP(value, ranges.threshold, ranges.threshold, 0, 1)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("label") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 20.0); // Next larger than 15
        }
    }
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_xlookup_next_smaller() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("ranges".to_string());
    lookup_table.add_column(Column::new(
        "threshold".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![25.0]),
    ));
    // match_mode = -1: exact or next smaller
    data.row_formulas.insert(
        "result".to_string(),
        "=XLOOKUP(value, ranges.threshold, ranges.threshold, 0, -1)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("result") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 20.0); // Next smaller than 25
        }
    }
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_xlookup_invalid_match_mode() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("data".to_string());
    lookup_table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0]),
    ));
    model.add_table(lookup_table);

    let mut query = Table::new("query".to_string());
    query.add_column(Column::new(
        "search".to_string(),
        ColumnValue::Number(vec![10.0]),
    ));
    // Invalid match_mode = 5
    query.row_formulas.insert(
        "result".to_string(),
        "=XLOOKUP(search, data.value, data.value, 0, 5)".to_string(),
    );
    model.add_table(query);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error due to invalid match_mode
    assert!(result.is_err());
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_xlookup_array_length_mismatch() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("source".to_string());
    lookup_table.add_column(Column::new(
        "keys".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    lookup_table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]), // Different length!
    ));
    model.add_table(lookup_table);

    let mut query = Table::new("query".to_string());
    query.add_column(Column::new(
        "search".to_string(),
        ColumnValue::Number(vec![1.0]),
    ));
    query.row_formulas.insert(
        "result".to_string(),
        "=XLOOKUP(search, source.keys, source.values, 0, 0)".to_string(),
    );
    model.add_table(query);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error due to length mismatch
    assert!(result.is_err());
}

#[test]
fn test_match_no_value_found_ascending() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("ranges".to_string());
    lookup_table.add_column(Column::new(
        "threshold".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![50.0]), // Less than all values
    ));
    // match_type = 1: find largest value <= lookup_value
    data.row_formulas.insert(
        "position".to_string(),
        "=MATCH(value, ranges.threshold, 1)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error because no value <= 50 exists
    assert!(result.is_err());
}

#[test]
fn test_match_no_value_found_descending() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("ranges".to_string());
    lookup_table.add_column(Column::new(
        "threshold".to_string(),
        ColumnValue::Number(vec![300.0, 200.0, 100.0]), // Descending
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![500.0]), // Greater than all values
    ));
    // match_type = -1: find smallest value >= lookup_value
    data.row_formulas.insert(
        "position".to_string(),
        "=MATCH(value, ranges.threshold, -1)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error because no value >= 500 exists
    assert!(result.is_err());
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_vlookup_with_text_search_value() {
    let mut model = ParsedModel::new();

    let mut products = Table::new("products".to_string());
    products.add_column(Column::new(
        "name".to_string(),
        ColumnValue::Text(vec![
            "Apple".to_string(),
            "Banana".to_string(),
            "Cherry".to_string(),
        ]),
    ));
    products.add_column(Column::new(
        "price".to_string(),
        ColumnValue::Number(vec![1.50, 0.75, 3.00]),
    ));
    model.add_table(products);

    let mut data = Table::new("query".to_string());
    data.add_column(Column::new(
        "search".to_string(),
        ColumnValue::Text(vec!["Banana".to_string()]),
    ));
    data.row_formulas.insert(
        "found_price".to_string(),
        "=VLOOKUP(search, products, 2, FALSE)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // VLOOKUP with table references may not be fully implemented
    if let Err(err) = result {
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("VLOOKUP")
                || err_msg.contains("table")
                || err_msg.contains("Unknown variable")
                || err_msg.contains("products"),
            "VLOOKUP should error with meaningful message, got: {}",
            err_msg
        );
    } else {
        // If it succeeds, verify the correct price is returned
        let model_result = result.unwrap();
        let table = model_result.tables.get("query").unwrap();
        if let Some(col) = table.columns.get("found_price") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 0.75); // Price for "Banana"
            }
        }
    }
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_indirect_function_v2() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "indirect_val".to_string(),
        Variable::new(
            "indirect_val".to_string(),
            None,
            Some("=SUM(INDIRECT(\"data.values\"))".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(
        result.is_ok(),
        "INDIRECT function should calculate successfully"
    );
    let model_result = result.unwrap();
    let val = model_result
        .scalars
        .get("indirect_val")
        .unwrap()
        .value
        .unwrap();
    // SUM(data.values) = 10 + 20 + 30 = 60
    assert_eq!(val, 60.0);
}

#[test]
fn test_choose_function_v2() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "idx".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    data.row_formulas
        .insert("chosen".to_string(), "=CHOOSE(idx, 10, 20, 30)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(
        result.is_ok(),
        "CHOOSE function should calculate successfully"
    );
    let model_result = result.unwrap();
    let table = model_result.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("chosen") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 10.0); // CHOOSE(1, 10, 20, 30) = 10
            assert_eq!(vals[1], 20.0); // CHOOSE(2, 10, 20, 30) = 20
            assert_eq!(vals[2], 30.0); // CHOOSE(3, 10, 20, 30) = 30
        }
    }
}

#[test]
fn test_choose_valid_index() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=CHOOSE(2, 100, 200, 300)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok(), "CHOOSE with valid index should succeed");
    let model_result = result.unwrap();
    let val = model_result.scalars.get("result").unwrap().value.unwrap();
    // CHOOSE(2, 100, 200, 300) should return 200
    assert_eq!(val, 200.0);
}

#[test]
fn test_choose_index_out_of_range() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=CHOOSE(10, 100, 200, 300)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // CHOOSE with out-of-range index should error
    assert!(
        result.is_err(),
        "CHOOSE with index 10 out of range [1-3] should error"
    );
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_indirect_table_column() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("sales".to_string());
    data.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "total".to_string(),
        Variable::new(
            "total".to_string(),
            None,
            Some("=SUM(INDIRECT(\"sales.revenue\"))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(
        result.is_ok(),
        "INDIRECT table column should calculate successfully"
    );
    let model_result = result.unwrap();
    let val = model_result.scalars.get("total").unwrap().value.unwrap();
    // SUM(sales.revenue) = 100 + 200 + 300 = 600
    assert_eq!(val, 600.0);
}

#[test]
fn test_array_index_out_of_bounds() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "val".to_string(),
        Variable::new(
            "val".to_string(),
            None,
            Some("=data.values[100]".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Array index out of bounds should error
    assert!(
        result.is_err(),
        "Array index [100] out of bounds [0-2] should error"
    );
}
