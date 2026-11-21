//! Date difference and fraction functions: DATEDIF, YEARFRAC, TIME

use super::{
    evaluate, parse_date_value, require_args, Datelike, EvalContext, EvalError, Expr, Value,
};
use crate::core::array_calculator::evaluator::{day_count, number_or};
use crate::core::precision;

use super::require_args_range;
use num_traits::ToPrimitive;

/// Try to evaluate a date difference function.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "DATEDIF" => eval_datedif(name, args, ctx)?,
        "YEARFRAC" => eval_yearfrac(name, args, ctx)?,
        "TIME" => eval_time(name, args, ctx)?,
        _ => return Ok(None),
    };

    Ok(Some(result))
}

/// Evaluate `DATEDIF`.
fn eval_datedif(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args(name, args, 3)?;
    let start = evaluate(&args[0], ctx)?;
    let end = evaluate(&args[1], ctx)?;
    let unit = evaluate(&args[2], ctx)?.as_text().to_uppercase();

    let start_date = parse_date_value(&start)?;
    let end_date = parse_date_value(&end)?;
    if start_date > end_date {
        return Err(EvalError::new(
            "#NUM!: DATEDIF: the start date is after the end date",
        ));
    }

    let result = match unit.as_str() {
        "D" => precision::whole((end_date - start_date).num_days()),
        "M" => {
            // Complete months between dates
            // (whole numbers, exact as floats)
            let years = f64::from(end_date.year() - start_date.year());
            let months = f64::from(end_date.month()) - f64::from(start_date.month());
            let mut total_months = years.mul_add(12.0, months);
            // Adjust if end day < start day (incomplete month)
            if end_date.day() < start_date.day() {
                total_months -= 1.0;
            }
            total_months.max(0.0)
        },
        "Y" => {
            // Complete years between dates
            let mut years = end_date.year() - start_date.year();
            // Check if we've reached the anniversary date
            if end_date.month() < start_date.month()
                || (end_date.month() == start_date.month() && end_date.day() < start_date.day())
            {
                years -= 1;
            }
            f64::from(years.max(0))
        },
        "MD" => {
            // Days between dates, ignoring months and years
            let mut day_diff = f64::from(end_date.day()) - f64::from(start_date.day());
            if day_diff < 0.0 {
                // Get days in the previous month
                let prev_month_date = if end_date.month() == 1 {
                    chrono::NaiveDate::from_ymd_opt(end_date.year() - 1, 12, 1)
                } else {
                    chrono::NaiveDate::from_ymd_opt(end_date.year(), end_date.month() - 1, 1)
                };
                let days_in_prev_month = prev_month_date
                    .and_then(|d| {
                        d.checked_add_months(chrono::Months::new(1))
                            .and_then(|next| next.pred_opt())
                    })
                    .map_or(30.0, |d| f64::from(d.day()));
                day_diff += days_in_prev_month;
            }
            day_diff
        },
        "YM" => {
            // Months between dates, ignoring years
            let mut month_diff = f64::from(end_date.month()) - f64::from(start_date.month());
            // An incomplete last month does not count, also across a year
            // end: 30 January to 1 January is 11 months
            if end_date.day() < start_date.day() {
                month_diff -= 1.0;
            }
            month_diff.rem_euclid(12.0)
        },
        "YD" => {
            // Days between dates, ignoring years
            use chrono::NaiveDate;

            // Find the most recent anniversary of start_date before or on end_date
            let anniversary_year = if end_date.month() > start_date.month()
                || (end_date.month() == start_date.month() && end_date.day() >= start_date.day())
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
                NaiveDate::from_ymd_opt(anniversary_year, start_date.month(), start_date.day())
            }
            .ok_or_else(|| EvalError::new("DATEDIF: invalid anniversary date"))?;

            precision::whole((end_date - anniversary).num_days())
        },
        _ => return Err(EvalError::new(format!("DATEDIF: unknown unit '{unit}'"))),
    };
    Ok(Value::Number(result))
}

/// Evaluate `YEARFRAC`.
fn eval_yearfrac(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range(name, args, 2, 3)?;
    let start = parse_date_value(&evaluate(&args[0], ctx)?)?;
    let end = parse_date_value(&evaluate(&args[1], ctx)?)?;
    let basis = if args.len() > 2 {
        number_or(&args[2], ctx, 0.0)?
    } else {
        0.0
    };
    let basis = day_count::Basis::from_number(basis, "YEARFRAC")?;
    Ok(Value::Number(day_count::yearfrac(start, end, basis)))
}

/// Evaluate `TIME`.
fn eval_time(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args(name, args, 3)?;
    let hour = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("TIME: hour must be a number"))?;
    let minute = evaluate(&args[1], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("TIME: minute must be a number"))?;
    let second = evaluate(&args[2], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("TIME: second must be a number"))?;

    // A fraction of a day, as Excel gives: each part cut to a whole
    // number, and past 24 hours it starts the day again (TIME(27, 0,
    // 0) is TIME(3, 0, 0)); a negative time is #NUM!
    let seconds = |part: f64, size: i64| part.to_i32().map(|n| i64::from(n) * size);
    let Some(total_seconds) = seconds(hour, 3600)
        .zip(seconds(minute, 60))
        .zip(seconds(second, 1))
        .map(|((h, m), s)| h + m + s)
    else {
        return Err(EvalError::new(format!(
            "#NUM!: TIME({hour}, {minute}, {second}) is past every time"
        )));
    };
    if total_seconds < 0 {
        return Err(EvalError::new(format!(
            "#NUM!: TIME({hour}, {minute}, {second}) is before midnight"
        )));
    }
    Ok(Value::Number(
        u32::try_from(total_seconds % 86_400).map_or(0.0, f64::from) / 86400.0,
    ))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use super::super::{EvalContext, Value};
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{ParsedModel, Variable};

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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("basis must be 0, 1, 2, 3 or 4"));
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
        assert_exact!(years, 1.0);

        let months = result.scalars.get("months_diff").unwrap().value.unwrap();
        assert_exact!(months, 12.0);
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

    /// An hour, minute or second is cut to a whole number (TIME(1.9, 0, 0)
    /// is 1:00, as in Excel; Gnumeric keeps the fraction). Their sum was an
    /// i32, which a million hours overflowed: Gnumeric gives the time of day
    /// it lands on. One no i32 holds is #NUM!, as in Excel (whose hours stop
    /// at 32767).
    #[test]
    fn a_time_counts_whole_parts_past_any_day() {
        use crate::core::array_calculator::evaluator::tests as h;
        h::is_exactly("TIME(1.9, 0, 0)", 3_600.0 / 86_400.0);
        h::is_exactly("TIME(1000000, 0, 0)", 57_600.0 / 86_400.0);
        h::is_exactly("TIME(0, 0, 2147483647)", 11_647.0 / 86_400.0);
        h::fails_with("TIME(1E+10, 0, 0)", "#NUM!");
        h::fails_with("TIME(0, -1E+10, 0)", "#NUM!");
        h::fails_with("TIME(0, 0, -1)", "before midnight");
    }

    /// Text where a number goes, or an argument that is an error, is an error
    #[test]
    fn an_argument_that_is_no_number_fails() {
        use crate::core::array_calculator::evaluator::tests as h;
        h::fails_with("TIME(\"x\", 0, 0)", "hour must be a number");
        h::fails_with("TIME(0, \"x\", 0)", "minute must be a number");
        h::fails_with("TIME(0, 0, \"x\")", "second must be a number");
        // An incomplete last month does not count (M), also across a year end (YM)
        h::is_exactly("DATEDIF(\"2020-01-31\", \"2020-03-01\", \"M\")", 1.0);
        h::is_exactly("DATEDIF(\"2020-01-30\", \"2021-01-01\", \"YM\")", 11.0);
    }

    /// An error in an argument is passed on, text that is no date is
    /// refused, and the start must not be after the end
    #[test]
    fn each_argument_is_checked() {
        use crate::core::array_calculator::evaluator::tests::{each_argument, fails_with};
        each_argument(
            "DATEDIF",
            &[
                ("\"2020-01-01\"", Some("Invalid date format")),
                ("\"2021-06-01\"", Some("Invalid date format")),
                ("\"D\"", None),
            ],
        );
        fails_with(
            "DATEDIF(\"2021-01-01\", \"2020-01-01\", \"D\")",
            "#NUM!: DATEDIF: the start date is after the end date",
        );
    }

    /// Before the anniversary a year is not complete, and the days since
    /// the last anniversary or the last month's day count across the turn
    /// of a year (`LibreOffice` Calc 24.2 gives each of these). From 29
    /// February the anniversary in a year without one is 28 February, as in
    /// Excel (`edge_excel_checks.yaml`), where `LibreOffice` takes 1 March
    #[test]
    fn anniversaries() {
        use crate::core::array_calculator::evaluator::tests as h;
        let datedif = |start: &str, end: &str, unit: &str| {
            h::number(&format!("DATEDIF(\"{start}\", \"{end}\", \"{unit}\")"))
        };
        assert_exact!(datedif("2020-06-15", "2023-03-01", "Y"), 2.0);
        assert_exact!(datedif("2022-12-20", "2023-01-05", "MD"), 16.0);
        assert_exact!(datedif("2020-03-10", "2023-03-15", "YD"), 5.0);
        assert_exact!(datedif("2020-06-15", "2023-03-01", "YD"), 259.0);
        assert_exact!(datedif("2020-02-29", "2024-03-01", "YD"), 1.0);
        assert_exact!(datedif("2020-02-29", "2023-03-01", "YD"), 1.0);
    }
}
