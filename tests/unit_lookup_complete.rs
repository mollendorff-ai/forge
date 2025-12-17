//! v6.7.0 - Lookup Functions Complete Unit Tests
//!
//! Comprehensive coverage of enterprise lookup functions:
//! HLOOKUP (horizontal lookup), INDIRECT (reference from text), XLOOKUP (modern lookup)
//!
//! Tests: Basic functionality, exact/approximate match, not found, out of bounds,
//! error handling, match modes, edge cases

#![cfg(not(feature = "demo"))]

use royalbit_forge::core::array_calculator::ArrayCalculator;
use royalbit_forge::types::{Column, ColumnValue, ParsedModel, Table, Variable};

// ═══════════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
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
// HLOOKUP FUNCTION TESTS (8 tests)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_hlookup_exact_match_numbers() {
    let mut model = ParsedModel::new();

    // Create a horizontal table: first row = IDs, second row = values
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "ids".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0]),
    ));
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    model.add_table(data);

    // HLOOKUP searches in the array and returns from the same array
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=HLOOKUP(3, data.ids, 1, FALSE())".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(val, 3.0, "HLOOKUP should find 3 in the array");
}

#[test]
fn test_hlookup_exact_match_text() {
    let mut model = ParsedModel::new();

    let mut products = Table::new("products".to_string());
    products.add_column(Column::new(
        "names".to_string(),
        ColumnValue::Text(vec![
            "Apple".to_string(),
            "Banana".to_string(),
            "Cherry".to_string(),
        ]),
    ));
    products.add_column(Column::new(
        "prices".to_string(),
        ColumnValue::Number(vec![1.5, 0.75, 2.25]),
    ));
    model.add_table(products);

    let mut query = Table::new("query".to_string());
    query.add_column(Column::new(
        "search".to_string(),
        ColumnValue::Text(vec!["Banana".to_string()]),
    ));
    query.add_row_formula(
        "found".to_string(),
        "=HLOOKUP(search, products.names, 1, FALSE())".to_string(),
    );
    model.add_table(query);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let result_table = result.tables.get("query").unwrap();
    let found = result_table.columns.get("found").unwrap();

    match &found.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "Banana", "HLOOKUP should find Banana");
        }
        _ => panic!("Expected Text result"),
    }
}

#[test]
fn test_hlookup_approximate_match() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "thresholds".to_string(),
        ColumnValue::Number(vec![0.0, 50.0, 100.0, 150.0]),
    ));
    model.add_table(data);

    // HLOOKUP with approximate match (TRUE or omitted) finds largest value <= lookup
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=HLOOKUP(75, data.thresholds, 1, TRUE())".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(
        val, 50.0,
        "HLOOKUP approximate should find 50 (largest <= 75)"
    );
}

#[test]
fn test_hlookup_default_approximate() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(data);

    // Default behavior is approximate match (range_lookup = TRUE)
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=HLOOKUP(25, data.values, 1)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(
        val, 20.0,
        "HLOOKUP default should use approximate match (find 20 for lookup 25)"
    );
}

#[test]
#[should_panic(expected = "Calculation should succeed")]
fn test_hlookup_not_found_exact() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    model.add_table(data);

    // Looking for 99 with exact match should fail
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=HLOOKUP(99, data.values, 1, FALSE())".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let _result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
}

#[test]
fn test_hlookup_first_element() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "items".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0]),
    ));
    model.add_table(data);

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=HLOOKUP(100, data.items, 1, FALSE())".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(val, 100.0, "HLOOKUP should find first element");
}

#[test]
fn test_hlookup_last_element() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "items".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    model.add_table(data);

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=HLOOKUP(40, data.items, 1, FALSE())".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(val, 40.0, "HLOOKUP should find last element");
}

#[test]
fn test_hlookup_zero() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![0.0, 5.0, 10.0]),
    ));
    model.add_table(data);

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=HLOOKUP(0, data.values, 1, FALSE())".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(val, 0.0, "HLOOKUP should handle zero value");
}

// ═══════════════════════════════════════════════════════════════════════════════
// INDIRECT FUNCTION TESTS (7 tests)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_indirect_scalar_reference() {
    let mut model = ParsedModel::new();

    model.add_scalar(
        "inputs.rate".to_string(),
        Variable::new("inputs.rate".to_string(), Some(0.05), None),
    );

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=INDIRECT(\"inputs.rate\") * 100".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(
        val, 5.0,
        "INDIRECT should resolve scalar reference (0.05*100=5)"
    );
}

#[test]
fn test_indirect_table_column() {
    let mut model = ParsedModel::new();

    let mut sales = Table::new("sales".to_string());
    sales.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0]),
    ));
    model.add_table(sales);

    model.add_scalar(
        "total".to_string(),
        Variable::new(
            "total".to_string(),
            None,
            Some("=SUM(INDIRECT(\"sales.revenue\"))".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("total").unwrap().value.unwrap();
    assert_eq!(val, 600.0, "INDIRECT should resolve table.column reference");
}

#[test]
fn test_indirect_with_text_column() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "names".to_string(),
        ColumnValue::Text(vec!["Alice".to_string(), "Bob".to_string()]),
    ));
    model.add_table(data);

    // Use a table with row formula to extract the text value
    let mut query = Table::new("query".to_string());
    query.add_column(Column::new(
        "id".to_string(),
        ColumnValue::Number(vec![1.0]),
    ));
    query.add_row_formula(
        "first_name".to_string(),
        "=INDEX(INDIRECT(\"data.names\"), 1)".to_string(),
    );
    model.add_table(query);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let result_table = result.tables.get("query").unwrap();
    let first_name = result_table.columns.get("first_name").unwrap();

    match &first_name.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "Alice", "INDIRECT should work with text columns");
        }
        _ => panic!("Expected Text result"),
    }
}

#[test]
fn test_indirect_zero_value() {
    let mut model = ParsedModel::new();

    model.add_scalar(
        "zero_val".to_string(),
        Variable::new("zero_val".to_string(), Some(0.0), None),
    );

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=INDIRECT(\"zero_val\") + 10".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(val, 10.0, "INDIRECT should handle zero values");
}

#[test]
fn test_indirect_negative_value() {
    let mut model = ParsedModel::new();

    model.add_scalar(
        "neg_val".to_string(),
        Variable::new("neg_val".to_string(), Some(-42.5), None),
    );

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=INDIRECT(\"neg_val\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(val, -42.5, "INDIRECT should handle negative values");
}

#[test]
#[should_panic(expected = "Calculation should succeed")]
fn test_indirect_invalid_reference() {
    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=INDIRECT(\"nonexistent.ref\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let _result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
}

#[test]
fn test_indirect_nested_calculation() {
    let mut model = ParsedModel::new();

    model.add_scalar(
        "base".to_string(),
        Variable::new("base".to_string(), Some(100.0), None),
    );

    model.add_scalar(
        "multiplier".to_string(),
        Variable::new("multiplier".to_string(), Some(3.0), None),
    );

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=INDIRECT(\"base\") * INDIRECT(\"multiplier\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(val, 300.0, "INDIRECT should work in nested calculations");
}

// ═══════════════════════════════════════════════════════════════════════════════
// XLOOKUP FUNCTION TESTS (12 tests)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_xlookup_exact_match_basic() {
    let mut model = ParsedModel::new();

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

    let mut sales = Table::new("sales".to_string());
    sales.add_column(Column::new(
        "search_id".to_string(),
        ColumnValue::Number(vec![103.0]),
    ));
    sales.add_row_formula(
        "name".to_string(),
        "=XLOOKUP(search_id, products.product_id, products.product_name)".to_string(),
    );
    model.add_table(sales);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let result_table = result.tables.get("sales").unwrap();
    let name = result_table.columns.get("name").unwrap();

    match &name.values {
        ColumnValue::Text(texts) => {
            assert_eq!(
                texts[0], "Widget C",
                "XLOOKUP should find Widget C for ID 103"
            );
        }
        _ => panic!("Expected Text result"),
    }
}

#[test]
fn test_xlookup_not_found_default() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "ids".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(data);

    let mut query = Table::new("query".to_string());
    query.add_column(Column::new(
        "search".to_string(),
        ColumnValue::Number(vec![99.0]),
    ));
    query.add_row_formula(
        "result".to_string(),
        "=XLOOKUP(search, data.ids, data.values, -999)".to_string(),
    );
    model.add_table(query);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let result_table = result.tables.get("query").unwrap();
    let res = result_table.columns.get("result").unwrap();

    match &res.values {
        ColumnValue::Number(nums) => {
            assert_eq!(
                nums[0], -999.0,
                "XLOOKUP should return if_not_found value when no match"
            );
        }
        _ => panic!("Expected Number result"),
    }
}

#[test]
fn test_xlookup_not_found_text() {
    let mut model = ParsedModel::new();

    let mut products = Table::new("products".to_string());
    products.add_column(Column::new(
        "codes".to_string(),
        ColumnValue::Text(vec!["A1".to_string(), "B2".to_string(), "C3".to_string()]),
    ));
    products.add_column(Column::new(
        "names".to_string(),
        ColumnValue::Text(vec![
            "Alpha".to_string(),
            "Beta".to_string(),
            "Gamma".to_string(),
        ]),
    ));
    model.add_table(products);

    let mut search = Table::new("search".to_string());
    search.add_column(Column::new(
        "code".to_string(),
        ColumnValue::Text(vec!["Z9".to_string()]),
    ));
    search.add_row_formula(
        "name".to_string(),
        "=XLOOKUP(code, products.codes, products.names, \"Not Found\")".to_string(),
    );
    model.add_table(search);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let result_table = result.tables.get("search").unwrap();
    let name = result_table.columns.get("name").unwrap();

    match &name.values {
        ColumnValue::Text(texts) => {
            assert_eq!(
                texts[0], "Not Found",
                "XLOOKUP should return text if_not_found value"
            );
        }
        _ => panic!("Expected Text result"),
    }
}

#[test]
#[should_panic(expected = "Calculation should succeed")]
fn test_xlookup_not_found_no_default() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "ids".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(data);

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XLOOKUP(99, data.ids, data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let _result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
}

#[test]
fn test_xlookup_match_mode_exact() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "keys".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0]),
    ));
    model.add_table(data);

    // XLOOKUP returns a scalar value, not an array, so no need for INDEX
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XLOOKUP(30, data.keys, data.values, -1, 0)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(
        val, 3.0,
        "XLOOKUP with match_mode=0 should find exact match"
    );
}

#[test]
fn test_xlookup_match_mode_next_smaller() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "thresholds".to_string(),
        ColumnValue::Number(vec![0.0, 50.0, 100.0, 150.0]),
    ));
    data.add_column(Column::new(
        "levels".to_string(),
        ColumnValue::Text(vec![
            "Low".to_string(),
            "Medium".to_string(),
            "High".to_string(),
            "Very High".to_string(),
        ]),
    ));
    model.add_table(data);

    let mut query = Table::new("query".to_string());
    query.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![75.0]),
    ));
    query.add_row_formula(
        "level".to_string(),
        "=XLOOKUP(value, data.thresholds, data.levels, \"Unknown\", -1)".to_string(),
    );
    model.add_table(query);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let result_table = result.tables.get("query").unwrap();
    let level = result_table.columns.get("level").unwrap();

    match &level.values {
        ColumnValue::Text(texts) => {
            assert_eq!(
                texts[0], "Medium",
                "XLOOKUP match_mode=-1 should find next smaller (50 for 75)"
            );
        }
        _ => panic!("Expected Text result"),
    }
}

#[test]
fn test_xlookup_match_mode_next_larger() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    data.add_column(Column::new(
        "results".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0, 400.0]),
    ));
    model.add_table(data);

    let mut query = Table::new("query".to_string());
    query.add_column(Column::new(
        "search".to_string(),
        ColumnValue::Number(vec![25.0]),
    ));
    query.add_row_formula(
        "result".to_string(),
        "=XLOOKUP(search, data.values, data.results, -1, 1)".to_string(),
    );
    model.add_table(query);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let result_table = result.tables.get("query").unwrap();
    let res = result_table.columns.get("result").unwrap();

    match &res.values {
        ColumnValue::Number(nums) => {
            assert_eq!(
                nums[0], 300.0,
                "XLOOKUP match_mode=1 should find next larger (30 for 25)"
            );
        }
        _ => panic!("Expected Number result"),
    }
}

#[test]
#[should_panic(expected = "Calculation should succeed")]
fn test_xlookup_invalid_match_mode() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "keys".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(data);

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XLOOKUP(2, data.keys, data.values, -1, 99)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let _result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
}

#[test]
#[should_panic(expected = "Calculation should succeed")]
fn test_xlookup_array_length_mismatch() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "keys".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]), // Shorter array!
    ));
    model.add_table(data);

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=XLOOKUP(2, data.keys, data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let _result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
}

#[test]
fn test_xlookup_first_element() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "ids".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    data.add_column(Column::new(
        "names".to_string(),
        ColumnValue::Text(vec![
            "First".to_string(),
            "Second".to_string(),
            "Third".to_string(),
        ]),
    ));
    model.add_table(data);

    let mut query = Table::new("query".to_string());
    query.add_column(Column::new(
        "search".to_string(),
        ColumnValue::Number(vec![1.0]),
    ));
    query.add_row_formula(
        "name".to_string(),
        "=XLOOKUP(search, data.ids, data.names)".to_string(),
    );
    model.add_table(query);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let result_table = result.tables.get("query").unwrap();
    let name = result_table.columns.get("name").unwrap();

    match &name.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "First", "XLOOKUP should find first element");
        }
        _ => panic!("Expected Text result"),
    }
}

#[test]
fn test_xlookup_last_element() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "codes".to_string(),
        ColumnValue::Text(vec!["A".to_string(), "B".to_string(), "C".to_string()]),
    ));
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0]),
    ));
    model.add_table(data);

    let mut query = Table::new("query".to_string());
    query.add_column(Column::new(
        "search".to_string(),
        ColumnValue::Text(vec!["C".to_string()]),
    ));
    query.add_row_formula(
        "value".to_string(),
        "=XLOOKUP(search, data.codes, data.values)".to_string(),
    );
    model.add_table(query);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let result_table = result.tables.get("query").unwrap();
    let value = result_table.columns.get("value").unwrap();

    match &value.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 300.0, "XLOOKUP should find last element");
        }
        _ => panic!("Expected Number result"),
    }
}

#[test]
fn test_xlookup_zero_value() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "amounts".to_string(),
        ColumnValue::Number(vec![0.0, 10.0, 20.0]),
    ));
    data.add_column(Column::new(
        "labels".to_string(),
        ColumnValue::Text(vec![
            "Zero".to_string(),
            "Ten".to_string(),
            "Twenty".to_string(),
        ]),
    ));
    model.add_table(data);

    let mut query = Table::new("query".to_string());
    query.add_column(Column::new(
        "search".to_string(),
        ColumnValue::Number(vec![0.0]),
    ));
    query.add_row_formula(
        "label".to_string(),
        "=XLOOKUP(search, data.amounts, data.labels)".to_string(),
    );
    model.add_table(query);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let result_table = result.tables.get("query").unwrap();
    let label = result_table.columns.get("label").unwrap();

    match &label.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "Zero", "XLOOKUP should handle zero values");
        }
        _ => panic!("Expected Text result"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// COMBINED / INTEGRATION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_combined_xlookup_with_indirect() {
    let mut model = ParsedModel::new();

    let mut employees = Table::new("employees".to_string());
    employees.add_column(Column::new(
        "emp_id".to_string(),
        ColumnValue::Number(vec![1001.0, 1002.0, 1003.0]),
    ));
    employees.add_column(Column::new(
        "salary".to_string(),
        ColumnValue::Number(vec![50000.0, 60000.0, 70000.0]),
    ));
    model.add_table(employees);

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some(
                "=XLOOKUP(1002, INDIRECT(\"employees.emp_id\"), INDIRECT(\"employees.salary\"))"
                    .to_string(),
            ),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(val, 60000.0, "XLOOKUP should work with INDIRECT references");
}

#[test]
fn test_combined_indirect_with_sum() {
    let mut model = ParsedModel::new();

    model.add_scalar(
        "val1".to_string(),
        Variable::new("val1".to_string(), Some(10.0), None),
    );
    model.add_scalar(
        "val2".to_string(),
        Variable::new("val2".to_string(), Some(20.0), None),
    );

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=INDIRECT(\"val1\") + INDIRECT(\"val2\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("result").unwrap().value.unwrap();
    assert_eq!(val, 30.0, "INDIRECT should work in arithmetic expressions");
}

#[test]
fn test_combined_hlookup_with_match() {
    let mut model = ParsedModel::new();

    let mut prices = Table::new("prices".to_string());
    prices.add_column(Column::new(
        "tiers".to_string(),
        ColumnValue::Text(vec![
            "Basic".to_string(),
            "Pro".to_string(),
            "Enterprise".to_string(),
        ]),
    ));
    prices.add_column(Column::new(
        "costs".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 50.0]),
    ));
    model.add_table(prices);

    // Use HLOOKUP to find Pro tier
    let mut query = Table::new("query".to_string());
    query.add_column(Column::new(
        "search".to_string(),
        ColumnValue::Text(vec!["Pro".to_string()]),
    ));
    query.add_row_formula(
        "tier".to_string(),
        "=HLOOKUP(search, prices.tiers, 1, FALSE())".to_string(),
    );
    model.add_table(query);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let result_table = result.tables.get("query").unwrap();
    let tier = result_table.columns.get("tier").unwrap();

    match &tier.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "Pro", "Combined lookup operations should work");
        }
        _ => panic!("Expected Text result"),
    }
}

#[test]
fn test_edge_xlookup_with_boolean_logic() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "flags".to_string(),
        ColumnValue::Number(vec![0.0, 1.0, 2.0]),
    ));
    data.add_column(Column::new(
        "statuses".to_string(),
        ColumnValue::Text(vec![
            "Inactive".to_string(),
            "Active".to_string(),
            "Pending".to_string(),
        ]),
    ));
    model.add_table(data);

    let mut query = Table::new("query".to_string());
    query.add_column(Column::new(
        "flag".to_string(),
        ColumnValue::Number(vec![1.0]),
    ));
    query.add_row_formula(
        "status".to_string(),
        "=IF(ISNUMBER(flag), XLOOKUP(flag, data.flags, data.statuses, \"Unknown\"), \"Invalid\")"
            .to_string(),
    );
    model.add_table(query);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let result_table = result.tables.get("query").unwrap();
    let status = result_table.columns.get("status").unwrap();

    match &status.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "Active", "XLOOKUP should work with boolean logic");
        }
        _ => panic!("Expected Text result"),
    }
}
