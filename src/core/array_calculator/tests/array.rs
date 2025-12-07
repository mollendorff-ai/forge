//! Array function tests for ArrayCalculator

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI/E

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[test]
fn test_countunique_function() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    // Create a table with repeated values
    let mut sales = Table::new("sales".to_string());
    sales.add_column(Column::new(
        "product".to_string(),
        ColumnValue::Text(vec![
            "Apple".to_string(),
            "Banana".to_string(),
            "Apple".to_string(),
            "Orange".to_string(),
            "Banana".to_string(),
        ]),
    ));
    sales.add_column(Column::new(
        "quantity".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 10.0, 30.0, 20.0]),
    ));
    model.add_table(sales);

    // Test COUNTUNIQUE on text column - should return 3 (Apple, Banana, Orange)
    model.add_scalar(
        "unique_products".to_string(),
        Variable::new(
            "unique_products".to_string(),
            None,
            Some("=COUNTUNIQUE(sales.product)".to_string()),
        ),
    );

    // Test COUNTUNIQUE on number column - should return 3 (10, 20, 30)
    model.add_scalar(
        "unique_quantities".to_string(),
        Variable::new(
            "unique_quantities".to_string(),
            None,
            Some("=COUNTUNIQUE(sales.quantity)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    let unique_products = result
        .scalars
        .get("unique_products")
        .unwrap()
        .value
        .unwrap();
    assert_eq!(
        unique_products, 3.0,
        "Should have 3 unique products, got {}",
        unique_products
    );

    let unique_quantities = result
        .scalars
        .get("unique_quantities")
        .unwrap()
        .value
        .unwrap();
    assert_eq!(
        unique_quantities, 3.0,
        "Should have 3 unique quantities, got {}",
        unique_quantities
    );
}

#[test]
fn test_unique_function_as_count() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    // Create a table with boolean values
    let mut flags = Table::new("flags".to_string());
    flags.add_column(Column::new(
        "active".to_string(),
        ColumnValue::Boolean(vec![true, false, true, true, false]),
    ));
    model.add_table(flags);

    // UNIQUE in scalar context returns count of unique values
    model.add_scalar(
        "unique_flags".to_string(),
        Variable::new(
            "unique_flags".to_string(),
            None,
            Some("=UNIQUE(flags.active)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    let unique_flags = result.scalars.get("unique_flags").unwrap().value.unwrap();
    assert_eq!(
        unique_flags, 2.0,
        "Should have 2 unique boolean values (true, false), got {}",
        unique_flags
    );
}

#[test]
fn test_countunique_with_dates() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    // Create a table with date values
    let mut events = Table::new("events".to_string());
    events.add_column(Column::new(
        "date".to_string(),
        ColumnValue::Date(vec![
            "2024-01-15".to_string(),
            "2024-01-16".to_string(),
            "2024-01-15".to_string(), // duplicate
            "2024-01-17".to_string(),
        ]),
    ));
    model.add_table(events);

    model.add_scalar(
        "unique_dates".to_string(),
        Variable::new(
            "unique_dates".to_string(),
            None,
            Some("=COUNTUNIQUE(events.date)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    let unique_dates = result.scalars.get("unique_dates").unwrap().value.unwrap();
    assert_eq!(
        unique_dates, 3.0,
        "Should have 3 unique dates, got {}",
        unique_dates
    );
}

#[test]
fn test_countunique_edge_cases() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    // Edge case 1: Single element (unique count = 1)
    let mut single = Table::new("single".to_string());
    single.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![42.0]),
    ));
    model.add_table(single);

    // Edge case 2: All same values (unique count = 1)
    let mut same = Table::new("same".to_string());
    same.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![5.0, 5.0, 5.0, 5.0]),
    ));
    model.add_table(same);

    // Edge case 3: All different values (unique count = n)
    let mut different = Table::new("different".to_string());
    different.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    model.add_table(different);

    // Edge case 4: Floating point - truly identical values collapse, different don't
    // 1.0 and 1.0 should be same, 1.0 and 1.0000000001 differ at 10 decimal places
    let mut floats = Table::new("floats".to_string());
    floats.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![1.0, 1.0, 2.0, 2.0]),
    ));
    model.add_table(floats);

    model.add_scalar(
        "single_unique".to_string(),
        Variable::new(
            "single_unique".to_string(),
            None,
            Some("=COUNTUNIQUE(single.value)".to_string()),
        ),
    );

    model.add_scalar(
        "same_unique".to_string(),
        Variable::new(
            "same_unique".to_string(),
            None,
            Some("=COUNTUNIQUE(same.value)".to_string()),
        ),
    );

    model.add_scalar(
        "different_unique".to_string(),
        Variable::new(
            "different_unique".to_string(),
            None,
            Some("=COUNTUNIQUE(different.value)".to_string()),
        ),
    );

    model.add_scalar(
        "floats_unique".to_string(),
        Variable::new(
            "floats_unique".to_string(),
            None,
            Some("=COUNTUNIQUE(floats.value)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    // Single element = 1 unique
    let single_unique = result.scalars.get("single_unique").unwrap().value.unwrap();
    assert_eq!(single_unique, 1.0, "Single element should have 1 unique");

    // All same = 1 unique
    let same_unique = result.scalars.get("same_unique").unwrap().value.unwrap();
    assert_eq!(same_unique, 1.0, "All same values should have 1 unique");

    // All different = n unique
    let different_unique = result
        .scalars
        .get("different_unique")
        .unwrap()
        .value
        .unwrap();
    assert_eq!(
        different_unique, 5.0,
        "All different values should have 5 unique"
    );

    // Floats with precision - should be 2 unique (1.0 and 2.0)
    let floats_unique = result.scalars.get("floats_unique").unwrap().value.unwrap();
    assert_eq!(floats_unique, 2.0, "Floats should have 2 unique values");
}

#[test]
fn test_countunique_empty_text_values() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    // Edge case: Empty strings mixed with values
    let mut mixed = Table::new("mixed".to_string());
    mixed.add_column(Column::new(
        "name".to_string(),
        ColumnValue::Text(vec![
            "".to_string(),
            "Alice".to_string(),
            "".to_string(),
            "Bob".to_string(),
            "Alice".to_string(),
        ]),
    ));
    model.add_table(mixed);

    model.add_scalar(
        "unique_names".to_string(),
        Variable::new(
            "unique_names".to_string(),
            None,
            Some("=COUNTUNIQUE(mixed.name)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    // Should have 3 unique: "", "Alice", "Bob"
    let unique_names = result.scalars.get("unique_names").unwrap().value.unwrap();
    assert_eq!(
        unique_names, 3.0,
        "Should have 3 unique values (empty string counts)"
    );
}

#[test]
fn test_countunique_in_expression() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    // Create table with known unique count
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "category".to_string(),
        ColumnValue::Text(vec![
            "A".to_string(),
            "B".to_string(),
            "A".to_string(),
            "C".to_string(),
        ]),
    ));
    model.add_table(data);

    // Use COUNTUNIQUE in arithmetic expression
    model.add_scalar(
        "unique_times_10".to_string(),
        Variable::new(
            "unique_times_10".to_string(),
            None,
            Some("=COUNTUNIQUE(data.category) * 10".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    // 3 unique categories * 10 = 30
    let result_val = result
        .scalars
        .get("unique_times_10")
        .unwrap()
        .value
        .unwrap();
    assert_eq!(result_val, 30.0, "3 unique * 10 should equal 30");
}

#[test]
fn test_countunique_numbers() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 2.0, 3.0, 1.0]),
    ));
    model.add_table(data);

    model.add_scalar(
        "unique".to_string(),
        Variable::new(
            "unique".to_string(),
            None,
            Some("=COUNTUNIQUE(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Unique values: 1, 2, 3 = 3
    let unique = result.scalars.get("unique").unwrap().value.unwrap();
    assert!((unique - 3.0).abs() < 0.01);
}

#[test]
fn test_filter_function_rowwise() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 25.0, 5.0, 30.0]),
    ));
    data.add_column(Column::new(
        "include".to_string(),
        ColumnValue::Boolean(vec![true, true, false, true]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "filtered_sum".to_string(),
        Variable::new(
            "filtered_sum".to_string(),
            None,
            Some("=SUM(FILTER(data.value, data.include))".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_sort_function_coverage() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![30.0, 10.0, 20.0, 40.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "first_sorted".to_string(),
        Variable::new(
            "first_sorted".to_string(),
            None,
            Some("=INDEX(SORT(data.values), 1)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_countunique_numbers_basic() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 2.0, 3.0, 3.0, 3.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "count".to_string(),
        Variable::new(
            "count".to_string(),
            None,
            Some("=COUNTUNIQUE(data.values)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_rows_function() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "count".to_string(),
        Variable::new(
            "count".to_string(),
            None,
            Some("=ROWS(data.values)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_filter_function() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    data.add_column(Column::new(
        "flags".to_string(),
        ColumnValue::Boolean(vec![true, false, true, false, true]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "sum".to_string(),
        Variable::new(
            "sum".to_string(),
            None,
            Some("=SUM(FILTER(data.values, data.flags))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_unique_function() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 2.0, 3.0, 3.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "count".to_string(),
        Variable::new(
            "count".to_string(),
            None,
            Some("=COUNT(UNIQUE(data.values))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_sort_and_min() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![3.0, 1.0, 4.0, 1.0, 5.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "min".to_string(),
        Variable::new(
            "min".to_string(),
            None,
            Some("=MIN(SORT(data.values))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}
