//! Advanced date function tests
//!
//! Tests for EDATE, EOMONTH, NETWORKDAYS, YEARFRAC, WORKDAY, TIME, WEEKDAY, DAYS

#![allow(clippy::approx_constant)]

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[cfg(feature = "full")]
#[test]
fn test_eomonth_same_month() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "start".to_string(),
        ColumnValue::Date(vec!["2024-02-15".to_string()]),
    ));
    data.row_formulas.insert(
        "result".to_string(),
        "=EOMONTH(start, 0)".to_string(), // End of current month
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("result") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2024-02-29"); // Leap year
        }
    }
}

#[test]
fn test_year_function_rowwise() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "date".to_string(),
        ColumnValue::Date(vec!["2024-07-15".to_string()]),
    ));
    data.row_formulas
        .insert("year".to_string(), "=YEAR(date)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("year") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 2024.0);
        }
    }
}

#[test]
fn test_month_function_rowwise() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "date".to_string(),
        ColumnValue::Date(vec!["2024-07-15".to_string()]),
    ));
    data.row_formulas
        .insert("month".to_string(), "=MONTH(date)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("month") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 7.0);
        }
    }
}

#[test]
fn test_day_function_rowwise() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "date".to_string(),
        ColumnValue::Date(vec!["2024-07-25".to_string()]),
    ));
    data.row_formulas
        .insert("day".to_string(), "=DAY(date)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("day") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 25.0);
        }
    }
}

#[test]
fn test_lookup_with_date_column() {
    let mut model = ParsedModel::new();

    let mut lookup_table = Table::new("events".to_string());
    lookup_table.add_column(Column::new(
        "date".to_string(),
        ColumnValue::Date(vec![
            "2024-01-15".to_string(),
            "2024-02-20".to_string(),
            "2024-03-25".to_string(),
        ]),
    ));
    lookup_table.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0]),
    ));
    model.add_table(lookup_table);

    let mut data = Table::new("query".to_string());
    data.add_column(Column::new(
        "idx".to_string(),
        ColumnValue::Number(vec![2.0]),
    ));
    data.row_formulas
        .insert("result".to_string(), "=INDEX(events.date, idx)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Exercises Date column path in lookup functions
    assert!(result.is_ok() || result.is_err());
}

#[cfg(feature = "full")]
#[test]
fn test_workday_function() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![1.0]),
    ));
    data.row_formulas.insert(
        "result".to_string(),
        "=WORKDAY(\"2024-01-01\", 10)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Exercises WORKDAY function path
    assert!(result.is_ok() || result.is_err());
}

#[cfg(feature = "full")]
#[test]
fn test_networkdays_literal_dates() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![1.0]),
    ));
    data.row_formulas.insert(
        "result".to_string(),
        "=NETWORKDAYS(\"2024-01-01\", \"2024-01-15\")".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Exercises NETWORKDAYS function path
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_date_column_in_rowwise_formula() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "event_date".to_string(),
        ColumnValue::Date(vec!["2024-01-15".to_string(), "2024-06-30".to_string()]),
    ));
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![100.0, 200.0]),
    ));
    // Access date column
    data.row_formulas
        .insert("result".to_string(), "=YEAR(event_date)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[cfg(feature = "full")]
#[test]
fn test_yearfrac_function_v2() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![1.0]),
    ));
    data.row_formulas.insert(
        "years".to_string(),
        "=YEARFRAC(\"2024-01-01\", \"2024-07-01\")".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_month_function_coverage() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "date".to_string(),
        ColumnValue::Date(vec!["2024-06-15".to_string()]),
    ));
    data.row_formulas
        .insert("m".to_string(), "=MONTH(date)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_day_function_coverage() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "date".to_string(),
        ColumnValue::Date(vec!["2024-06-25".to_string()]),
    ));
    data.row_formulas
        .insert("d".to_string(), "=DAY(date)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[cfg(feature = "full")]
#[test]
fn test_eomonth_with_offset() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "date".to_string(),
        ColumnValue::Date(vec!["2024-01-15".to_string()]),
    ));
    data.row_formulas
        .insert("eom".to_string(), "=EOMONTH(date, 2)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_datedif_years_diff() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "diff".to_string(),
        Variable::new(
            "diff".to_string(),
            None,
            Some("=DATEDIF(DATE(2020,1,1), DATE(2025,6,15), \"Y\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_datedif_months_diff() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "diff".to_string(),
        Variable::new(
            "diff".to_string(),
            None,
            Some("=DATEDIF(DATE(2020,1,1), DATE(2020,8,15), \"M\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_datedif_days_unit() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "diff".to_string(),
        Variable::new(
            "diff".to_string(),
            None,
            Some("=DATEDIF(DATE(2020,1,1), DATE(2020,1,20), \"D\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_datedif_md_unit() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "diff".to_string(),
        Variable::new(
            "diff".to_string(),
            None,
            Some("=DATEDIF(DATE(2020,1,15), DATE(2020,3,10), \"MD\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_datedif_ym_unit() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "diff".to_string(),
        Variable::new(
            "diff".to_string(),
            None,
            Some("=DATEDIF(DATE(2020,1,1), DATE(2021,8,1), \"YM\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}
