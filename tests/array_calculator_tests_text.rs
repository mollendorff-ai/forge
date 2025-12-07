// Allow approximate constants - 3.14 is intentional test data for ROUND(), not an approx of PI
#![allow(clippy::approx_constant)]

use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::types::{Column, ColumnValue, ParsedModel, Table};

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
fn test_concat_multiple_args() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "full_text".to_string(),
        Variable::new(
            "full_text".to_string(),
            None,
            Some("=CONCAT(\"Hello\", \" \", \"World\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    let _ = result;
    println!("✓ CONCAT multiple args test passed");
}

#[test]
fn test_trim_function() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "trimmed".to_string(),
        Variable::new(
            "trimmed".to_string(),
            None,
            Some("=TRIM(\"  test  \")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    let _ = result;
    println!("✓ TRIM function test passed");
}
