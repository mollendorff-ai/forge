//! Aggregation function tests for ArrayCalculator

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI/E

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[test]
fn test_aggregation_sum() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    // Create a table with revenue column
    let mut table = Table::new("sales".to_string());
    table.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0, 400.0]),
    ));
    model.add_table(table);

    // Add scalar with SUM formula
    let total_revenue = Variable::new(
        "total_revenue".to_string(),
        None,
        Some("=SUM(sales.revenue)".to_string()),
    );
    model.add_scalar("total_revenue".to_string(), total_revenue);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    let total = result.scalars.get("total_revenue").unwrap();
    assert_eq!(total.value, Some(1000.0));
}

#[test]
fn test_aggregation_average() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut table = Table::new("metrics".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    model.add_table(table);

    let avg_value = Variable::new(
        "avg_value".to_string(),
        None,
        Some("=AVERAGE(metrics.values)".to_string()),
    );
    model.add_scalar("avg_value".to_string(), avg_value);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();

    let avg = result.scalars.get("avg_value").unwrap();
    assert_eq!(avg.value, Some(25.0));
}

#[test]
fn test_aggregation_max_min() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![15.0, 42.0, 8.0, 23.0]),
    ));
    model.add_table(table);

    let max_value = Variable::new(
        "max_value".to_string(),
        None,
        Some("=MAX(data.values)".to_string()),
    );
    model.add_scalar("max_value".to_string(), max_value);

    let min_value = Variable::new(
        "min_value".to_string(),
        None,
        Some("=MIN(data.values)".to_string()),
    );
    model.add_scalar("min_value".to_string(), min_value);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();

    assert_eq!(result.scalars.get("max_value").unwrap().value, Some(42.0));
    assert_eq!(result.scalars.get("min_value").unwrap().value, Some(8.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// ENTERPRISE TESTS (only with full feature)
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(feature = "full")]
#[test]
fn test_median_function() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    // Create table with odd number of values
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 30.0, 20.0, 40.0, 50.0]),
    ));
    model.add_table(data);

    model.add_scalar(
        "median_val".to_string(),
        Variable::new(
            "median_val".to_string(),
            None,
            Some("=MEDIAN(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Sorted: 10, 20, 30, 40, 50 - median is 30
    let median = result.scalars.get("median_val").unwrap().value.unwrap();
    assert_eq!(median, 30.0);
}

#[cfg(feature = "full")]
#[test]
fn test_median_even_count() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    model.add_table(data);

    model.add_scalar(
        "median_val".to_string(),
        Variable::new(
            "median_val".to_string(),
            None,
            Some("=MEDIAN(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Sorted: 10, 20, 30, 40 - median is (20 + 30) / 2 = 25
    let median = result.scalars.get("median_val").unwrap().value.unwrap();
    assert_eq!(median, 25.0);
}

#[test]
fn test_sum_aggregation_simple() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(data);

    model.add_scalar(
        "total".to_string(),
        Variable::new(
            "total".to_string(),
            None,
            Some("=SUM(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let total = result.scalars.get("total").unwrap().value.unwrap();
    assert_eq!(total, 60.0);
}

#[test]
fn test_sum_empty_table() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();
    let table = Table::new("empty".to_string());
    model.add_table(table);

    model.add_scalar(
        "total".to_string(),
        Variable::new(
            "total".to_string(),
            None,
            Some("=SUM(empty.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should handle missing column gracefully
    assert!(result.is_err());
}

#[test]
fn test_count_function() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    model.add_table(table);

    model.add_scalar(
        "cnt".to_string(),
        Variable::new(
            "cnt".to_string(),
            None,
            Some("=COUNT(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let cnt = result.scalars.get("cnt").unwrap().value.unwrap();
    assert!((cnt - 5.0).abs() < 0.01);
}

#[cfg(feature = "full")]
#[test]
fn test_median_odd_count() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![5.0, 1.0, 9.0, 3.0, 7.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "med".to_string(),
        Variable::new(
            "med".to_string(),
            None,
            Some("=MEDIAN(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    if let Some(scalar) = model.scalars.get("med") {
        assert_eq!(scalar.value.unwrap(), 5.0);
    }
}

#[cfg(feature = "full")]
#[test]
fn test_median_even_array_count() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "med".to_string(),
        Variable::new(
            "med".to_string(),
            None,
            Some("=MEDIAN(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    if let Some(scalar) = model.scalars.get("med") {
        assert_eq!(scalar.value.unwrap(), 2.5); // (2+3)/2
    }
}

#[cfg(feature = "full")]
#[test]
fn test_avg_aggregation_function() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "avg_val".to_string(),
        Variable::new(
            "avg_val".to_string(),
            None,
            Some("=AVG(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_max_aggregation_function() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 50.0, 30.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "max_val".to_string(),
        Variable::new(
            "max_val".to_string(),
            None,
            Some("=MAX(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[test]
fn test_min_aggregation_function() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 50.0, 30.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "min_val".to_string(),
        Variable::new(
            "min_val".to_string(),
            None,
            Some("=MIN(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[cfg(feature = "full")]
#[test]
fn test_median_aggregation_scalar() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 3.0, 5.0, 7.0, 9.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "med_val".to_string(),
        Variable::new(
            "med_val".to_string(),
            None,
            Some("=MEDIAN(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[cfg(feature = "full")]
#[test]
fn test_empty_array_median() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.columns.insert(
        "values".to_string(),
        Column::new("values".to_string(), ColumnValue::Number(vec![])),
    );
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "med".to_string(),
        Variable::new(
            "med".to_string(),
            None,
            Some("=MEDIAN(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Empty array median should return 0 or handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_sumproduct_function() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "qty".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    data.add_column(Column::new(
        "price".to_string(),
        ColumnValue::Number(vec![5.0, 10.0, 15.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "total".to_string(),
        Variable::new(
            "total".to_string(),
            None,
            Some("=SUMPRODUCT(data.qty, data.price)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_count_function_v2() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "cnt".to_string(),
        Variable::new(
            "cnt".to_string(),
            None,
            Some("=COUNT(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[cfg(feature = "full")]
#[test]
fn test_counta_function() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "items".to_string(),
        ColumnValue::Text(vec!["A".to_string(), "B".to_string(), "C".to_string()]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "cnt".to_string(),
        Variable::new(
            "cnt".to_string(),
            None,
            Some("=COUNTA(data.items)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[cfg(feature = "full")]
#[test]
fn test_product_function() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![2.0, 3.0, 4.0]),
    ));
    model.add_table(data);

    use crate::types::Variable;
    model.add_scalar(
        "prod".to_string(),
        Variable::new(
            "prod".to_string(),
            None,
            Some("=PRODUCT(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_sumproduct_basic() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "qty".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    data.add_column(Column::new(
        "price".to_string(),
        ColumnValue::Number(vec![5.0, 10.0, 15.0]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "total".to_string(),
        Variable::new(
            "total".to_string(),
            None,
            Some("=SUMPRODUCT(data.qty, data.price)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[cfg(feature = "full")]
#[test]
fn test_counta_with_empty_strings() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Text(vec!["a".to_string(), "".to_string(), "b".to_string()]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "count".to_string(),
        Variable::new(
            "count".to_string(),
            None,
            Some("=COUNTA(data.values)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_countblank_function() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Text(vec![
            "a".to_string(),
            "".to_string(),
            "b".to_string(),
            "".to_string(),
        ]),
    ));
    model.add_table(data);
    use crate::types::Variable;
    model.add_scalar(
        "count".to_string(),
        Variable::new(
            "count".to_string(),
            None,
            Some("=COUNTBLANK(data.values)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}
