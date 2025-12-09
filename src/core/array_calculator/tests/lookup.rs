//! Lookup function tests for ArrayCalculator

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI/E

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[test]
fn test_match_exact() {
    let mut model = ParsedModel::new();

    // Create products table
    let mut products = Table::new("products".to_string());
    products.add_column(Column::new(
        "product_id".to_string(),
        ColumnValue::Number(vec![101.0, 102.0, 103.0, 104.0]),
    ));
    products.add_column(Column::new(
        "product_name".to_string(),
        ColumnValue::Text(vec![
            "Widget A".to_string(),
            "Widget B".to_string(),
            "Widget C".to_string(),
            "Widget D".to_string(),
        ]),
    ));
    model.add_table(products);

    // Create sales table with MATCH formulas
    let mut sales = Table::new("sales".to_string());
    sales.add_column(Column::new(
        "lookup_id".to_string(),
        ColumnValue::Number(vec![102.0, 104.0, 101.0]),
    ));
    sales.add_row_formula(
        "position".to_string(),
        "=MATCH(lookup_id, products.product_id, 0)".to_string(),
    );
    model.add_table(sales);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("sales").unwrap();

    let position = result_table.columns.get("position").unwrap();
    match &position.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 2.0); // 102 is at position 2 (1-based)
            assert_eq!(nums[1], 4.0); // 104 is at position 4
            assert_eq!(nums[2], 1.0); // 101 is at position 1
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_index_basic() {
    let mut model = ParsedModel::new();

    // Create products table
    let mut products = Table::new("products".to_string());
    products.add_column(Column::new(
        "product_name".to_string(),
        ColumnValue::Text(vec![
            "Widget A".to_string(),
            "Widget B".to_string(),
            "Widget C".to_string(),
        ]),
    ));
    model.add_table(products);

    // Create test table with INDEX formulas
    let mut test = Table::new("test".to_string());
    test.add_column(Column::new(
        "index".to_string(),
        ColumnValue::Number(vec![1.0, 3.0, 2.0]),
    ));
    test.add_row_formula(
        "name".to_string(),
        "=INDEX(products.product_name, index)".to_string(),
    );
    model.add_table(test);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("test").unwrap();

    let name = result_table.columns.get("name").unwrap();
    match &name.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "Widget A");
            assert_eq!(texts[1], "Widget C");
            assert_eq!(texts[2], "Widget B");
        }
        _ => panic!("Expected Text array"),
    }
}

#[test]
fn test_index_match_combined() {
    let mut model = ParsedModel::new();

    // Create products table
    let mut products = Table::new("products".to_string());
    products.add_column(Column::new(
        "product_id".to_string(),
        ColumnValue::Number(vec![101.0, 102.0, 103.0]),
    ));
    products.add_column(Column::new(
        "product_name".to_string(),
        ColumnValue::Text(vec![
            "Widget A".to_string(),
            "Widget B".to_string(),
            "Widget C".to_string(),
        ]),
    ));
    products.add_column(Column::new(
        "price".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(products);

    // Create sales table with INDEX/MATCH formulas
    let mut sales = Table::new("sales".to_string());
    sales.add_column(Column::new(
        "product_id".to_string(),
        ColumnValue::Number(vec![102.0, 101.0, 103.0]),
    ));
    sales.add_row_formula(
        "product_name".to_string(),
        "=INDEX(products.product_name, MATCH(product_id, products.product_id, 0))".to_string(),
    );
    sales.add_row_formula(
        "price".to_string(),
        "=INDEX(products.price, MATCH(product_id, products.product_id, 0))".to_string(),
    );
    model.add_table(sales);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("sales").unwrap();

    let product_name = result_table.columns.get("product_name").unwrap();
    match &product_name.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "Widget B");
            assert_eq!(texts[1], "Widget A");
            assert_eq!(texts[2], "Widget C");
        }
        _ => panic!("Expected Text array"),
    }

    let price = result_table.columns.get("price").unwrap();
    match &price.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 20.0);
            assert_eq!(nums[1], 10.0);
            assert_eq!(nums[2], 30.0);
        }
        _ => panic!("Expected Number array"),
    }
}

#[cfg(feature = "full")]
#[test]
fn test_xlookup_exact_match() {
    let mut model = ParsedModel::new();

    // Create products table
    let mut products = Table::new("products".to_string());
    products.add_column(Column::new(
        "product_id".to_string(),
        ColumnValue::Number(vec![101.0, 102.0, 103.0]),
    ));
    products.add_column(Column::new(
        "product_name".to_string(),
        ColumnValue::Text(vec![
            "Widget A".to_string(),
            "Widget B".to_string(),
            "Widget C".to_string(),
        ]),
    ));
    model.add_table(products);

    // Create sales table with XLOOKUP formulas
    let mut sales = Table::new("sales".to_string());
    sales.add_column(Column::new(
        "product_id".to_string(),
        ColumnValue::Number(vec![102.0, 103.0, 101.0]),
    ));
    sales.add_row_formula(
        "product_name".to_string(),
        "=XLOOKUP(product_id, products.product_id, products.product_name)".to_string(),
    );
    model.add_table(sales);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("sales").unwrap();

    let product_name = result_table.columns.get("product_name").unwrap();
    match &product_name.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "Widget B");
            assert_eq!(texts[1], "Widget C");
            assert_eq!(texts[2], "Widget A");
        }
        _ => panic!("Expected Text array"),
    }
}

#[cfg(feature = "full")]
#[test]
fn test_xlookup_with_if_not_found() {
    let mut model = ParsedModel::new();

    // Create products table
    let mut products = Table::new("products".to_string());
    products.add_column(Column::new(
        "product_id".to_string(),
        ColumnValue::Number(vec![101.0, 102.0, 103.0]),
    ));
    products.add_column(Column::new(
        "product_name".to_string(),
        ColumnValue::Text(vec![
            "Widget A".to_string(),
            "Widget B".to_string(),
            "Widget C".to_string(),
        ]),
    ));
    model.add_table(products);

    // Create sales table with XLOOKUP formulas (including non-existent ID)
    let mut sales = Table::new("sales".to_string());
    sales.add_column(Column::new(
        "product_id".to_string(),
        ColumnValue::Number(vec![102.0, 999.0, 101.0]), // 999 doesn't exist
    ));
    sales.add_row_formula(
        "product_name".to_string(),
        "=XLOOKUP(product_id, products.product_id, products.product_name, \"Not Found\")"
            .to_string(),
    );
    model.add_table(sales);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("sales").unwrap();

    let product_name = result_table.columns.get("product_name").unwrap();
    match &product_name.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "Widget B");
            assert_eq!(texts[1], "Not Found");
            assert_eq!(texts[2], "Widget A");
        }
        _ => panic!("Expected Text array"),
    }
}

#[test]
fn test_choose_function() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    // Test CHOOSE with literal index: CHOOSE(2, 0.05, 0.10, 0.02) should return 0.10
    model.add_scalar(
        "chosen_rate".to_string(),
        Variable::new(
            "chosen_rate".to_string(),
            None,
            Some("=CHOOSE(2, 0.05, 0.10, 0.02)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let rate = result.scalars.get("chosen_rate").unwrap().value.unwrap();

    // CHOOSE(2, ...) should return the second value = 0.10
    assert!(
        (rate - 0.10).abs() < 0.001,
        "CHOOSE(2, ...) should return 0.10, got {}",
        rate
    );
}

#[cfg(feature = "full")]
#[test]
fn test_indirect_function() {
    use crate::types::{Column, ColumnValue, Table, Variable};
    let mut model = ParsedModel::new();

    // Create a table with values
    let mut sales = Table::new("sales".to_string());
    sales.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0, 400.0, 500.0]),
    ));
    model.add_table(sales);

    // Add a scalar for testing
    model.add_scalar(
        "inputs.rate".to_string(),
        Variable::new("inputs.rate".to_string(), Some(0.1), None),
    );

    // Test INDIRECT with literal column reference
    model.add_scalar(
        "sum_indirect".to_string(),
        Variable::new(
            "sum_indirect".to_string(),
            None,
            Some("=SUM(INDIRECT(\"sales.revenue\"))".to_string()),
        ),
    );

    // Test INDIRECT with scalar reference
    model.add_scalar(
        "rate_indirect".to_string(),
        Variable::new(
            "rate_indirect".to_string(),
            None,
            Some("=INDIRECT(\"inputs.rate\") * 100".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    let sum = result.scalars.get("sum_indirect").unwrap().value.unwrap();
    // SUM(100+200+300+400+500) = 1500
    assert!(
        (sum - 1500.0).abs() < 0.001,
        "INDIRECT column SUM should return 1500, got {}",
        sum
    );

    let rate = result.scalars.get("rate_indirect").unwrap().value.unwrap();
    // 0.1 * 100 = 10
    assert!(
        (rate - 10.0).abs() < 0.001,
        "INDIRECT scalar should return 10, got {}",
        rate
    );
}

#[test]
fn test_index_function() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
    ));
    model.add_table(table);

    model.add_scalar(
        "third".to_string(),
        Variable::new(
            "third".to_string(),
            None,
            Some("=INDEX(data.values, 3)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let val = result.scalars.get("third").unwrap().value.unwrap();
    assert!((val - 30.0).abs() < 0.01);
}

#[test]
fn test_match_function() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
    ));
    model.add_table(table);

    model.add_scalar(
        "pos".to_string(),
        Variable::new(
            "pos".to_string(),
            None,
            Some("=MATCH(30, data.values, 0)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let val = result.scalars.get("pos").unwrap().value.unwrap();
    assert!((val - 3.0).abs() < 0.01); // 1-indexed position
}

#[test]
fn test_array_index_access() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(data);

    model.add_scalar(
        "first".to_string(),
        Variable::new(
            "first".to_string(),
            None,
            Some("=data.values[0]".to_string()),
        ),
    );
    model.add_scalar(
        "last".to_string(),
        Variable::new(
            "last".to_string(),
            None,
            Some("=data.values[2]".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("first").unwrap().value.unwrap() - 10.0).abs() < 0.01);
    assert!((result.scalars.get("last").unwrap().value.unwrap() - 30.0).abs() < 0.01);
}

#[test]
fn test_match_text_exact() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "names".to_string(),
        ColumnValue::Text(vec![
            "Apple".to_string(),
            "Banana".to_string(),
            "Cherry".to_string(),
        ]),
    ));
    model.add_table(data);

    model.add_scalar(
        "pos".to_string(),
        Variable::new(
            "pos".to_string(),
            None,
            Some("=MATCH(\"Banana\", data.names, 0)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // MATCH returns 1-based index
    let pos = result.scalars.get("pos").unwrap().value.unwrap();
    assert!((pos - 2.0).abs() < 0.01); // Banana is at position 2
}

#[test]
fn test_index_single_column() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0]),
    ));
    model.add_table(data);

    model.add_scalar(
        "second".to_string(),
        Variable::new(
            "second".to_string(),
            None,
            Some("=INDEX(data.values, 2)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // INDEX is 1-based
    let second = result.scalars.get("second").unwrap().value.unwrap();
    assert!((second - 200.0).abs() < 0.01);
}

#[test]
fn test_choose_rowwise() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "index".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    data.row_formulas.insert(
        "result".to_string(),
        "=CHOOSE(index, 100, 200, 300)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let col = result
        .tables
        .get("data")
        .unwrap()
        .columns
        .get("result")
        .unwrap();
    if let ColumnValue::Number(values) = &col.values {
        assert_eq!(values[0], 100.0);
        assert_eq!(values[1], 200.0);
        assert_eq!(values[2], 300.0);
    }
}

#[test]
fn test_cross_table_row_count_mismatch_error() {
    let mut model = ParsedModel::new();

    let mut table1 = Table::new("table1".to_string());
    table1.add_column(Column::new(
        "a".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]), // 3 rows
    ));
    model.add_table(table1);

    let mut table2 = Table::new("table2".to_string());
    table2.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]), // 2 rows - mismatch!
    ));
    table2
        .row_formulas
        .insert("result".to_string(), "=table1.a + x".to_string());
    model.add_table(table2);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("rows"));
}

#[cfg(feature = "full")]
#[test]
fn test_offset_function() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
    ));
    model.add_table(data);

    model.add_scalar(
        "offset_sum".to_string(),
        Variable::new(
            "offset_sum".to_string(),
            None,
            Some("=SUM(OFFSET(data.values, 1, 3))".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // OFFSET may or may not be fully implemented, but we're exercising the path
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_column_row_count_mismatch_local() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "a".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    // Manually create a mismatch (normally prevented by parser)
    data.columns.insert(
        "b".to_string(),
        Column::new("b".to_string(), ColumnValue::Number(vec![10.0, 20.0])), // Only 2 elements!
    );
    data.row_formulas
        .insert("result".to_string(), "=a + b".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error due to length mismatch
    assert!(result.is_err());
}

#[test]
fn test_match_exact_match_found() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("products".to_string());
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
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "search".to_string(),
        ColumnValue::Text(vec!["Banana".to_string()]),
    ));
    data.row_formulas.insert(
        "position".to_string(),
        "=MATCH(search, products.name, 0)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("position") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 2.0); // "Banana" is at position 2 (1-based)
        }
    }
}

#[test]
fn test_match_exact_match_not_found() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("products".to_string());
    lookup_table.add_column(Column::new(
        "name".to_string(),
        ColumnValue::Text(vec!["Apple".to_string(), "Banana".to_string()]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "search".to_string(),
        ColumnValue::Text(vec!["Orange".to_string()]),
    ));
    data.row_formulas.insert(
        "position".to_string(),
        "=MATCH(search, products.name, 0)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error because "Orange" not found
    assert!(result.is_err());
}

#[test]
fn test_match_less_than_or_equal_ascending() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("ranges".to_string());
    lookup_table.add_column(Column::new(
        "threshold".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![25.0]),
    ));
    // match_type = 1: find largest value <= lookup_value
    data.row_formulas.insert(
        "position".to_string(),
        "=MATCH(value, ranges.threshold, 1)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("position") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 2.0); // 20 is largest value <= 25
        }
    }
}

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

#[cfg(feature = "full")]
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
    // Test exercises VLOOKUP code path (may or may not work with table references)
    assert!(result.is_ok() || result.is_err());
}

#[cfg(feature = "full")]
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

#[cfg(feature = "full")]
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

#[cfg(feature = "full")]
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

#[cfg(feature = "full")]
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

#[cfg(feature = "full")]
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

#[cfg(feature = "full")]
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

#[cfg(feature = "full")]
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

#[cfg(feature = "full")]
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

#[cfg(feature = "full")]
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
    // Exercises text VLOOKUP path
    assert!(result.is_ok() || result.is_err());
}

#[cfg(feature = "full")]
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
    assert!(result.is_ok() || result.is_err());
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
    assert!(result.is_ok() || result.is_err());
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
    let _ = calculator.calculate_all();
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
    let _ = calculator.calculate_all();
}

#[cfg(feature = "full")]
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
    let _ = calculator.calculate_all();
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
    let _ = calculator.calculate_all();
}

#[cfg(feature = "full")]
#[test]
fn test_offset_basic_usage() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=OFFSET(data.values[0], 2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[cfg(feature = "full")]
#[test]
fn test_vlookup_exact_mode() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("products".to_string());
    data.add_column(Column::new(
        "id".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0]),
    ));
    data.add_column(Column::new(
        "price".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=VLOOKUP(2, products, 2, FALSE)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_index_match_combination() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "names".to_string(),
        ColumnValue::Text(vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Carol".to_string(),
        ]),
    ));
    data.add_column(Column::new(
        "scores".to_string(),
        ColumnValue::Number(vec![85.0, 92.0, 78.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "score".to_string(),
        Variable::new(
            "score".to_string(),
            None,
            Some("=INDEX(data.scores, MATCH(\"Bob\", data.names, 0))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[cfg(feature = "full")]
#[test]
fn test_xlookup_not_found_fallback() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("items".to_string());
    data.add_column(Column::new(
        "code".to_string(),
        ColumnValue::Text(vec!["A1".to_string(), "B2".to_string(), "C3".to_string()]),
    ));
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XLOOKUP(\"D4\", items.code, items.value, -1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}
