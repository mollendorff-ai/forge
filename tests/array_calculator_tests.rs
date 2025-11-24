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
