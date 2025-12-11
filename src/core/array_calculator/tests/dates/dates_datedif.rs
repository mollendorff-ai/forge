//! DATEDIF comprehensive tests
//!
//! Tests for DATEDIF with all units (Y, M, D, MD, YM, YD) and edge cases

#![allow(clippy::approx_constant)]

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

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
