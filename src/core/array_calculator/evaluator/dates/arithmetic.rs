//! Date arithmetic and extraction functions: YEAR, MONTH, DAY, DATE, EDATE, EOMONTH, DAYS

use chrono::{Days, Months, NaiveDate};

use super::{
    evaluate, parse_date_value, require_args, Datelike, EvalContext, EvalError, Expr, Value,
};

/// Try to evaluate a date arithmetic function.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
// Date arithmetic casts f64 to i32 for year/month/day (bounded by calendar rules: months 1-12,
// days 1-31). chrono NaiveDate serial numbers fit in i32/i64. Splitting this match would
// fragment the date function dispatch table.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "YEAR" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            let date = parse_date_value(&val)?;
            Value::Number(f64::from(date.year()))
        },

        "MONTH" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            let date = parse_date_value(&val)?;
            Value::Number(f64::from(date.month()))
        },

        "DAY" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            let date = parse_date_value(&val)?;
            Value::Number(f64::from(date.day()))
        },

        "DATE" => {
            require_args(name, args, 3)?;
            let year = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DATE: year must be a number"))?
                as i32;
            let month = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DATE: month must be a number"))?
                as i32;
            let day = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DATE: day must be a number"))?
                as i32;

            // Handle month overflow/underflow (Excel-compatible behavior)
            let total_months = (year * 12 + month - 1) as i32;
            let adj_year = total_months.div_euclid(12);
            let adj_month = (total_months.rem_euclid(12) + 1) as u32;

            // Start from day 1 of the adjusted month, then add (day - 1) days
            // This handles day=0 (last day of prev month) and day>max (overflow to next month)
            let base_date = NaiveDate::from_ymd_opt(adj_year, adj_month, 1).ok_or_else(|| {
                EvalError::new(format!("DATE: invalid date {adj_year}-{adj_month}-1"))
            })?;
            let date = if day >= 1 {
                base_date
                    .checked_add_days(Days::new((day - 1) as u64))
                    .ok_or_else(|| EvalError::new("DATE: day overflow"))?
            } else {
                // day <= 0: go back from day 1
                base_date
                    .checked_sub_days(Days::new((1 - day) as u64))
                    .ok_or_else(|| EvalError::new("DATE: day underflow"))?
            };
            // Return Excel serial number (days since 1899-12-30)
            let excel_base = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
            Value::Number((date - excel_base).num_days() as f64)
        },

        "EDATE" => {
            require_args(name, args, 2)?;
            let start_date = evaluate(&args[0], ctx)?;
            let months = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("EDATE requires months as number"))?
                as i32;

            let date = parse_date_value(&start_date)?;

            let result = if months >= 0 {
                date.checked_add_months(Months::new(months as u32))
            } else {
                date.checked_sub_months(Months::new((-months) as u32))
            }
            .ok_or_else(|| EvalError::new("EDATE: Invalid date result"))?;

            Value::Text(result.format("%Y-%m-%d").to_string())
        },

        "EOMONTH" => {
            require_args(name, args, 2)?;
            let start_date = evaluate(&args[0], ctx)?;
            let months = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("EOMONTH requires months as number"))?
                as i32;

            let date = parse_date_value(&start_date)?;

            let adjusted = if months >= 0 {
                date.checked_add_months(Months::new(months as u32))
            } else {
                date.checked_sub_months(Months::new((-months) as u32))
            }
            .ok_or_else(|| EvalError::new("EOMONTH: Invalid date result"))?;

            // Get last day of that month
            let year = adjusted.year();
            let month = adjusted.month();
            let last_day = if month == 12 {
                NaiveDate::from_ymd_opt(year + 1, 1, 1).and_then(|d| d.pred_opt())
            } else {
                NaiveDate::from_ymd_opt(year, month + 1, 1).and_then(|d| d.pred_opt())
            }
            .ok_or_else(|| EvalError::new("EOMONTH: Invalid date result"))?;

            Value::Text(last_day.format("%Y-%m-%d").to_string())
        },

        "DAYS" => {
            require_args(name, args, 2)?;
            let end = evaluate(&args[0], ctx)?;
            let start = evaluate(&args[1], ctx)?;

            let end_date = parse_date_value(&end)?;
            let start_date = parse_date_value(&start)?;

            Value::Number((end_date - start_date).num_days() as f64)
        },

        _ => return Ok(None),
    };

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)] // Exact float comparison validated against Excel/Gnumeric/R
    use super::super::super::tests::eval;
    use super::super::{EvalContext, Value};
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table};

    #[test]
    fn test_date_parts() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("YEAR(\"2024-06-15\")", &ctx).unwrap(),
            Value::Number(2024.0)
        );
        assert_eq!(
            eval("MONTH(\"2024-06-15\")", &ctx).unwrap(),
            Value::Number(6.0)
        );
        assert_eq!(
            eval("DAY(\"2024-06-15\")", &ctx).unwrap(),
            Value::Number(15.0)
        );
    }

    #[test]
    fn test_date_construction() {
        let ctx = EvalContext::new();
        // DATE returns Excel serial number (days since 1899-12-30)
        // 2024-06-15 = 45458
        assert_eq!(
            eval("DATE(2024, 6, 15)", &ctx).unwrap(),
            Value::Number(45458.0)
        );
    }

    #[test]
    fn test_date_subtraction() {
        let ctx = EvalContext::new();
        // Full year (leap year 2024)
        assert_eq!(
            eval("DATE(2024, 12, 31) - DATE(2024, 1, 1)", &ctx).unwrap(),
            Value::Number(365.0)
        );
        // 30 days in January
        assert_eq!(
            eval("DATE(2024, 1, 31) - DATE(2024, 1, 1)", &ctx).unwrap(),
            Value::Number(30.0)
        );
    }

    #[test]
    fn test_date_month_overflow() {
        let ctx = EvalContext::new();
        // Month 14 = February of next year (2025-02-15 = serial 45703)
        assert_eq!(
            eval("DATE(2024, 14, 15)", &ctx).unwrap(),
            Value::Number(45703.0)
        );
    }

    #[test]
    fn test_edate() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("EDATE(\"2024-01-15\", 3)", &ctx).unwrap(),
            Value::Text("2024-04-15".to_string())
        );
    }

    #[test]
    fn test_edate_negative_months() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("EDATE(\"2024-04-15\", -3)", &ctx).unwrap(),
            Value::Text("2024-01-15".to_string())
        );
    }

    #[test]
    fn test_eomonth() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("EOMONTH(\"2024-01-15\", 0)", &ctx).unwrap(),
            Value::Text("2024-01-31".to_string())
        );
    }

    #[test]
    fn test_eomonth_december() {
        let ctx = EvalContext::new();
        // End of December (exercises month == 12 branch)
        assert_eq!(
            eval("EOMONTH(\"2024-12-15\", 0)", &ctx).unwrap(),
            Value::Text("2024-12-31".to_string())
        );
    }

    #[test]
    fn test_days() {
        let ctx = EvalContext::new();
        // DAYS(end, start) returns end - start
        assert_eq!(
            eval("DAYS(\"2024-01-31\", \"2024-01-01\")", &ctx).unwrap(),
            Value::Number(30.0)
        );
    }

    #[test]
    fn test_date_function_basic() {
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
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 45672.0); // 2025-01-15
                assert_eq!(nums[1], 45463.0); // 2024-06-20
                assert_eq!(nums[2], 45291.0); // 2023-12-31
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_year_function_basic() {
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
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_date_leap_year_feb_29() {
        let ctx = EvalContext::new();
        // Leap year: DATE(2024, 2, 29) should be valid
        let result = eval("DATE(2024, 2, 29)", &ctx).unwrap();
        if let Value::Number(serial) = result {
            assert_eq!(serial, 45351.0);
        } else {
            panic!("Expected Number from DATE");
        }
    }

    #[test]
    fn test_date_non_leap_year_feb_29_rollover() {
        let ctx = EvalContext::new();
        // Non-leap year: DATE(2023, 2, 29) should roll to March 1
        let result = eval("DATE(2023, 2, 29)", &ctx).unwrap();
        if let Value::Number(serial) = result {
            assert_eq!(serial, 44986.0);
        } else {
            panic!("Expected Number from DATE");
        }
    }

    #[test]
    fn test_date_month_zero_previous_year() {
        let ctx = EvalContext::new();
        // Month zero: DATE(2024, 0, 1) should go to previous year December
        let result = eval("DATE(2024, 0, 1)", &ctx).unwrap();
        if let Value::Number(serial) = result {
            assert_eq!(serial, 45261.0);
        } else {
            panic!("Expected Number from DATE");
        }
    }

    #[test]
    fn test_date_day_zero_previous_month_last_day() {
        let ctx = EvalContext::new();
        // Day zero: DATE(2024, 1, 0) should go to previous month's last day
        let result = eval("DATE(2024, 1, 0)", &ctx).unwrap();
        if let Value::Number(serial) = result {
            assert_eq!(serial, 45291.0);
        } else {
            panic!("Expected Number from DATE");
        }
    }

    #[test]
    fn test_eomonth_subtraction() {
        let ctx = EvalContext::new();
        let result = eval("EOMONTH(\"2024-01-15\", 0) - DATE(2024, 1, 1)", &ctx).unwrap();
        assert_eq!(result, Value::Number(30.0));
    }
}
