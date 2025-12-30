//! Date difference and fraction functions: DATEDIF, YEARFRAC, TIME

use super::{
    evaluate, parse_date_value, require_args, Datelike, EvalContext, EvalError, Expr, Value,
};

use super::require_args_range;

/// Try to evaluate a date difference function.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "DATEDIF" => {
            require_args(name, args, 3)?;
            let start = evaluate(&args[0], ctx)?;
            let end = evaluate(&args[1], ctx)?;
            let unit = evaluate(&args[2], ctx)?.as_text().to_uppercase();

            let start_date = parse_date_value(&start)?;
            let end_date = parse_date_value(&end)?;

            let result = match unit.as_str() {
                "D" => (end_date - start_date).num_days() as f64,
                "M" => {
                    // Complete months between dates
                    let years = end_date.year() - start_date.year();
                    let months = end_date.month() as i32 - start_date.month() as i32;
                    let mut total_months = years * 12 + months;
                    // Adjust if end day < start day (incomplete month)
                    if end_date.day() < start_date.day() {
                        total_months -= 1;
                    }
                    total_months.max(0) as f64
                },
                "Y" => {
                    // Complete years between dates
                    let mut years = end_date.year() - start_date.year();
                    // Check if we've reached the anniversary date
                    if end_date.month() < start_date.month()
                        || (end_date.month() == start_date.month()
                            && end_date.day() < start_date.day())
                    {
                        years -= 1;
                    }
                    years.max(0) as f64
                },
                "MD" => {
                    // Days between dates, ignoring months and years
                    let mut day_diff = end_date.day() as i32 - start_date.day() as i32;
                    if day_diff < 0 {
                        // Get days in the previous month
                        let prev_month_date = if end_date.month() == 1 {
                            chrono::NaiveDate::from_ymd_opt(end_date.year() - 1, 12, 1)
                        } else {
                            chrono::NaiveDate::from_ymd_opt(
                                end_date.year(),
                                end_date.month() - 1,
                                1,
                            )
                        };
                        let days_in_prev_month = prev_month_date
                            .and_then(|d| {
                                d.checked_add_months(chrono::Months::new(1))
                                    .and_then(|next| next.pred_opt())
                            })
                            .map(|d| d.day() as i32)
                            .unwrap_or(30);
                        day_diff += days_in_prev_month;
                    }
                    day_diff as f64
                },
                "YM" => {
                    // Months between dates, ignoring years
                    let mut month_diff = end_date.month() as i32 - start_date.month() as i32;
                    if month_diff < 0 {
                        month_diff += 12;
                    }
                    // Adjust if end day < start day (incomplete month)
                    if end_date.day() < start_date.day() && month_diff > 0 {
                        month_diff -= 1;
                    }
                    month_diff as f64
                },
                "YD" => {
                    // Days between dates, ignoring years
                    use chrono::NaiveDate;

                    // Find the most recent anniversary of start_date before or on end_date
                    let anniversary_year = if end_date.month() > start_date.month()
                        || (end_date.month() == start_date.month()
                            && end_date.day() >= start_date.day())
                    {
                        end_date.year()
                    } else {
                        end_date.year() - 1
                    };

                    // Handle Feb 29 -> Feb 28 for non-leap years
                    let anniversary = if start_date.month() == 2 && start_date.day() == 29 {
                        NaiveDate::from_ymd_opt(anniversary_year, 2, 29)
                            .or_else(|| NaiveDate::from_ymd_opt(anniversary_year, 2, 28))
                    } else {
                        NaiveDate::from_ymd_opt(
                            anniversary_year,
                            start_date.month(),
                            start_date.day(),
                        )
                    }
                    .ok_or_else(|| EvalError::new("DATEDIF: invalid anniversary date"))?;

                    (end_date - anniversary).num_days() as f64
                },
                _ => return Err(EvalError::new(format!("DATEDIF: unknown unit '{unit}'"))),
            };
            Value::Number(result)
        },

        "YEARFRAC" => {
            require_args_range(name, args, 2, 3)?;
            let start = evaluate(&args[0], ctx)?;
            let end = evaluate(&args[1], ctx)?;
            let basis = if args.len() > 2 {
                evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0
            };

            let start_date = parse_date_value(&start)?;
            let end_date = parse_date_value(&end)?;

            let result = match basis {
                0 | 4 => {
                    // US 30/360 and European 30/360
                    let mut d1 = start_date.day() as i32;
                    let m1 = start_date.month() as i32;
                    let y1 = start_date.year() as i32;
                    let mut d2 = end_date.day() as i32;
                    let m2 = end_date.month() as i32;
                    let y2 = end_date.year() as i32;

                    if d1 == 31 {
                        d1 = 30;
                    }
                    if d2 == 31 && (d1 >= 30 || basis == 4) {
                        d2 = 30;
                    }

                    let days_30_360 = ((y2 - y1) * 360 + (m2 - m1) * 30 + (d2 - d1)) as f64;
                    days_30_360 / 360.0
                },
                1 => {
                    // Actual/actual
                    let days = (end_date - start_date).num_days() as f64;
                    let year = start_date.year();
                    let is_leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
                    let year_days = if is_leap { 366.0 } else { 365.0 };
                    days / year_days
                },
                2 => {
                    // Actual/360
                    let days = (end_date - start_date).num_days() as f64;
                    days / 360.0
                },
                3 => {
                    // Actual/365
                    let days = (end_date - start_date).num_days() as f64;
                    days / 365.0
                },
                _ => return Err(EvalError::new(format!("YEARFRAC: unknown basis {basis}"))),
            };
            Value::Number(result)
        },

        "TIME" => {
            require_args(name, args, 3)?;
            let hour = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("TIME: hour must be a number"))?
                as i32;
            let minute = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("TIME: minute must be a number"))?
                as i32;
            let second = evaluate(&args[2], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("TIME: second must be a number"))?
                as i32;

            // Return as fraction of day (Excel time serial)
            let total_seconds = hour * 3600 + minute * 60 + second;
            Value::Number(total_seconds as f64 / 86400.0)
        },

        _ => return Ok(None),
    };

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use super::super::{EvalContext, Value};
    use crate::core::array_calculator::ArrayCalculator;
    #[allow(unused_imports)]
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_datedif() {
        let ctx = EvalContext::new();
        // Days between Jan 1 and Jan 31 = 30
        assert_eq!(
            eval("DATEDIF(\"2024-01-01\", \"2024-01-31\", \"D\")", &ctx).unwrap(),
            Value::Number(30.0)
        );
    }

    #[test]
    fn test_datedif_all_units() {
        let ctx = EvalContext::new();
        // Test M (months)
        assert_eq!(
            eval("DATEDIF(\"2024-01-15\", \"2024-06-20\", \"M\")", &ctx).unwrap(),
            Value::Number(5.0)
        );
        // Test Y (years)
        assert_eq!(
            eval("DATEDIF(\"2022-01-01\", \"2024-01-01\", \"Y\")", &ctx).unwrap(),
            Value::Number(2.0)
        );
        // Test MD (day difference ignoring months)
        assert_eq!(
            eval("DATEDIF(\"2024-01-10\", \"2024-02-15\", \"MD\")", &ctx).unwrap(),
            Value::Number(5.0)
        );
        // Test YM (month difference ignoring years)
        assert_eq!(
            eval("DATEDIF(\"2022-03-01\", \"2024-08-01\", \"YM\")", &ctx).unwrap(),
            Value::Number(5.0)
        );
        // Test YD (day difference ignoring years)
        assert_eq!(
            eval("DATEDIF(\"2022-01-01\", \"2024-02-01\", \"YD\")", &ctx).unwrap(),
            Value::Number(31.0)
        );
    }

    #[test]
    fn test_datedif_negative_day_diff() {
        let ctx = EvalContext::new();
        // MD where end day < start day (uses actual days in previous month)
        assert_eq!(
            eval("DATEDIF(\"2024-01-20\", \"2024-02-10\", \"MD\")", &ctx).unwrap(),
            Value::Number(21.0)
        );
    }

    #[test]
    fn test_datedif_negative_month_diff() {
        let ctx = EvalContext::new();
        // YM where end month < start month
        assert_eq!(
            eval("DATEDIF(\"2022-10-01\", \"2024-03-01\", \"YM\")", &ctx).unwrap(),
            Value::Number(5.0)
        );
    }

    #[test]
    fn test_datedif_unknown_unit() {
        let ctx = EvalContext::new();
        let result = eval("DATEDIF(\"2024-01-01\", \"2024-12-31\", \"X\")", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown unit"));
    }

    #[test]
    fn test_yearfrac_all_bases() {
        let ctx = EvalContext::new();
        // Basis 1 (actual/actual)
        let result = eval("YEARFRAC(\"2024-01-01\", \"2024-07-01\", 1)", &ctx).unwrap();
        if let Value::Number(n) = result {
            assert!((n - 0.497).abs() < 0.01);
        }
        // Basis 2 (actual/360)
        let result = eval("YEARFRAC(\"2024-01-01\", \"2024-07-01\", 2)", &ctx).unwrap();
        if let Value::Number(n) = result {
            assert!((n - 0.506).abs() < 0.01);
        }
        // Basis 3 (actual/365)
        let result = eval("YEARFRAC(\"2024-01-01\", \"2024-07-01\", 3)", &ctx).unwrap();
        if let Value::Number(n) = result {
            assert!((n - 0.499).abs() < 0.01);
        }
        // Basis 4 (European 30/360)
        let result = eval("YEARFRAC(\"2024-01-01\", \"2024-07-01\", 4)", &ctx).unwrap();
        if let Value::Number(n) = result {
            assert!((n - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn test_yearfrac_unknown_basis() {
        let ctx = EvalContext::new();
        let result = eval("YEARFRAC(\"2024-01-01\", \"2024-07-01\", 5)", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown basis"));
    }

    #[test]
    fn test_time() {
        let ctx = EvalContext::new();
        // TIME(12, 0, 0) = 0.5 (noon is half a day)
        let result = eval("TIME(12, 0, 0)", &ctx).unwrap();
        if let Value::Number(n) = result {
            assert!((n - 0.5).abs() < 0.0001);
        } else {
            panic!("Expected number");
        }
    }

    #[test]
    fn test_datedif_function_basic() {
        let mut model = ParsedModel::new();

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
        assert_eq!(years, 1.0);

        let months = result.scalars.get("months_diff").unwrap().value.unwrap();
        assert_eq!(months, 12.0);
    }

    #[test]
    fn test_datedif_yd_unit() {
        let mut model = ParsedModel::new();
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
        let diff = result.scalars.get("diff").unwrap().value.unwrap();
        assert!((diff - 60.0).abs() < 1.0);
    }

    #[test]
    fn test_time_function_datedif() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "noon".to_string(),
            Variable::new(
                "noon".to_string(),
                None,
                Some("=TIME(12, 0, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let noon_val = result.scalars.get("noon").unwrap().value.unwrap();
        assert!((noon_val - 0.5).abs() < 0.001);
    }
}
