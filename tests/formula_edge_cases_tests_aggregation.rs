//! Formula edge case tests for 100% coverage
//! Tests date, math, text, lookup, array, conditional aggregation, and FORGE functions
//! Uses programmatic model creation for reliability

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI/E

use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::types::{Column, ColumnValue, ParsedModel, Table, Variable};

// Helper to create a variable with formula
#[allow(dead_code)]
fn var_formula(path: &str, formula: &str) -> Variable {
    Variable::new(path.to_string(), None, Some(formula.to_string()))
}

// Helper to create a variable with value
#[allow(dead_code)]
fn var_value(path: &str, value: f64) -> Variable {
    Variable::new(path.to_string(), Some(value), None)
}

// ═══════════════════════════════════════════════════════════════════════════
// DATE FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_sumif_text_criteria() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("sales".to_string());
    table.add_column(Column::new(
        "region".to_string(),
        ColumnValue::Text(vec![
            "North".to_string(),
            "South".to_string(),
            "North".to_string(),
            "East".to_string(),
            "North".to_string(),
        ]),
    ));
    table.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![1000.0, 500.0, 1500.0, 750.0, 2000.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "results.sumif".to_string(),
        var_formula(
            "results.sumif",
            "=SUMIF(sales.region, \"North\", sales.amount)",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let s = result.scalars.get("results.sumif").unwrap();
    assert_eq!(s.value, Some(4500.0)); // 1000 + 1500 + 2000
}

#[test]
fn test_countif_text_criteria() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("sales".to_string());
    table.add_column(Column::new(
        "region".to_string(),
        ColumnValue::Text(vec![
            "North".to_string(),
            "South".to_string(),
            "North".to_string(),
            "East".to_string(),
            "North".to_string(),
        ]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "results.countif".to_string(),
        var_formula("results.countif", "=COUNTIF(sales.region, \"North\")"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let c = result.scalars.get("results.countif").unwrap();
    assert_eq!(c.value, Some(3.0));
}

#[test]
fn test_averageif_text_criteria() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("sales".to_string());
    table.add_column(Column::new(
        "region".to_string(),
        ColumnValue::Text(vec![
            "North".to_string(),
            "South".to_string(),
            "North".to_string(),
            "East".to_string(),
            "North".to_string(),
        ]),
    ));
    table.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![1000.0, 500.0, 1500.0, 750.0, 2000.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "results.avgif".to_string(),
        var_formula(
            "results.avgif",
            "=AVERAGEIF(sales.region, \"North\", sales.amount)",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let a = result.scalars.get("results.avgif").unwrap();
    assert_eq!(a.value, Some(1500.0)); // 4500 / 3
}

#[test]
fn test_sumif_numeric_gt() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("sales".to_string());
    table.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![1000.0, 500.0, 1500.0, 750.0, 2000.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "results.sumif_gt".to_string(),
        var_formula(
            "results.sumif_gt",
            "=SUMIF(sales.amount, \">1000\", sales.amount)",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let s = result.scalars.get("results.sumif_gt").unwrap();
    assert_eq!(s.value, Some(3500.0)); // 1500 + 2000
}

#[test]
fn test_sumifs_multiple_criteria() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("sales".to_string());
    table.add_column(Column::new(
        "region".to_string(),
        ColumnValue::Text(vec![
            "North".to_string(),
            "South".to_string(),
            "North".to_string(),
            "East".to_string(),
            "North".to_string(),
        ]),
    ));
    table.add_column(Column::new(
        "status".to_string(),
        ColumnValue::Text(vec![
            "Active".to_string(),
            "Active".to_string(),
            "Inactive".to_string(),
            "Active".to_string(),
            "Active".to_string(),
        ]),
    ));
    table.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![1000.0, 500.0, 1500.0, 750.0, 2000.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "results.sumifs".to_string(),
        var_formula(
            "results.sumifs",
            "=SUMIFS(sales.amount, sales.region, \"North\", sales.status, \"Active\")",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let s = result.scalars.get("results.sumifs").unwrap();
    assert_eq!(s.value, Some(3000.0)); // 1000 + 2000 (North AND Active)
}

#[test]
fn test_countifs_multiple_criteria() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("sales".to_string());
    table.add_column(Column::new(
        "region".to_string(),
        ColumnValue::Text(vec![
            "North".to_string(),
            "South".to_string(),
            "North".to_string(),
            "East".to_string(),
            "North".to_string(),
        ]),
    ));
    table.add_column(Column::new(
        "status".to_string(),
        ColumnValue::Text(vec![
            "Active".to_string(),
            "Active".to_string(),
            "Inactive".to_string(),
            "Active".to_string(),
            "Active".to_string(),
        ]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "results.countifs".to_string(),
        var_formula(
            "results.countifs",
            "=COUNTIFS(sales.region, \"North\", sales.status, \"Active\")",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let c = result.scalars.get("results.countifs").unwrap();
    assert_eq!(c.value, Some(2.0));
}

#[test]
fn test_sum_array() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("numbers".to_string());
    table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "array.sum".to_string(),
        var_formula("array.sum", "=SUM(numbers.value)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let s = result.scalars.get("array.sum").unwrap();
    assert_eq!(s.value, Some(150.0));
}

#[test]
fn test_count_array() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("numbers".to_string());
    table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "array.count".to_string(),
        var_formula("array.count", "=COUNT(numbers.value)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let c = result.scalars.get("array.count").unwrap();
    assert_eq!(c.value, Some(5.0));
}

#[test]
fn test_max_array() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("numbers".to_string());
    table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "array.max".to_string(),
        var_formula("array.max", "=MAX(numbers.value)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let m = result.scalars.get("array.max").unwrap();
    assert_eq!(m.value, Some(50.0));
}

#[test]
fn test_min_array() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("numbers".to_string());
    table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "array.min".to_string(),
        var_formula("array.min", "=MIN(numbers.value)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let m = result.scalars.get("array.min").unwrap();
    assert_eq!(m.value, Some(10.0));
}

#[test]
fn test_cross_table_row_count_mismatch() {
    let mut model = ParsedModel::new();

    let mut table1 = Table::new("source".to_string());
    table1.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]), // 2 rows
    ));
    model.add_table(table1);

    let mut table2 = Table::new("target".to_string());
    table2.add_column(Column::new(
        "base".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]), // 3 rows
    ));
    // Add formula as row_formula
    table2
        .row_formulas
        .insert("computed".to_string(), "=source.value + base".to_string());
    model.add_table(table2);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
}

#[test]
fn test_sum_multiple_scalars() {
    let mut model = ParsedModel::new();
    model
        .scalars
        .insert("a.value".to_string(), var_value("a.value", 10.0));
    model
        .scalars
        .insert("b.value".to_string(), var_value("b.value", 20.0));
    model
        .scalars
        .insert("c.value".to_string(), var_value("c.value", 30.0));

    model.scalars.insert(
        "test.sum".to_string(),
        var_formula("test.sum", "=a.value + b.value + c.value"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let v = result.scalars.get("test.sum").unwrap();
    assert_eq!(v.value, Some(60.0));
}

#[test]
fn test_sumif_with_greater_than() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0, 25.0, 30.0, 5.0, 40.0]),
    ));
    table.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![100.0, 250.0, 300.0, 50.0, 400.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "agg.sumif".to_string(),
        var_formula("agg.sumif", "=SUMIF(data.value, \">20\", data.amount)"),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let s = result.scalars.get("agg.sumif").unwrap();
    assert_eq!(s.value, Some(950.0)); // 250 + 300 + 400 (where value > 20)
}

#[test]
fn test_averageif_function() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("scores".to_string());
    table.add_column(Column::new(
        "category".to_string(),
        ColumnValue::Text(vec![
            "A".to_string(),
            "B".to_string(),
            "A".to_string(),
            "A".to_string(),
            "B".to_string(),
        ]),
    ));
    table.add_column(Column::new(
        "score".to_string(),
        ColumnValue::Number(vec![60.0, 75.0, 80.0, 90.0, 95.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "agg.avgif".to_string(),
        var_formula(
            "agg.avgif",
            "=AVERAGEIF(scores.category, \"A\", scores.score)",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let avg = result.scalars.get("agg.avgif").unwrap();
    // Average of A scores: 60, 80, 90 = 76.67
    let v = avg.value.unwrap();
    assert!(v > 76.0 && v < 77.0);
}

#[test]
fn test_countifs_function() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("products".to_string());
    table.add_column(Column::new(
        "category".to_string(),
        ColumnValue::Text(vec![
            "A".to_string(),
            "B".to_string(),
            "A".to_string(),
            "A".to_string(),
            "B".to_string(),
        ]),
    ));
    table.add_column(Column::new(
        "price".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 15.0, 25.0, 30.0]),
    ));
    model.add_table(table);

    model.scalars.insert(
        "count.catA_high".to_string(),
        var_formula(
            "count.catA_high",
            "=COUNTIFS(products.category, \"A\", products.price, \">10\")",
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();
    let c = result.scalars.get("count.catA_high").unwrap();
    // Category A with price > 10: rows 2 (15) and 3 (25) = 2
    assert_eq!(c.value, Some(2.0));
}
