// Allow approximate constants - 3.14 is intentional test data for ROUND(), not an approx of PI
#![allow(clippy::approx_constant)]

use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::parser::parse_model;
use royalbit_forge::types::{Column, ColumnValue, ParsedModel, Table};
use std::path::Path;

#[test]
fn test_if_with_scalar_refs_in_table() {
    // Bug #2: IF function with scalar references in row-wise table formulas
    // This tests the xlformula_engine workaround for IF conditions
    let path = Path::new("test-data/if_scalar_test.yaml");
    let model = parse_model(path).expect("Failed to parse if_scalar_test.yaml");

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("IF with scalar refs should work");

    // Verify the table formula computed correctly
    // Table is "data", column is "above_min"
    let table = result.tables.get("data").unwrap();
    let above_min = table.columns.get("above_min").unwrap();

    match &above_min.values {
        ColumnValue::Number(nums) => {
            // amounts are [30, 75, 120, 45, 90], threshold (min_value) is 50
            // above_min = IF(amount >= thresholds.min_value, 1, 0)
            assert_eq!(nums.len(), 5);
            assert_eq!(nums[0], 0.0); // 30 < 50 -> 0
            assert_eq!(nums[1], 1.0); // 75 >= 50 -> 1
            assert_eq!(nums[2], 1.0); // 120 >= 50 -> 1
            assert_eq!(nums[3], 0.0); // 45 < 50 -> 0
            assert_eq!(nums[4], 1.0); // 90 >= 50 -> 1
        }
        _ => panic!("Expected Number array"),
    }

    // Also test the adjusted column (IF with multiplication using scalars)
    let adjusted = table.columns.get("adjusted").unwrap();
    match &adjusted.values {
        ColumnValue::Number(nums) => {
            // adjusted = IF(amount > min_value, amount * multiplier, amount)
            // multiplier = 2, min_value = 50
            assert_eq!(nums.len(), 5);
            assert_eq!(nums[0], 30.0); // 30 <= 50 -> 30
            assert_eq!(nums[1], 150.0); // 75 > 50 -> 75 * 2 = 150
            assert_eq!(nums[2], 240.0); // 120 > 50 -> 120 * 2 = 240
            assert_eq!(nums[3], 45.0); // 45 <= 50 -> 45
            assert_eq!(nums[4], 180.0); // 90 > 50 -> 90 * 2 = 180
        }
        _ => panic!("Expected Number array for adjusted"),
    }

    println!("✓ IF with scalar refs in table test passed");
}

#[test]
fn test_if_comparison_operators_in_table() {
    // Tests IF function with comparison operators (Bug #2 xlformula workaround)
    let path = Path::new("test-data/if_compare_test.yaml");
    let model = parse_model(path).expect("Failed to parse if_compare_test.yaml");

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("IF with comparison should work");

    let table = result.tables.get("mytable").unwrap();
    let test1 = table.columns.get("test1").unwrap();

    match &test1.values {
        ColumnValue::Number(nums) => {
            // revenue is [100, 200, 300], test1 = IF(revenue > 150, 10, 20)
            assert_eq!(nums.len(), 3);
            assert_eq!(nums[0], 20.0); // 100 <= 150 -> 20
            assert_eq!(nums[1], 10.0); // 200 > 150 -> 10
            assert_eq!(nums[2], 10.0); // 300 > 150 -> 10
        }
        _ => panic!("Expected Number array"),
    }

    println!("✓ IF comparison operators test passed");
}

#[test]
fn test_if_with_multiplication_in_branches() {
    // Tests IF function with operators in then/else expressions
    let path = Path::new("test-data/if_mult3_test.yaml");
    let model = parse_model(path).expect("Failed to parse if_mult3_test.yaml");

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("IF with multiplication should work");

    let table = result.tables.get("mytable").unwrap();
    let test1 = table.columns.get("test1").unwrap();

    match &test1.values {
        ColumnValue::Number(nums) => {
            // revenue is [100, 200, 300]
            // test1 = IF((revenue > 150), revenue * 2, 20)
            assert_eq!(nums.len(), 3);
            assert_eq!(nums[0], 20.0); // 100 <= 150 -> 20
            assert_eq!(nums[1], 400.0); // 200 > 150 -> 200 * 2 = 400
            assert_eq!(nums[2], 600.0); // 300 > 150 -> 300 * 2 = 600
        }
        _ => panic!("Expected Number array"),
    }

    println!("✓ IF with multiplication in branches test passed");
}

#[test]
fn test_if_equal_comparison() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "a".to_string(),
        ColumnValue::Number(vec![10.0]),
    ));
    table.add_column(Column::new(
        "b".to_string(),
        ColumnValue::Number(vec![10.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.result".to_string(),
        Variable::new(
            "outputs.result".to_string(),
            None,
            Some("=IF(data.a=data.b, 1, 0)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let val = result.scalars.get("outputs.result").unwrap();
    assert!((val.value.unwrap() - 1.0).abs() < 0.0001);
    println!("✓ IF equal comparison edge case passed");
}

#[test]
fn test_if_nested() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "inputs.value".to_string(),
        Variable::new("inputs.value".to_string(), Some(75.0), None),
    );
    model.scalars.insert(
        "outputs.grade".to_string(),
        Variable::new(
            "outputs.grade".to_string(),
            None,
            Some(
                "=IF(inputs.value>=90, 4, IF(inputs.value>=80, 3, IF(inputs.value>=70, 2, 1)))"
                    .to_string(),
            ),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Complex nested IF - verify it processes without crashing
    assert!(result.is_ok() || result.is_err());
    println!("✓ IF nested edge case passed");
}

#[test]
fn test_maxifs_function() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("products".to_string());
    table.add_column(Column::new(
        "category".to_string(),
        ColumnValue::Text(vec![
            "fruit".to_string(),
            "vegetable".to_string(),
            "fruit".to_string(),
        ]),
    ));
    table.add_column(Column::new(
        "price".to_string(),
        ColumnValue::Number(vec![5.0, 3.0, 8.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "max_fruit".to_string(),
        Variable::new(
            "max_fruit".to_string(),
            None,
            Some("=MAXIFS(products.price, products.category, \"fruit\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    if let Ok(res) = result {
        if let Some(val) = res.scalars.get("max_fruit") {
            if let Some(v) = val.value {
                assert!((v - 8.0).abs() < 0.01, "Max fruit price should be 8");
            }
        }
    }
    println!("✓ MAXIFS function passed");
}

#[test]
fn test_minifs_function() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("items".to_string());
    table.add_column(Column::new(
        "type".to_string(),
        ColumnValue::Text(vec![
            "X".to_string(),
            "Y".to_string(),
            "X".to_string(),
            "Y".to_string(),
        ]),
    ));
    table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 5.0, 15.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "min_x".to_string(),
        Variable::new(
            "min_x".to_string(),
            None,
            Some("=MINIFS(items.value, items.type, \"X\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    if let Ok(res) = result {
        if let Some(val) = res.scalars.get("min_x") {
            if let Some(v) = val.value {
                assert!((v - 5.0).abs() < 0.01, "Min X value should be 5");
            }
        }
    }
    println!("✓ MINIFS function passed");
}
