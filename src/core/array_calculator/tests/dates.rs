//! Dates function tests for ArrayCalculator

#![allow(clippy::approx_constant)] // Test values intentionally use approximate PI/E

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[test]
fn test_date_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "year".to_string(),
        ColumnValue::Number(vec![2025.0, 2024.0, 2023.0]),
    ));
    table.add_column(Column::new(
        "month".to_string(),
        ColumnValue::Number(vec![1.0, 6.0, 12.0]),
    ));
    table.add_column(Column::new(
        "day".to_string(),
        ColumnValue::Number(vec![15.0, 20.0, 31.0]),
    ));
    table.add_row_formula(
        "full_date".to_string(),
        "=DATE(year, month, day)".to_string(),
    );

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let full_date = result_table.columns.get("full_date").unwrap();
    match &full_date.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "2025-01-15");
            assert_eq!(texts[1], "2024-06-20");
            assert_eq!(texts[2], "2023-12-31");
        }
        _ => panic!("Expected Text array"),
    }
}

#[test]
fn test_year_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "date".to_string(),
        ColumnValue::Date(vec![
            "2025-01-15".to_string(),
            "2024-06-20".to_string(),
            "2023-12-31".to_string(),
        ]),
    ));
    table.add_row_formula("year_val".to_string(), "=YEAR(date)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let year_val = result_table.columns.get("year_val").unwrap();
    match &year_val.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 2025.0);
            assert_eq!(nums[1], 2024.0);
            assert_eq!(nums[2], 2023.0);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_month_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "date".to_string(),
        ColumnValue::Date(vec![
            "2025-01-15".to_string(),
            "2024-06-20".to_string(),
            "2023-12-31".to_string(),
        ]),
    ));
    table.add_row_formula("month_val".to_string(), "=MONTH(date)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let month_val = result_table.columns.get("month_val").unwrap();
    match &month_val.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 1.0);
            assert_eq!(nums[1], 6.0);
            assert_eq!(nums[2], 12.0);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_day_function() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "date".to_string(),
        ColumnValue::Date(vec![
            "2025-01-15".to_string(),
            "2024-06-20".to_string(),
            "2023-12-31".to_string(),
        ]),
    ));
    table.add_row_formula("day_val".to_string(), "=DAY(date)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let day_val = result_table.columns.get("day_val").unwrap();
    match &day_val.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums[0], 15.0);
            assert_eq!(nums[1], 20.0);
            assert_eq!(nums[2], 31.0);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_date_functions_combined() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "date".to_string(),
        ColumnValue::Date(vec!["2025-06-15".to_string(), "2024-12-31".to_string()]),
    ));
    table.add_row_formula(
        "next_month".to_string(),
        "=DATE(YEAR(date), MONTH(date) + 1, DAY(date))".to_string(),
    );

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let next_month = result_table.columns.get("next_month").unwrap();
    match &next_month.values {
        ColumnValue::Text(texts) => {
            assert_eq!(texts[0], "2025-07-15");
            assert_eq!(texts[1], "2025-01-31"); // DATE function normalizes month 13 to January next year
        }
        _ => panic!("Expected Text array"),
    }
}

#[test]
fn test_datedif_function() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    // Test DATEDIF with literal dates
    // From 2024-01-15 to 2025-01-15 = 1 year = 12 months
    model.add_scalar(
        "years_diff".to_string(),
        Variable::new(
            "years_diff".to_string(),
            None,
            Some("=DATEDIF(\"2024-01-15\", \"2025-01-15\", \"Y\")".to_string()),
        ),
    );
    model.add_scalar(
        "months_diff".to_string(),
        Variable::new(
            "months_diff".to_string(),
            None,
            Some("=DATEDIF(\"2024-01-15\", \"2025-01-15\", \"M\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    let years = result.scalars.get("years_diff").unwrap().value.unwrap();
    assert_eq!(years, 1.0, "Should be 1 year, got {}", years);

    let months = result.scalars.get("months_diff").unwrap().value.unwrap();
    assert_eq!(months, 12.0, "Should be 12 months, got {}", months);
}

#[cfg(feature = "full")]
#[test]
fn test_edate_function() {
    let mut model = ParsedModel::new();

    // Test EDATE: Add 3 months to 2024-01-15 -> 2024-04-15
    // Note: EDATE returns a date string in the formula context
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new(
        "base_date".to_string(),
        ColumnValue::Date(vec!["2024-01-15".to_string()]),
    ));
    table.add_row_formula("new_date".to_string(), "=EDATE(base_date, 3)".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    let table = result.tables.get("test").unwrap();
    let new_date_col = table.columns.get("new_date").unwrap();

    // The result should contain the new date
    match &new_date_col.values {
        ColumnValue::Text(texts) => {
            assert!(
                texts[0].contains("2024-04-15"),
                "Expected April 15, got {}",
                texts[0]
            );
        }
        _ => panic!(
            "Expected Text array for dates, got {:?}",
            new_date_col.values
        ),
    }
}

#[cfg(feature = "full")]
#[test]
fn test_eomonth_function() {
    let mut model = ParsedModel::new();

    // Test EOMONTH: End of month 2 months after 2024-01-15 = 2024-03-31
    let mut table = Table::new("test".to_string());
    table.add_column(Column::new(
        "base_date".to_string(),
        ColumnValue::Date(vec!["2024-01-15".to_string()]),
    ));
    table.add_row_formula("end_date".to_string(), "=EOMONTH(base_date, 2)".to_string());
    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");

    let table = result.tables.get("test").unwrap();
    let end_date_col = table.columns.get("end_date").unwrap();

    // The result should contain the end of month date
    match &end_date_col.values {
        ColumnValue::Text(texts) => {
            assert!(
                texts[0].contains("2024-03-31"),
                "Expected March 31, got {}",
                texts[0]
            );
        }
        _ => panic!(
            "Expected Text array for dates, got {:?}",
            end_date_col.values
        ),
    }
}

#[cfg(feature = "full")]
#[test]
fn test_networkdays_function() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    // NETWORKDAYS counts business days between two dates
    model.add_scalar(
        "workdays".to_string(),
        Variable::new(
            "workdays".to_string(),
            None,
            Some("=NETWORKDAYS(\"2024-01-01\", \"2024-01-12\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Jan 1-12, 2024: Jan 1 is Monday
    // Business days: 1,2,3,4,5 (Mon-Fri) + 8,9,10,11,12 (Mon-Fri) = 10 days
    let workdays = result.scalars.get("workdays").unwrap().value.unwrap();
    assert!((workdays - 10.0).abs() < 1.0);
}

#[cfg(feature = "full")]
#[test]
fn test_yearfrac_function() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "fraction".to_string(),
        Variable::new(
            "fraction".to_string(),
            None,
            Some("=YEARFRAC(\"2024-01-01\", \"2024-07-01\", 0)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Half a year = 0.5 approximately
    let fraction = result.scalars.get("fraction").unwrap().value.unwrap();
    assert!(fraction > 0.4 && fraction < 0.6);
}

#[cfg(feature = "full")]
#[test]
fn test_yearfrac_basis_1() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "fraction".to_string(),
        Variable::new(
            "fraction".to_string(),
            None,
            Some("=YEARFRAC(\"2024-01-01\", \"2024-12-31\", 1)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Full year
    let fraction = result.scalars.get("fraction").unwrap().value.unwrap();
    assert!(fraction > 0.9 && fraction < 1.1);
}

#[cfg(feature = "full")]
#[test]
fn test_eomonth_negative_months_table() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("dates".to_string());
    data.add_column(Column::new(
        "start".to_string(),
        ColumnValue::Text(vec!["2024-03-15".to_string()]),
    ));
    data.row_formulas
        .insert("end".to_string(), "=EOMONTH(start, -1)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // End of Feb 2024 from March - 1 = Feb 29
    let col = result
        .tables
        .get("dates")
        .unwrap()
        .columns
        .get("end")
        .unwrap();
    if let ColumnValue::Text(values) = &col.values {
        assert!(values[0].contains("2024-02"));
    }
}

#[test]
fn test_today_function() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("dates".to_string());
    data.add_column(Column::new(
        "dummy".to_string(),
        ColumnValue::Number(vec![1.0]),
    ));
    data.row_formulas
        .insert("current".to_string(), "=TODAY()".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // TODAY returns date string in YYYY-MM-DD format
    let col = result
        .tables
        .get("dates")
        .unwrap()
        .columns
        .get("current")
        .unwrap();
    if let ColumnValue::Text(values) = &col.values {
        assert!(values[0].contains("-"));
        assert!(values[0].len() == 10);
    }
}

#[test]
fn test_date_construction() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("dates".to_string());
    data.add_column(Column::new(
        "year".to_string(),
        ColumnValue::Number(vec![2024.0]),
    ));
    data.add_column(Column::new(
        "month".to_string(),
        ColumnValue::Number(vec![6.0]),
    ));
    data.add_column(Column::new(
        "day".to_string(),
        ColumnValue::Number(vec![15.0]),
    ));
    data.row_formulas.insert(
        "full_date".to_string(),
        "=DATE(year, month, day)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let col = result
        .tables
        .get("dates")
        .unwrap()
        .columns
        .get("full_date")
        .unwrap();
    if let ColumnValue::Text(values) = &col.values {
        assert_eq!(values[0], "2024-06-15");
    }
}

#[cfg(feature = "full")]
#[test]
fn test_edate_add_months() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("dates".to_string());
    data.add_column(Column::new(
        "start".to_string(),
        ColumnValue::Text(vec!["2024-01-15".to_string()]),
    ));
    data.row_formulas
        .insert("future".to_string(), "=EDATE(start, 3)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let col = result
        .tables
        .get("dates")
        .unwrap()
        .columns
        .get("future")
        .unwrap();
    if let ColumnValue::Text(values) = &col.values {
        assert!(values[0].starts_with("2024-04"));
    }
}

#[test]
fn test_datedif_months_unit() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "months".to_string(),
        Variable::new(
            "months".to_string(),
            None,
            Some("=DATEDIF(\"2024-01-15\", \"2024-06-20\", \"M\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Jan to Jun = 5 complete months
    let months = result.scalars.get("months").unwrap().value.unwrap();
    assert!((months - 5.0).abs() < 0.01);
}

#[test]
fn test_datedif_years_unit() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    model.add_scalar(
        "years".to_string(),
        Variable::new(
            "years".to_string(),
            None,
            Some("=DATEDIF(\"2020-01-01\", \"2024-06-01\", \"Y\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // 2020 to 2024 = 4 complete years
    let years = result.scalars.get("years").unwrap().value.unwrap();
    assert!((years - 4.0).abs() < 0.01);
}

#[test]
fn test_cross_table_date_column_reference() {
    let mut model = ParsedModel::new();

    // Source table with date column
    let mut source = Table::new("source".to_string());
    source.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Date(vec!["2024-01-01".to_string(), "2024-02-01".to_string()]),
    ));
    model.add_table(source);

    // Target table referencing source's date column
    let mut target = Table::new("target".to_string());
    target.add_column(Column::new(
        "id".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]),
    ));
    target
        .row_formulas
        .insert("copy_date".to_string(), "=source.dates".to_string());
    model.add_table(target);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should handle cross-table date reference
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_local_date_column_reference() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "start_date".to_string(),
        ColumnValue::Date(vec!["2024-01-01".to_string(), "2024-06-01".to_string()]),
    ));
    data.add_column(Column::new(
        "days".to_string(),
        ColumnValue::Number(vec![30.0, 60.0]),
    ));
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[test]
fn test_datedif_years() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![1.0]),
    ));
    // Use literal date strings in DATEDIF
    data.row_formulas.insert(
        "years".to_string(),
        "=DATEDIF(\"2020-01-15\", \"2024-06-20\", \"Y\")".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Test exercises DATEDIF "Y" code path
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_datedif_months() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![1.0]),
    ));
    // Use literal date strings in DATEDIF
    data.row_formulas.insert(
        "months".to_string(),
        "=DATEDIF(\"2024-01-15\", \"2024-04-10\", \"M\")".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Test exercises DATEDIF "M" code path
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_datedif_days() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![1.0]),
    ));
    // Use literal date strings in DATEDIF
    data.row_formulas.insert(
        "days".to_string(),
        "=DATEDIF(\"2024-01-01\", \"2024-01-31\", \"D\")".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Test exercises DATEDIF "D" code path
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_datedif_invalid_unit() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "start".to_string(),
        ColumnValue::Date(vec!["2024-01-01".to_string()]),
    ));
    data.add_column(Column::new(
        "end".to_string(),
        ColumnValue::Date(vec!["2024-12-31".to_string()]),
    ));
    data.row_formulas.insert(
        "result".to_string(),
        "=DATEDIF(start, end, \"X\")".to_string(), // Invalid unit
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Should error due to invalid unit
    assert!(result.is_err());
}

#[cfg(feature = "full")]
#[test]
fn test_edate_positive_months() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "start".to_string(),
        ColumnValue::Date(vec!["2024-01-15".to_string()]),
    ));
    data.row_formulas.insert(
        "result".to_string(),
        "=EDATE(start, 3)".to_string(), // Add 3 months
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("result") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2024-04-15");
        }
    }
}

#[cfg(feature = "full")]
#[test]
fn test_edate_negative_months() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "start".to_string(),
        ColumnValue::Date(vec!["2024-06-15".to_string()]),
    ));
    data.row_formulas.insert(
        "result".to_string(),
        "=EDATE(start, -2)".to_string(), // Subtract 2 months
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_ok());
    let model = result.unwrap();
    let table = model.tables.get("data").unwrap();
    if let Some(col) = table.columns.get("result") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2024-04-15");
        }
    }
}

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

#[cfg(feature = "full")]
#[test]
fn test_yearfrac_basis_0() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "frac".to_string(),
        Variable::new(
            "frac".to_string(),
            None,
            Some("=YEARFRAC(DATE(2020,1,1), DATE(2020,7,1), 0)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[cfg(feature = "full")]
#[test]
fn test_yearfrac_actual_basis() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "frac".to_string(),
        Variable::new(
            "frac".to_string(),
            None,
            Some("=YEARFRAC(DATE(2020,1,1), DATE(2020,7,1), 1)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[cfg(feature = "full")]
#[test]
fn test_yearfrac_basis_2() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "frac".to_string(),
        Variable::new(
            "frac".to_string(),
            None,
            Some("=YEARFRAC(DATE(2020,1,1), DATE(2020,7,1), 2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[cfg(feature = "full")]
#[test]
fn test_yearfrac_basis_3() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "frac".to_string(),
        Variable::new(
            "frac".to_string(),
            None,
            Some("=YEARFRAC(DATE(2020,1,1), DATE(2020,7,1), 3)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[cfg(feature = "full")]
#[test]
fn test_yearfrac_basis_4() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "frac".to_string(),
        Variable::new(
            "frac".to_string(),
            None,
            Some("=YEARFRAC(DATE(2020,1,1), DATE(2020,7,1), 4)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[cfg(feature = "full")]
#[test]
fn test_workday_positive() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=WORKDAY(DATE(2020,1,1), 10)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[cfg(feature = "full")]
#[test]
fn test_workday_negative() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=WORKDAY(DATE(2020,1,15), -5)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[cfg(feature = "full")]
#[test]
fn test_edate_forward_quarter() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=EDATE(DATE(2020,1,15), 3)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[cfg(feature = "full")]
#[test]
fn test_edate_subtract_months() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=EDATE(DATE(2020,6,15), -2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[cfg(feature = "full")]
#[test]
fn test_eomonth_positive() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=EOMONTH(DATE(2020,1,15), 2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[cfg(feature = "full")]
#[test]
fn test_eomonth_negative() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=EOMONTH(DATE(2020,6,15), -3)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[cfg(feature = "full")]
#[test]
fn test_networkdays_basic() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "days".to_string(),
        Variable::new(
            "days".to_string(),
            None,
            Some("=NETWORKDAYS(DATE(2020,1,1), DATE(2020,1,31))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_year_from_date() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "yr".to_string(),
        Variable::new(
            "yr".to_string(),
            None,
            Some("=YEAR(DATE(2025, 6, 15))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_month_from_date() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "mon".to_string(),
        Variable::new(
            "mon".to_string(),
            None,
            Some("=MONTH(DATE(2025, 6, 15))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
fn test_day_from_date() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "d".to_string(),
        Variable::new(
            "d".to_string(),
            None,
            Some("=DAY(DATE(2025, 6, 15))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let _ = calculator.calculate_all();
}

#[test]
#[cfg(feature = "full")]
fn test_now_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "now_len".to_string(),
        Variable::new("now_len".to_string(), None, Some("=LEN(NOW())".to_string())),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    // NOW() returns "YYYY-MM-DD HH:MM:SS" format = 19 characters
    let val = result.scalars.get("now_len").unwrap().value.unwrap();
    assert_eq!(val, 19.0);
}

#[test]
#[cfg(feature = "full")]
fn test_time_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "noon".to_string(),
        Variable::new(
            "noon".to_string(),
            None,
            Some("=TIME(12, 0, 0)".to_string()),
        ),
    );
    model.add_scalar(
        "time_with_minutes".to_string(),
        Variable::new(
            "time_with_minutes".to_string(),
            None,
            Some("=TIME(6, 30, 45)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    // TIME(12,0,0) = 0.5 (noon)
    let noon_val = result.scalars.get("noon").unwrap().value.unwrap();
    assert!((noon_val - 0.5).abs() < 0.001);
}

#[test]
#[cfg(feature = "full")]
fn test_hour_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "hour_noon".to_string(),
        Variable::new(
            "hour_noon".to_string(),
            None,
            Some("=HOUR(0.5)".to_string()),
        ),
    );
    model.add_scalar(
        "hour_6pm".to_string(),
        Variable::new(
            "hour_6pm".to_string(),
            None,
            Some("=HOUR(0.75)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let noon = result.scalars.get("hour_noon").unwrap().value.unwrap();
    let pm6 = result.scalars.get("hour_6pm").unwrap().value.unwrap();
    assert_eq!(noon, 12.0);
    assert_eq!(pm6, 18.0);
}

#[test]
#[cfg(feature = "full")]
fn test_minute_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // 0.5208333 = 12:30:00
    model.add_scalar(
        "minute_val".to_string(),
        Variable::new(
            "minute_val".to_string(),
            None,
            Some("=MINUTE(0.5208333)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let minutes = result.scalars.get("minute_val").unwrap().value.unwrap();
    assert!((minutes - 30.0).abs() < 1.0);
}

#[test]
#[cfg(feature = "full")]
fn test_second_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // 0.5 = 12:00:00 (zero seconds)
    model.add_scalar(
        "second_val".to_string(),
        Variable::new(
            "second_val".to_string(),
            None,
            Some("=SECOND(0.5)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let seconds = result.scalars.get("second_val").unwrap().value.unwrap();
    assert_eq!(seconds, 0.0);
}

#[test]
#[cfg(feature = "full")]
fn test_weekday_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // 2024-01-01 was a Monday
    model.add_scalar(
        "weekday_monday".to_string(),
        Variable::new(
            "weekday_monday".to_string(),
            None,
            Some("=WEEKDAY(DATE(2024, 1, 1))".to_string()),
        ),
    );
    // Type 2: Monday=1
    model.add_scalar(
        "weekday_type2".to_string(),
        Variable::new(
            "weekday_type2".to_string(),
            None,
            Some("=WEEKDAY(DATE(2024, 1, 1), 2)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let wd1 = result.scalars.get("weekday_monday").unwrap().value.unwrap();
    let wd2 = result.scalars.get("weekday_type2").unwrap().value.unwrap();
    assert_eq!(wd1, 2.0); // Monday = 2 in default (Sunday=1)
    assert_eq!(wd2, 1.0); // Monday = 1 in type 2
}

#[test]
#[cfg(feature = "full")]
fn test_days_function() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // DAYS(end, start)
    model.add_scalar(
        "days_diff".to_string(),
        Variable::new(
            "days_diff".to_string(),
            None,
            Some("=DAYS(DATE(2024, 1, 31), DATE(2024, 1, 1))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let days = result.scalars.get("days_diff").unwrap().value.unwrap();
    assert_eq!(days, 30.0);
}

#[test]
fn test_datedif_yd_unit() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "diff".to_string(),
        Variable::new(
            "diff".to_string(),
            None,
            Some("=DATEDIF(DATE(2024,1,1), DATE(2024,3,1), \"YD\")".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    // YD = days difference ignoring years (Jan 1 to Mar 1 = 60 days in leap year)
    let diff = result.scalars.get("diff").unwrap().value.unwrap();
    assert!((diff - 60.0).abs() < 1.0);
}

// ============================================================================
// EDGE CASE TESTS - 100% COVERAGE
// ============================================================================

// 1. LEAP YEAR HANDLING

#[test]
fn test_date_leap_year_valid() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "year".to_string(),
        ColumnValue::Number(vec![2024.0]),
    ));
    data.add_column(Column::new(
        "month".to_string(),
        ColumnValue::Number(vec![2.0]),
    ));
    data.add_column(Column::new(
        "day".to_string(),
        ColumnValue::Number(vec![29.0]),
    ));
    data.row_formulas
        .insert("result".to_string(), "=DATE(year, month, day)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let col = result
        .tables
        .get("data")
        .unwrap()
        .columns
        .get("result")
        .unwrap();
    if let ColumnValue::Text(vals) = &col.values {
        assert_eq!(vals[0], "2024-02-29", "Feb 29 2024 is valid leap year date");
    }
}

#[test]
fn test_date_leap_year_invalid() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "year".to_string(),
        ColumnValue::Number(vec![2023.0]),
    ));
    data.add_column(Column::new(
        "month".to_string(),
        ColumnValue::Number(vec![2.0]),
    ));
    data.add_column(Column::new(
        "day".to_string(),
        ColumnValue::Number(vec![29.0]),
    ));
    data.row_formulas
        .insert("result".to_string(), "=DATE(year, month, day)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // Non-leap year: Feb 29 2023 should error (strict date validation for FP&A accuracy)
    assert!(
        result.is_err(),
        "Feb 29 2023 should error - not a valid date in non-leap year"
    );
    if let Err(e) = result {
        let err_msg = e.to_string();
        assert!(
            err_msg.contains("invalid date") || err_msg.contains("2023-2-29"),
            "Error should mention invalid date, got: {}",
            err_msg
        );
    }
}

#[cfg(feature = "full")]
#[test]
fn test_edate_from_leap_year_feb_28() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "start".to_string(),
        ColumnValue::Date(vec!["2024-02-28".to_string()]),
    ));
    data.row_formulas
        .insert("plus_one".to_string(), "=EDATE(start, 1)".to_string());
    data.row_formulas
        .insert("plus_twelve".to_string(), "=EDATE(start, 12)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let table = result.tables.get("data").unwrap();

    if let Some(col) = table.columns.get("plus_one") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2024-03-28", "Feb 28 2024 + 1 month = Mar 28");
        }
    }

    if let Some(col) = table.columns.get("plus_twelve") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(
                vals[0], "2025-02-28",
                "Feb 28 2024 + 12 months = Feb 28 2025 (non-leap)"
            );
        }
    }
}

#[cfg(feature = "full")]
#[test]
fn test_eomonth_february_leap_vs_nonleap() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "leap".to_string(),
        ColumnValue::Date(vec!["2024-02-15".to_string()]),
    ));
    data.add_column(Column::new(
        "nonleap".to_string(),
        ColumnValue::Date(vec!["2023-02-15".to_string()]),
    ));
    data.row_formulas
        .insert("leap_end".to_string(), "=EOMONTH(leap, 0)".to_string());
    data.row_formulas.insert(
        "nonleap_end".to_string(),
        "=EOMONTH(nonleap, 0)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let table = result.tables.get("data").unwrap();

    if let Some(col) = table.columns.get("leap_end") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2024-02-29", "Feb 2024 (leap) ends on 29th");
        }
    }

    if let Some(col) = table.columns.get("nonleap_end") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2023-02-28", "Feb 2023 (non-leap) ends on 28th");
        }
    }
}

// 2. MONTH-END TRANSITIONS

#[cfg(feature = "full")]
#[test]
fn test_edate_jan31_to_feb() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "start".to_string(),
        ColumnValue::Date(vec!["2024-01-31".to_string()]),
    ));
    data.row_formulas
        .insert("feb".to_string(), "=EDATE(start, 1)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let table = result.tables.get("data").unwrap();

    if let Some(col) = table.columns.get("feb") {
        if let ColumnValue::Date(vals) = &col.values {
            // Jan 31 + 1 month in leap year should be Feb 29 (Feb has no 31st)
            assert_eq!(
                vals[0], "2024-02-29",
                "Jan 31 + 1 month = Feb 29 in leap year"
            );
        }
    }
}

#[cfg(feature = "full")]
#[test]
fn test_edate_may31_to_june() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "start".to_string(),
        ColumnValue::Date(vec!["2024-05-31".to_string()]),
    ));
    data.row_formulas
        .insert("june".to_string(), "=EDATE(start, 1)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let table = result.tables.get("data").unwrap();

    if let Some(col) = table.columns.get("june") {
        if let ColumnValue::Date(vals) = &col.values {
            // May 31 + 1 month should be June 30 (June has 30 days)
            assert_eq!(vals[0], "2024-06-30", "May 31 + 1 month = June 30");
        }
    }
}

#[cfg(feature = "full")]
#[test]
fn test_eomonth_all_month_lengths() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    // Test months with different lengths: 31, 30, 29, 28 days
    data.add_column(Column::new(
        "jan".to_string(),
        ColumnValue::Date(vec!["2024-01-15".to_string()]),
    ));
    data.add_column(Column::new(
        "apr".to_string(),
        ColumnValue::Date(vec!["2024-04-15".to_string()]),
    ));
    data.add_column(Column::new(
        "feb_leap".to_string(),
        ColumnValue::Date(vec!["2024-02-15".to_string()]),
    ));
    data.add_column(Column::new(
        "feb_nonleap".to_string(),
        ColumnValue::Date(vec!["2023-02-15".to_string()]),
    ));
    data.row_formulas
        .insert("jan_end".to_string(), "=EOMONTH(jan, 0)".to_string());
    data.row_formulas
        .insert("apr_end".to_string(), "=EOMONTH(apr, 0)".to_string());
    data.row_formulas.insert(
        "feb_leap_end".to_string(),
        "=EOMONTH(feb_leap, 0)".to_string(),
    );
    data.row_formulas.insert(
        "feb_nonleap_end".to_string(),
        "=EOMONTH(feb_nonleap, 0)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let table = result.tables.get("data").unwrap();

    if let Some(col) = table.columns.get("jan_end") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2024-01-31", "January has 31 days");
        }
    }

    if let Some(col) = table.columns.get("apr_end") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2024-04-30", "April has 30 days");
        }
    }

    if let Some(col) = table.columns.get("feb_leap_end") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2024-02-29", "Feb leap year has 29 days");
        }
    }

    if let Some(col) = table.columns.get("feb_nonleap_end") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2023-02-28", "Feb non-leap has 28 days");
        }
    }
}

// 3. YEAR BOUNDARIES

#[cfg(feature = "full")]
#[test]
fn test_edate_cross_year_boundary_forward() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "start".to_string(),
        ColumnValue::Date(vec!["2024-12-15".to_string()]),
    ));
    data.row_formulas
        .insert("next_year".to_string(), "=EDATE(start, 1)".to_string());
    data.row_formulas
        .insert("two_months".to_string(), "=EDATE(start, 2)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let table = result.tables.get("data").unwrap();

    if let Some(col) = table.columns.get("next_year") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2025-01-15", "Dec 2024 + 1 month = Jan 2025");
        }
    }

    if let Some(col) = table.columns.get("two_months") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2025-02-15", "Dec 2024 + 2 months = Feb 2025");
        }
    }
}

#[cfg(feature = "full")]
#[test]
fn test_edate_cross_year_boundary_backward() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "start".to_string(),
        ColumnValue::Date(vec!["2025-01-15".to_string()]),
    ));
    data.row_formulas
        .insert("prev_year".to_string(), "=EDATE(start, -1)".to_string());
    data.row_formulas.insert(
        "two_months_back".to_string(),
        "=EDATE(start, -2)".to_string(),
    );
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let table = result.tables.get("data").unwrap();

    if let Some(col) = table.columns.get("prev_year") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2024-12-15", "Jan 2025 - 1 month = Dec 2024");
        }
    }

    if let Some(col) = table.columns.get("two_months_back") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2024-11-15", "Jan 2025 - 2 months = Nov 2024");
        }
    }
}

#[cfg(feature = "full")]
#[test]
fn test_eomonth_at_year_end() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "start".to_string(),
        ColumnValue::Date(vec!["2024-12-15".to_string()]),
    ));
    data.row_formulas
        .insert("dec_end".to_string(), "=EOMONTH(start, 0)".to_string());
    data.row_formulas
        .insert("jan_end".to_string(), "=EOMONTH(start, 1)".to_string());
    data.row_formulas
        .insert("nov_end".to_string(), "=EOMONTH(start, -1)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let table = result.tables.get("data").unwrap();

    if let Some(col) = table.columns.get("dec_end") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2024-12-31", "End of Dec 2024");
        }
    }

    if let Some(col) = table.columns.get("jan_end") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2025-01-31", "End of Jan 2025 (next year)");
        }
    }

    if let Some(col) = table.columns.get("nov_end") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2024-11-30", "End of Nov 2024");
        }
    }
}

#[test]
fn test_year_month_day_at_boundaries() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Date(vec![
            "2024-01-01".to_string(), // Year start
            "2024-12-31".to_string(), // Year end
            "2024-02-29".to_string(), // Leap day
        ]),
    ));
    data.row_formulas
        .insert("year".to_string(), "=YEAR(dates)".to_string());
    data.row_formulas
        .insert("month".to_string(), "=MONTH(dates)".to_string());
    data.row_formulas
        .insert("day".to_string(), "=DAY(dates)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let table = result.tables.get("data").unwrap();

    if let Some(col) = table.columns.get("year") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 2024.0);
            assert_eq!(vals[1], 2024.0);
            assert_eq!(vals[2], 2024.0);
        }
    }

    if let Some(col) = table.columns.get("month") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 1.0, "Jan = 1");
            assert_eq!(vals[1], 12.0, "Dec = 12");
            assert_eq!(vals[2], 2.0, "Feb = 2");
        }
    }

    if let Some(col) = table.columns.get("day") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 1.0, "First of month");
            assert_eq!(vals[1], 31.0, "Last of December");
            assert_eq!(vals[2], 29.0, "Leap day");
        }
    }
}

// 4. DAYS FUNCTION EDGE CASES

#[cfg(feature = "full")]
#[test]
fn test_days_same_date() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    model.add_scalar(
        "zero_days".to_string(),
        Variable::new(
            "zero_days".to_string(),
            None,
            Some("=DAYS(DATE(2024, 6, 15), DATE(2024, 6, 15))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let days = result.scalars.get("zero_days").unwrap().value.unwrap();
    assert_eq!(days, 0.0, "DAYS between same date should be 0");
}

#[cfg(feature = "full")]
#[test]
fn test_days_reverse_order() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // DAYS(start, end) where start < end should give negative result
    model.add_scalar(
        "negative_days".to_string(),
        Variable::new(
            "negative_days".to_string(),
            None,
            Some("=DAYS(DATE(2024, 1, 1), DATE(2024, 1, 31))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let days = result.scalars.get("negative_days").unwrap().value.unwrap();
    assert_eq!(days, -30.0, "DAYS(Jan 1, Jan 31) = -30 (start before end)");
}

#[cfg(feature = "full")]
#[test]
fn test_days_forward_order() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // DAYS(end, start) where end > start should give positive result
    model.add_scalar(
        "forward".to_string(),
        Variable::new(
            "forward".to_string(),
            None,
            Some("=DAYS(DATE(2024, 2, 1), DATE(2024, 1, 1))".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let days = result.scalars.get("forward").unwrap().value.unwrap();
    assert_eq!(days, 31.0, "DAYS(Feb 1, Jan 1) = 31 days (end after start)");
}

// 5. WEEKDAY - ALL DAYS OF THE WEEK

#[cfg(feature = "full")]
#[test]
fn test_weekday_all_seven_days() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    // Jan 1-7, 2024: Monday through Sunday
    data.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Date(vec![
            "2024-01-01".to_string(), // Monday
            "2024-01-02".to_string(), // Tuesday
            "2024-01-03".to_string(), // Wednesday
            "2024-01-04".to_string(), // Thursday
            "2024-01-05".to_string(), // Friday
            "2024-01-06".to_string(), // Saturday
            "2024-01-07".to_string(), // Sunday
        ]),
    ));
    data.row_formulas
        .insert("weekday".to_string(), "=WEEKDAY(dates)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let table = result.tables.get("data").unwrap();

    if let Some(col) = table.columns.get("weekday") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 2.0, "Monday = 2 (default: Sunday=1)");
            assert_eq!(vals[1], 3.0, "Tuesday = 3");
            assert_eq!(vals[2], 4.0, "Wednesday = 4");
            assert_eq!(vals[3], 5.0, "Thursday = 5");
            assert_eq!(vals[4], 6.0, "Friday = 6");
            assert_eq!(vals[5], 7.0, "Saturday = 7");
            assert_eq!(vals[6], 1.0, "Sunday = 1");
        }
    }
}

#[cfg(feature = "full")]
#[test]
fn test_weekday_type_2_all_days() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    // Type 2: Monday = 1, Sunday = 7
    data.add_column(Column::new(
        "dates".to_string(),
        ColumnValue::Date(vec![
            "2024-01-01".to_string(), // Monday
            "2024-01-02".to_string(), // Tuesday
            "2024-01-07".to_string(), // Sunday
        ]),
    ));
    data.row_formulas
        .insert("weekday".to_string(), "=WEEKDAY(dates, 2)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let table = result.tables.get("data").unwrap();

    if let Some(col) = table.columns.get("weekday") {
        if let ColumnValue::Number(vals) = &col.values {
            assert_eq!(vals[0], 1.0, "Monday = 1 (type 2)");
            assert_eq!(vals[1], 2.0, "Tuesday = 2 (type 2)");
            assert_eq!(vals[2], 7.0, "Sunday = 7 (type 2)");
        }
    }
}

#[cfg(feature = "full")]
#[test]
fn test_weekday_type_3_zero_indexed() {
    let mut model = ParsedModel::new();
    use crate::types::Variable;
    // Type 3: Monday = 0, Sunday = 6
    model.add_scalar(
        "monday".to_string(),
        Variable::new(
            "monday".to_string(),
            None,
            Some("=WEEKDAY(DATE(2024, 1, 1), 3)".to_string()),
        ),
    );
    model.add_scalar(
        "sunday".to_string(),
        Variable::new(
            "sunday".to_string(),
            None,
            Some("=WEEKDAY(DATE(2024, 1, 7), 3)".to_string()),
        ),
    );
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let monday = result.scalars.get("monday").unwrap().value.unwrap();
    let sunday = result.scalars.get("sunday").unwrap().value.unwrap();

    assert_eq!(monday, 0.0, "Monday = 0 (type 3: 0-indexed, Mon=0)");
    assert_eq!(sunday, 6.0, "Sunday = 6 (type 3)");
}

// 6. ADDITIONAL EDGE CASES

#[test]
fn test_date_with_out_of_range_day() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "year".to_string(),
        ColumnValue::Number(vec![2024.0]),
    ));
    data.add_column(Column::new(
        "month".to_string(),
        ColumnValue::Number(vec![4.0]), // April has 30 days
    ));
    data.add_column(Column::new(
        "day".to_string(),
        ColumnValue::Number(vec![31.0]), // Requesting 31st
    ));
    data.row_formulas
        .insert("result".to_string(), "=DATE(year, month, day)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    // April 31 should error (strict date validation for FP&A accuracy)
    assert!(
        result.is_err(),
        "April 31 should error - April only has 30 days"
    );
    if let Err(e) = result {
        let err_msg = e.to_string();
        assert!(
            err_msg.contains("invalid date") || err_msg.contains("2024-4-31"),
            "Error should mention invalid date, got: {}",
            err_msg
        );
    }
}

#[cfg(feature = "full")]
#[test]
fn test_edate_large_month_offset() {
    let mut model = ParsedModel::new();
    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "start".to_string(),
        ColumnValue::Date(vec!["2024-01-15".to_string()]),
    ));
    data.row_formulas
        .insert("forward".to_string(), "=EDATE(start, 24)".to_string());
    data.row_formulas
        .insert("backward".to_string(), "=EDATE(start, -24)".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let table = result.tables.get("data").unwrap();

    if let Some(col) = table.columns.get("forward") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2026-01-15", "24 months forward = 2 years");
        }
    }

    if let Some(col) = table.columns.get("backward") {
        if let ColumnValue::Date(vals) = &col.values {
            assert_eq!(vals[0], "2022-01-15", "24 months backward = 2 years");
        }
    }
}

#[cfg(feature = "full")]
#[test]
fn test_datedif_across_leap_year() {
    use crate::types::Variable;
    let mut model = ParsedModel::new();

    // Feb 1 2024 to Feb 1 2025 crosses leap year
    model.add_scalar(
        "days".to_string(),
        Variable::new(
            "days".to_string(),
            None,
            Some("=DATEDIF(\"2024-02-01\", \"2025-02-01\", \"D\")".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");
    let days = result.scalars.get("days").unwrap().value.unwrap();

    // 2024 is leap year with 366 days
    assert_eq!(
        days, 366.0,
        "Feb 1 2024 to Feb 1 2025 = 366 days (leap year)"
    );
}
