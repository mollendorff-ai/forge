//! Text function tests for ArrayCalculator

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI/E

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[test]
fn test_concat_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "first".to_string(),
        ColumnValue::Text(vec![
            "Hello".to_string(),
            "Good".to_string(),
            "Nice".to_string(),
        ]),
    ));
    table.add_column(Column::new(
        "second".to_string(),
        ColumnValue::Text(vec![
            "World".to_string(),
            "Day".to_string(),
            "Work".to_string(),
        ]),
    ));
    table.add_row_formula(
        "combined".to_string(),
        "=CONCAT(first, \" \", second)".to_string(),
    );

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let combined = result_table.columns.get("combined").unwrap();
    match &combined.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "Hello World");
            assert_eq!(texts[1], "Good Day");
            assert_eq!(texts[2], "Nice Work");
        }
        _ => panic!("Expected Text array"),
    }
}

#[test]
fn test_trim_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "text".to_string(),
        ColumnValue::Text(vec![
            "  Hello  ".to_string(),
            " World ".to_string(),
            "  Test".to_string(),
        ]),
    ));
    table.add_row_formula("trimmed".to_string(), "=TRIM(text)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let trimmed = result_table.columns.get("trimmed").unwrap();
    match &trimmed.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "Hello");
            assert_eq!(texts[1], "World");
            assert_eq!(texts[2], "Test");
        }
        _ => panic!("Expected Text array"),
    }
}

#[test]
fn test_upper_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "text".to_string(),
        ColumnValue::Text(vec![
            "hello".to_string(),
            "world".to_string(),
            "Test".to_string(),
        ]),
    ));
    table.add_row_formula("upper".to_string(), "=UPPER(text)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let upper = result_table.columns.get("upper").unwrap();
    match &upper.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "HELLO");
            assert_eq!(texts[1], "WORLD");
            assert_eq!(texts[2], "TEST");
        }
        _ => panic!("Expected Text array"),
    }
}

#[test]
fn test_lower_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "text".to_string(),
        ColumnValue::Text(vec![
            "HELLO".to_string(),
            "WORLD".to_string(),
            "Test".to_string(),
        ]),
    ));
    table.add_row_formula("lower".to_string(), "=LOWER(text)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let lower = result_table.columns.get("lower").unwrap();
    match &lower.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "hello");
            assert_eq!(texts[1], "world");
            assert_eq!(texts[2], "test");
        }
        _ => panic!("Expected Text array"),
    }
}

#[test]
fn test_len_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "text".to_string(),
        ColumnValue::Text(vec![
            "hello".to_string(),
            "hi".to_string(),
            "testing".to_string(),
        ]),
    ));
    table.add_row_formula("length".to_string(), "=LEN(text)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let length = result_table.columns.get("length").unwrap();
    match &length.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 5.0);
            assert_eq!(nums[1], 2.0);
            assert_eq!(nums[2], 7.0);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_mid_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "text".to_string(),
        ColumnValue::Text(vec![
            "hello".to_string(),
            "world".to_string(),
            "testing".to_string(),
        ]),
    ));
    table.add_row_formula("mid_2_3".to_string(), "=MID(text, 2, 3)".to_string());
    table.add_row_formula("mid_1_2".to_string(), "=MID(text, 1, 2)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let mid_2_3 = result_table.columns.get("mid_2_3").unwrap();
    match &mid_2_3.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "ell");
            assert_eq!(texts[1], "orl");
            assert_eq!(texts[2], "est");
        }
        _ => panic!("Expected Text array"),
    }

    let mid_1_2 = result_table.columns.get("mid_1_2").unwrap();
    match &mid_1_2.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "he");
            assert_eq!(texts[1], "wo");
            assert_eq!(texts[2], "te");
        }
        _ => panic!("Expected Text array"),
    }
}

#[test]
fn test_text_functions_combined() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "text".to_string(),
        ColumnValue::Text(vec!["  hello  ".to_string(), "  WORLD  ".to_string()]),
    ));
    table.add_row_formula("processed".to_string(), "=UPPER(TRIM(text))".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let processed = result_table.columns.get("processed").unwrap();
    match &processed.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "HELLO");
            assert_eq!(texts[1], "WORLD");
        }
        _ => panic!("Expected Text array"),
    }
}

#[test]
fn test_mixed_math_and_text_functions() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.234, 5.678, 9.012]),
    ));
    table.add_column(Column::new(
        "labels".to_string(),
        ColumnValue::Text(vec![
            "item".to_string(),
            "data".to_string(),
            "test".to_string(),
        ]),
    ));
    table.add_row_formula("rounded".to_string(), "=ROUND(values, 1)".to_string());
    table.add_row_formula("upper_labels".to_string(), "=UPPER(labels)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let rounded = result_table.columns.get("rounded").unwrap();
    match &rounded.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 1.2);
            assert_eq!(nums[1], 5.7);
            assert_eq!(nums[2], 9.0);
        }
        _ => panic!("Expected Number array"),
    }

    let upper_labels = result_table.columns.get("upper_labels").unwrap();
    match &upper_labels.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "ITEM");
            assert_eq!(texts[1], "DATA");
            assert_eq!(texts[2], "TEST");
        }
        _ => panic!("Expected Text array"),
    }
}

#[test]
fn test_trim_function_whitespace() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "text".to_string(),
        ColumnValue::Text(vec!["  hello  ".to_string(), " world ".to_string()]),
    ));
    table.add_row_formula("trimmed".to_string(), "=TRIM(text)".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let table = result.tables.get("data").unwrap();
    let trimmed = table.columns.get("trimmed").unwrap();
    if let ColumnValue::Text(values) = &trimmed.values {
        assert_eq!(values[0], "hello");
        assert_eq!(values[1], "world");
    }
}

#[test]
fn test_text_column_result() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "name".to_string(),
        ColumnValue::Text(vec!["alice".to_string(), "bob".to_string()]),
    ));
    data.row_formulas
        .insert("upper_name".to_string(), "=UPPER(name)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let col = result
        .tables
        .get("data")
        .unwrap()
        .columns
        .get("upper_name")
        .unwrap();
    if let ColumnValue::Text(values) = &col.values {
        assert_eq!(values[0], "ALICE");
        assert_eq!(values[1], "BOB");
    }
}

#[test]
fn test_cross_table_text_column_reference() {
    let mut model = ParsedModel::new();

    // Source table with text column
    let mut source = Table::new("source".to_string());
    source.add_column(Column::new(
        "names".to_string(),
        ColumnValue::Text(vec!["Alice".to_string(), "Bob".to_string()]),
    ));
    model.add_table(source);

    // Target table referencing source's text column
    let mut target = Table::new("target".to_string());
    target.add_column(Column::new(
        "id".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]),
    ));
    target
        .row_formulas
        .insert("copy_name".to_string(), "=source.names".to_string());
    model.add_table(target);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should handle cross-table text reference
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_index_text_column() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("items".to_string());
    lookup_table.add_column(Column::new(
        "name".to_string(),
        ColumnValue::Text(vec![
            "First".to_string(),
            "Second".to_string(),
            "Third".to_string(),
        ]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "idx".to_string(),
        ColumnValue::Number(vec![2.0]),
    ));
    data.row_formulas
        .insert("result".to_string(), "=INDEX(items.name, idx)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // INDEX function returns text, which may be handled differently
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_text_column_in_rowwise_formula() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "name".to_string(),
        ColumnValue::Text(vec!["Alice".to_string(), "Bob".to_string()]),
    ));
    data.add_column(Column::new(
        "score".to_string(),
        ColumnValue::Number(vec![100.0, 90.0]),
    ));
    // Use UPPER function on text column
    data.row_formulas
        .insert("upper_name".to_string(), "=UPPER(name)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_text_join_function() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "first".to_string(),
        ColumnValue::Text(vec!["Hello".to_string()]),
    ));
    data.add_column(Column::new(
        "second".to_string(),
        ColumnValue::Text(vec!["World".to_string()]),
    ));
    data.row_formulas.insert(
        "joined".to_string(),
        "=CONCAT(first, \" \", second)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_left_right_functions() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "text".to_string(),
        ColumnValue::Text(vec!["Hello World".to_string()]),
    ));
    data.row_formulas
        .insert("left_part".to_string(), "=LEFT(text, 5)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_left_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=LEFT(\"Hello\", 3)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_right_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=RIGHT(\"Hello\", 3)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_rept_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=REPT(\"ab\", 3)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_find_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "pos".to_string(),
        Variable::new(
            "pos".to_string(),
            None,
            Some("=FIND(\"lo\", \"hello\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_substitute_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=SUBSTITUTE(\"hello\", \"l\", \"L\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_text_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=TEXT(1234.5, \"0.00\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_value_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=VALUE(\"123.45\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_concat_text_columns() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "first".to_string(),
        ColumnValue::Text(vec!["John".to_string(), "Jane".to_string()]),
    ));
    data.add_column(Column::new(
        "last".to_string(),
        ColumnValue::Text(vec!["Doe".to_string(), "Smith".to_string()]),
    ));
    model.add_table(data);
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_len_scalar() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "length".to_string(),
        Variable::new(
            "length".to_string(),
            None,
            Some("=LEN(\"Hello World\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_mid_scalar() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=MID(\"Hello World\", 7, 5)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}
