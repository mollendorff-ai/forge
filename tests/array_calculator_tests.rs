use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::parser::parse_model;
use royalbit_forge::types::{Column, ColumnValue, ParsedModel, Table};
use std::path::Path;

#[test]
fn test_simple_table_calculation() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("financials".to_string());
    table.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![1000.0, 1200.0, 1500.0, 1800.0]),
    ));
    table.add_column(Column::new(
        "cogs".to_string(),
        ColumnValue::Number(vec![300.0, 360.0, 450.0, 540.0]),
    ));
    table.add_row_formula("gross_profit".to_string(), "=revenue - cogs".to_string());
    table.add_row_formula(
        "gross_margin".to_string(),
        "=gross_profit / revenue".to_string(),
    );

    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    let result_table = result.tables.get("financials").unwrap();

    // Check gross_profit
    let gross_profit = result_table.columns.get("gross_profit").unwrap();
    match &gross_profit.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums.len(), 4);
            assert_eq!(nums[0], 700.0);
            assert_eq!(nums[1], 840.0);
            assert_eq!(nums[2], 1050.0);
            assert_eq!(nums[3], 1260.0);
        }
        _ => panic!("Expected Number array"),
    }

    // Check gross_margin
    let gross_margin = result_table.columns.get("gross_margin").unwrap();
    match &gross_margin.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums.len(), 4);
            assert!((nums[0] - 0.7).abs() < 0.0001);
            assert!((nums[1] - 0.7).abs() < 0.0001);
            assert!((nums[2] - 0.7).abs() < 0.0001);
            assert!((nums[3] - 0.7).abs() < 0.0001);
        }
        _ => panic!("Expected Number array"),
    }

    println!("✓ Simple table calculation succeeded");
}

#[test]
fn test_calculate_quarterly_pl() {
    let path = Path::new("test-data/v1.0/quarterly_pl.yaml");
    let model = parse_model(path).expect("Failed to parse");

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    match result {
        Ok(calculated_model) => {
            println!("✓ Calculation succeeded");
            for (name, table) in &calculated_model.tables {
                println!("  Table '{}': {} columns", name, table.columns.len());
                for (col_name, col) in &table.columns {
                    println!("    - {}: {} rows", col_name, col.values.len());
                }
            }
        }
        Err(e) => {
            println!("✗ Calculation failed: {}", e);
            panic!("Calculation failed: {}", e);
        }
    }
}

// ============================================================================
// Text Functions Tests (v1.1.0 Enhancement)
// ============================================================================
// NOTE: xlformula_engine v0.1.18 has limited function support.
// Only LEFT and RIGHT are available. See PHASE2-4-FUNCTION-SUPPORT.md for details.

#[test]
fn test_text_left_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("text".to_string());

    table.add_column(Column::new(
        "test_values".to_string(),
        ColumnValue::Text(vec![
            "hello".to_string(),
            "world".to_string(),
            "testing".to_string(),
        ]),
    ));
    table.add_row_formula("first_2".to_string(), "=LEFT(test_values, 2)".to_string());
    table.add_row_formula("first_3".to_string(), "=LEFT(test_values, 3)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("text").unwrap();

    let first_2 = result_table.columns.get("first_2").unwrap();
    match &first_2.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "he");
            assert_eq!(texts[1], "wo");
            assert_eq!(texts[2], "te");
        }
        _ => panic!("Expected Text array"),
    }

    println!("✓ LEFT function test passed");
}

#[test]
fn test_text_right_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("text".to_string());

    table.add_column(Column::new(
        "test_values".to_string(),
        ColumnValue::Text(vec![
            "hello".to_string(),
            "world".to_string(),
            "testing".to_string(),
        ]),
    ));
    table.add_row_formula("last_2".to_string(), "=RIGHT(test_values, 2)".to_string());
    table.add_row_formula("last_3".to_string(), "=RIGHT(test_values, 3)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("text").unwrap();

    let last_2 = result_table.columns.get("last_2").unwrap();
    match &last_2.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "lo");
            assert_eq!(texts[1], "ld");
            assert_eq!(texts[2], "ng");
        }
        _ => panic!("Expected Text array"),
    }

    println!("✓ RIGHT function test passed");
}

// ============================================================================
// ArrayCalculator Enhancement Tests (v1.1.0)
// ============================================================================
// Tests verifying that ArrayCalculator now supports Text, Boolean, Date columns
// (not just Number columns)

#[test]
fn test_text_column_support() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("text_test".to_string());

    table.add_column(Column::new(
        "names".to_string(),
        ColumnValue::Text(vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ]),
    ));
    // LEFT and RIGHT work with text columns
    table.add_row_formula("initials".to_string(), "=LEFT(names, 1)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Text column support should work");
    let result_table = result.tables.get("text_test").unwrap();

    let initials = result_table.columns.get("initials").unwrap();
    match &initials.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "A");
            assert_eq!(texts[1], "B");
            assert_eq!(texts[2], "C");
        }
        _ => panic!("Expected Text array"),
    }

    println!("✓ Text column support verified");
}

#[test]
fn test_mixed_column_types() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("mixed".to_string());

    table.add_column(Column::new(
        "numbers".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    table.add_column(Column::new(
        "labels".to_string(),
        ColumnValue::Text(vec![
            "Item A".to_string(),
            "Item B".to_string(),
            "Item C".to_string(),
        ]),
    ));
    // Test that we can work with both Number and Text columns in same table
    table.add_row_formula("doubled".to_string(), "=numbers * 2".to_string());
    table.add_row_formula("codes".to_string(), "=RIGHT(labels, 1)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Mixed types should work");
    let result_table = result.tables.get("mixed").unwrap();

    let doubled = result_table.columns.get("doubled").unwrap();
    match &doubled.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 20.0);
            assert_eq!(nums[1], 40.0);
            assert_eq!(nums[2], 60.0);
        }
        _ => panic!("Expected Number array"),
    }

    let codes = result_table.columns.get("codes").unwrap();
    match &codes.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "A");
            assert_eq!(texts[1], "B");
            assert_eq!(texts[2], "C");
        }
        _ => panic!("Expected Text array"),
    }

    println!("✓ Mixed column types verified");
}

// ============================================================================
// v4.3.0 Bug Fix Tests
// ============================================================================
// Tests verifying critical bug fixes for nested scalar references and IF
// functions with scalar references in table formulas

#[test]
fn test_nested_scalar_references() {
    // Bug #1: Nested scalar reference resolution
    // scalar formulas referencing other scalars using qualified names (section.scalar)
    let path = Path::new("test-data/quota_forecast.yaml");
    let model = parse_model(path).expect("Failed to parse quota_forecast.yaml");

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Nested scalar references should resolve");

    // Verify the burn_rate scalar was calculated correctly
    // burn_rate = current_usage_pct / hours_since_reset = 26 / 43 ≈ 0.6047
    let burn_rate = result.scalars.get("forecast.burn_rate").unwrap();
    assert!(
        (burn_rate.value.unwrap() - 0.6047).abs() < 0.01,
        "Expected ~0.6047, got {}",
        burn_rate.value.unwrap()
    );

    // Verify projected_total (nested reference chain)
    // projected_add = burn_rate * hours_until_reset = 0.6047 * 125 ≈ 75.58
    // projected_total = current_usage_pct + projected_add = 26 + 75.58 ≈ 101.58
    let projected_total = result.scalars.get("forecast.projected_total").unwrap();
    assert!(
        (projected_total.value.unwrap() - 101.58).abs() < 0.1,
        "Expected ~101.58, got {}",
        projected_total.value.unwrap()
    );

    println!("✓ Nested scalar references test passed");
}

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
fn test_scalar_with_metadata() {
    // Bug #3: Schema validation for scalars with metadata (value/notes/unit)
    let path = Path::new("test-data/scalar_metadata_test.yaml");
    let model = parse_model(path).expect("Failed to parse scalar_metadata_test.yaml");

    // Verify scalars were parsed correctly with metadata
    let tax_rate = model.scalars.get("config.tax_rate").unwrap();
    assert!(
        (tax_rate.value.unwrap() - 0.25).abs() < 0.0001,
        "Expected 0.25 for tax_rate"
    );

    let discount = model.scalars.get("config.discount_rate").unwrap();
    assert!(
        (discount.value.unwrap() - 0.10).abs() < 0.0001,
        "Expected 0.10 for discount_rate"
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Scalar with metadata should work");

    // Verify scalars are still present after calculation
    assert!(result.scalars.contains_key("config.tax_rate"));
    assert!(result.scalars.contains_key("config.discount_rate"));

    println!("✓ Scalar with metadata test passed");
}
