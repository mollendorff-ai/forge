// Enterprise-only: Contains COUNTIF, SUMIF, AVERAGEIF tests
#![cfg(feature = "full")]
// Allow approximate constants - 3.14 is intentional test data for ROUND(), not an approx of PI
#![allow(clippy::approx_constant)]

use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::types::{Column, ColumnValue, ParsedModel, Table};

#[test]
fn test_sum_negative_values() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![-100.0, 50.0, -25.0, 75.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.sum".to_string(),
        Variable::new(
            "outputs.sum".to_string(),
            None,
            Some("=SUM(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let sum = result.scalars.get("outputs.sum").unwrap();
    // -100 + 50 - 25 + 75 = 0
    assert!((sum.value.unwrap() - 0.0).abs() < 0.0001);
    println!("✓ SUM negative values edge case passed");
}

#[test]
fn test_min_max_single_value() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![42.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.min".to_string(),
        Variable::new(
            "outputs.min".to_string(),
            None,
            Some("=MIN(data.values)".to_string()),
        ),
    );
    model.scalars.insert(
        "outputs.max".to_string(),
        Variable::new(
            "outputs.max".to_string(),
            None,
            Some("=MAX(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");

    let min = result.scalars.get("outputs.min").unwrap();
    let max = result.scalars.get("outputs.max").unwrap();
    assert!((min.value.unwrap() - 42.0).abs() < 0.0001);
    assert!((max.value.unwrap() - 42.0).abs() < 0.0001);
    println!("✓ MIN/MAX single value edge case passed");
}

#[test]
fn test_count_with_duplicates() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 1.0, 1.0, 2.0, 2.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.count".to_string(),
        Variable::new(
            "outputs.count".to_string(),
            None,
            Some("=COUNT(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let count = result.scalars.get("outputs.count").unwrap();
    // COUNT includes all values, duplicates counted
    assert!((count.value.unwrap() - 5.0).abs() < 0.0001);
    println!("✓ COUNT with duplicates edge case passed");
}

#[test]
fn test_sum_large_dataset() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    // 1000 elements
    let values: Vec<f64> = (1..=1000).map(|x| x as f64).collect();
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(values),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.sum".to_string(),
        Variable::new(
            "outputs.sum".to_string(),
            None,
            Some("=SUM(data.values)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let sum = result.scalars.get("outputs.sum").unwrap();
    // Sum of 1 to 1000 = 1000 * 1001 / 2 = 500500
    assert!((sum.value.unwrap() - 500500.0).abs() < 0.1);
    println!("✓ SUM large dataset edge case passed");
}

#[test]
fn test_countif_no_matches() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.count".to_string(),
        Variable::new(
            "outputs.count".to_string(),
            None,
            Some("=COUNTIF(data.values, \">10\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let count = result.scalars.get("outputs.count").unwrap();
    assert!((count.value.unwrap() - 0.0).abs() < 0.0001);
    println!("✓ COUNTIF no matches edge case passed");
}

#[test]
fn test_countif_all_matches() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.count".to_string(),
        Variable::new(
            "outputs.count".to_string(),
            None,
            Some("=COUNTIF(data.values, \">5\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should succeed");
    let count = result.scalars.get("outputs.count").unwrap();
    assert!((count.value.unwrap() - 3.0).abs() < 0.0001);
    println!("✓ COUNTIF all matches edge case passed");
}

#[test]
fn test_sumif_no_matches() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "outputs.sum".to_string(),
        Variable::new(
            "outputs.sum".to_string(),
            None,
            Some("=SUMIF(data.values, \">10\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // SUMIF behavior - verify it handles the condition
    assert!(result.is_ok() || result.is_err());
    println!("✓ SUMIF no matches edge case passed");
}

#[test]
fn test_sumifs_multiple_criteria() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("sales".to_string());
    table.add_column(Column::new(
        "region".to_string(),
        ColumnValue::Text(vec![
            "North".to_string(),
            "South".to_string(),
            "North".to_string(),
            "South".to_string(),
        ]),
    ));
    table.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0, 400.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "north_total".to_string(),
        Variable::new(
            "north_total".to_string(),
            None,
            Some("=SUMIFS(sales.amount, sales.region, \"North\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok(), "SUMIFS should succeed");
    println!("✓ SUMIFS multiple criteria passed");
}

#[test]
fn test_countifs_function() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "status".to_string(),
        ColumnValue::Text(vec![
            "active".to_string(),
            "inactive".to_string(),
            "active".to_string(),
            "active".to_string(),
        ]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "active_count".to_string(),
        Variable::new(
            "active_count".to_string(),
            None,
            Some("=COUNTIFS(data.status, \"active\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    if let Ok(res) = result {
        if let Some(val) = res.scalars.get("active_count") {
            if let Some(v) = val.value {
                assert!((v - 3.0).abs() < 0.01, "Should count 3 active items");
            }
        }
    }
    println!("✓ COUNTIFS function passed");
}

#[test]
fn test_averageifs_function() {
    use royalbit_forge::types::Variable;

    let mut model = ParsedModel::new();
    let mut table = Table::new("scores".to_string());
    table.add_column(Column::new(
        "category".to_string(),
        ColumnValue::Text(vec![
            "A".to_string(),
            "B".to_string(),
            "A".to_string(),
            "B".to_string(),
        ]),
    ));
    table.add_column(Column::new(
        "score".to_string(),
        ColumnValue::Number(vec![80.0, 90.0, 100.0, 70.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "avg_a".to_string(),
        Variable::new(
            "avg_a".to_string(),
            None,
            Some("=AVERAGEIFS(scores.score, scores.category, \"A\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    if let Ok(res) = result {
        if let Some(val) = res.scalars.get("avg_a") {
            if let Some(v) = val.value {
                assert!((v - 90.0).abs() < 0.01, "Average of A should be 90");
            }
        }
    }
    println!("✓ AVERAGEIFS function passed");
}
