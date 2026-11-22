//! Date arithmetic and extraction functions: YEAR, MONTH, DAY, DATE, EDATE, EOMONTH, DAYS

use crate::core::precision;
use chrono::{Days, Months, NaiveDate};
use num_traits::ToPrimitive;

use super::{
    evaluate, parse_date_value, require_args, Datelike, EvalContext, EvalError, Expr, Value,
};

/// Try to evaluate a date arithmetic function.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "YEAR" => eval_year(name, args, ctx)?,
        "MONTH" => eval_month(name, args, ctx)?,
        "DAY" => eval_day(name, args, ctx)?,
        "DATE" => eval_date(name, args, ctx)?,
        "EDATE" => eval_edate(name, args, ctx)?,
        "EOMONTH" => eval_eomonth(name, args, ctx)?,
        "DAYS" => eval_days(name, args, ctx)?,
        _ => return Ok(None),
    };

    Ok(Some(result))
}

/// Evaluate `YEAR`.
fn eval_year(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args(name, args, 1)?;
    let val = evaluate(&args[0], ctx)?;
    let date = parse_date_value(&val)?;
    Ok(Value::Number(f64::from(date.year())))
}

/// Evaluate `MONTH`.
fn eval_month(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args(name, args, 1)?;
    let val = evaluate(&args[0], ctx)?;
    let date = parse_date_value(&val)?;
    Ok(Value::Number(f64::from(date.month())))
}

/// Evaluate `DAY`.
fn eval_day(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args(name, args, 1)?;
    let val = evaluate(&args[0], ctx)?;
    let date = parse_date_value(&val)?;
    Ok(Value::Number(f64::from(date.day())))
}

/// Evaluate `DATE`.
fn eval_date(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args(name, args, 3)?;
    let year = evaluate(&args[0], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("DATE: year must be a number"))?;
    let month = evaluate(&args[1], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("DATE: month must be a number"))?;
    let day = evaluate(&args[2], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("DATE: day must be a number"))?;

    // Excel cuts each to a whole number. A year from 0 to 1899 is that
    // many years after 1900 (DATE(99, 1, 1) is 1999-01-01); below 0 or
    // from 10000 is #NUM!
    let Some(whole_year) = year.to_i32().filter(|y| (0..10_000).contains(y)) else {
        return Err(EvalError::new(format!(
            "#NUM!: DATE year must be from 0 to 9999, got {year}"
        )));
    };
    let whole_year = if whole_year < 1900 {
        whole_year + 1900
    } else {
        whole_year
    };
    // The month and the day count on from January 1, either way:
    // month 0 is the December before, day 0 the month before's last
    // day, and 13 or 32 run into the next. One past an i32 is past
    // every calendar.
    let january = NaiveDate::from_ymd_opt(whole_year, 1, 1).unwrap();
    let date = month
        .to_i32()
        .and_then(|m| add_months(january, i64::from(m) - 1))
        .zip(day.to_i32())
        .and_then(|(first, d)| add_days(first, i64::from(d) - 1))
        .ok_or_else(|| {
            EvalError::new(format!(
                "#NUM!: DATE({year}, {month}, {day}) is past every date"
            ))
        })?;
    // A date, as EDATE and EOMONTH give (ADR-060): its day number
    // (days since 1899-12-30) wherever a number is read. Before
    // 1900-03-01, which Excel cannot hold as a date, the day number.
    let excel_base = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
    let first = NaiveDate::from_ymd_opt(1900, 3, 1).unwrap();
    if date > NaiveDate::from_ymd_opt(9999, 12, 31).unwrap() {
        return Err(EvalError::new(format!(
            "#NUM!: DATE gives {date}, after 9999-12-31, the last date Excel holds"
        )));
    }
    Ok(if date >= first {
        Value::Text(date.format("%Y-%m-%d").to_string())
    } else {
        Value::Number(precision::whole((date - excel_base).num_days()))
    })
}

/// Evaluate `EDATE`.
fn eval_edate(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args(name, args, 2)?;
    let start_date = evaluate(&args[0], ctx)?;
    let months = evaluate(&args[1], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("EDATE requires months as number"))?;

    let date = parse_date_value(&start_date)?;

    // Whole months, as Excel cuts them
    let result = months
        .to_i64()
        .and_then(|months| add_months(date, months))
        .ok_or_else(|| {
            EvalError::new(format!("#NUM!: EDATE: {months} months is past every date"))
        })?;

    Ok(Value::Text(result.format("%Y-%m-%d").to_string()))
}

/// Evaluate `EOMONTH`.
fn eval_eomonth(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args(name, args, 2)?;
    let start_date = evaluate(&args[0], ctx)?;
    let months = evaluate(&args[1], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new("EOMONTH requires months as number"))?;

    let date = parse_date_value(&start_date)?;

    let adjusted = months
        .to_i64()
        .and_then(|months| add_months(date, months))
        .ok_or_else(|| {
            EvalError::new(format!(
                "#NUM!: EOMONTH: {months} months is past every date"
            ))
        })?;

    // Get last day of that month
    let year = adjusted.year();
    let month = adjusted.month();
    let last_day = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).and_then(|d| d.pred_opt())
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).and_then(|d| d.pred_opt())
    }
    .ok_or_else(|| EvalError::new("EOMONTH: Invalid date result"))?;

    Ok(Value::Text(last_day.format("%Y-%m-%d").to_string()))
}

/// Evaluate `DAYS`.
fn eval_days(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args(name, args, 2)?;
    let end = evaluate(&args[0], ctx)?;
    let start = evaluate(&args[1], ctx)?;

    let end_date = parse_date_value(&end)?;
    let start_date = parse_date_value(&start)?;

    Ok(Value::Number(precision::whole(
        (end_date - start_date).num_days(),
    )))
}

/// `date` moved on by `months`, back for a negative count; `None` past the
/// calendar chrono holds.
fn add_months(date: NaiveDate, months: i64) -> Option<NaiveDate> {
    let count = Months::new(u32::try_from(months.unsigned_abs()).ok()?);
    if months < 0 {
        date.checked_sub_months(count)
    } else {
        date.checked_add_months(count)
    }
}

/// `date` moved on by `days`, back for a negative count; `None` past the
/// calendar chrono holds.
const fn add_days(date: NaiveDate, days: i64) -> Option<NaiveDate> {
    let count = Days::new(days.unsigned_abs());
    if days < 0 {
        date.checked_sub_days(count)
    } else {
        date.checked_add_days(count)
    }
}

#[cfg(test)]
mod tests {
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
        // DATE returns a date (ADR-060), whose day number (days since
        // 1899-12-30) is 45458 for 2024-06-15
        let date = eval("DATE(2024, 6, 15)", &ctx).unwrap();
        assert_eq!(date, Value::Text("2024-06-15".to_string()));
        assert_eq!(date.as_number(), Some(45458.0));
        // Before 1900-03-01 Excel holds no date: the day number, the days
        // since 1899-12-30 (Excel's own serial is a day less, without the 29
        // February 1900 it counts)
        assert_eq!(
            eval("DATE(1900, 1, 15)", &ctx).unwrap(),
            Value::Number(16.0)
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
            eval("DATE(2024, 14, 15)", &ctx).unwrap().as_number(),
            Some(45703.0)
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
        // A column of DATE is a column of dates, written as text (ADR-060)
        assert_eq!(
            full_date.values,
            ColumnValue::Text(
                ["2025-01-15", "2024-06-20", "2023-12-31"]
                    .map(String::from)
                    .to_vec()
            )
        );
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
        let nums = year_val.values.numbers().unwrap();
        assert_exact!(nums[0], 2025.0);
        assert_exact!(nums[1], 2024.0);
        assert_exact!(nums[2], 2023.0);
    }

    #[test]
    fn test_date_leap_year_feb_29() {
        let ctx = EvalContext::new();
        // Leap year: DATE(2024, 2, 29) should be valid
        let result = eval("DATE(2024, 2, 29)", &ctx).unwrap();
        assert_exact!(result.as_number().unwrap(), 45351.0);
    }

    #[test]
    fn test_date_non_leap_year_feb_29_rollover() {
        let ctx = EvalContext::new();
        // Non-leap year: DATE(2023, 2, 29) should roll to March 1
        let result = eval("DATE(2023, 2, 29)", &ctx).unwrap();
        assert_exact!(result.as_number().unwrap(), 44986.0);
    }

    #[test]
    fn test_date_month_zero_previous_year() {
        let ctx = EvalContext::new();
        // Month zero: DATE(2024, 0, 1) should go to previous year December
        let result = eval("DATE(2024, 0, 1)", &ctx).unwrap();
        assert_exact!(result.as_number().unwrap(), 45261.0);
    }

    #[test]
    fn test_date_day_zero_previous_month_last_day() {
        let ctx = EvalContext::new();
        // Day zero: DATE(2024, 1, 0) should go to previous month's last day
        let result = eval("DATE(2024, 1, 0)", &ctx).unwrap();
        assert_exact!(result.as_number().unwrap(), 45291.0);
    }

    #[test]
    fn test_eomonth_subtraction() {
        let ctx = EvalContext::new();
        let result = eval("EOMONTH(\"2024-01-15\", 0) - DATE(2024, 1, 1)", &ctx).unwrap();
        assert_eq!(result, Value::Number(30.0));
    }

    /// A month or a day count so far that no date has it was cut to the
    /// largest `i32`, then added to the year's months or negated, which
    /// overflowed: a crash in a debug build, a wrong date in a release one.
    /// Excel gives #NUM! for a date outside its calendar.
    #[test]
    fn a_month_or_a_day_past_every_calendar_is_num() {
        use crate::core::array_calculator::evaluator::tests as h;
        for formula in [
            "DATE(2020, 1E+10, 1)",
            "DATE(2020, -1E+10, 1)",
            "DATE(2020, 1, 1E+10)",
            "DATE(2020, 1, -1E+10)",
            "EDATE(\"2020-01-31\", 1E+10)",
            "EDATE(\"2020-01-31\", -1E+10)",
            "EOMONTH(\"2020-01-31\", 1E+10)",
            "EOMONTH(\"2020-01-31\", -1E+10)",
        ] {
            h::fails_with(formula, "#NUM!");
        }
    }

    /// A year, month, day or count of months is cut to a whole number, as
    /// Excel cuts it; month 13 and day 0 run into the next year and back.
    #[test]
    fn date_parts_are_whole_numbers() {
        use crate::core::array_calculator::evaluator::tests as h;
        assert_eq!(h::text("DATE(2020.9, 1.9, 1.9)"), "2020-01-01");
        assert_eq!(h::text("DATE(2020, 13, 0)"), "2020-12-31");
        assert_eq!(h::text("DATE(2020, 0, 1)"), "2019-12-01");
        assert_eq!(h::text("DATE(2020, 1, -0.5)"), "2019-12-31");
        // Year 0 is 1900, whose 1 January is before 1900-03-01: a day number
        h::is_exactly("DATE(-0.5, 1, 1)", 2.0);
        h::fails_with("DATE(1E+10, 1, 1)", "got 10000000000");
        h::fails_with("DATE(10000, 1, 1)", "#NUM!");
        assert_eq!(h::text("EDATE(\"2020-01-31\", 1.9)"), "2020-02-29");
        assert_eq!(h::text("EDATE(\"2020-03-31\", -1.9)"), "2020-02-29");
        assert_eq!(h::text("EOMONTH(\"2020-01-15\", 1.9)"), "2020-02-29");
        // Past what a u32 counts, and inside it but past the calendar
        h::fails_with("EDATE(\"2020-01-31\", 5E+9)", "#NUM!");
        h::fails_with("EDATE(\"2020-01-31\", 4E+9)", "#NUM!");
        h::fails_with("EOMONTH(\"2020-01-31\", 5E+9)", "#NUM!");
    }

    /// Text where a number goes, or an argument that is an error, is an error
    #[test]
    fn an_argument_that_is_no_number_fails() {
        use crate::core::array_calculator::evaluator::tests as h;
        h::fails_with("DATE(\"x\", 1, 1)", "year must be a number");
        h::fails_with("DATE(2020, \"x\", 1)", "month must be a number");
        h::fails_with("DATE(2020, 1, \"x\")", "day must be a number");
        h::fails_with("EDATE(\"2020-01-31\", \"x\")", "months as number");
        h::fails_with("EOMONTH(\"2020-01-31\", \"x\")", "months as number");
    }

    /// An error in any argument is passed on, and what is neither a date nor
    /// a number is refused
    #[test]
    fn each_argument_is_checked() {
        use crate::core::array_calculator::evaluator::tests::each_argument;
        let date = ("\"2024-01-31\"", Some("Invalid date format"));
        for name in ["YEAR", "MONTH", "DAY"] {
            each_argument(name, &[date]);
        }
        each_argument("DAYS", &[date, date]);
        each_argument(
            "DATE",
            &[
                ("2024", Some("DATE: year must be a number")),
                ("1", Some("DATE: month must be a number")),
                ("31", Some("DATE: day must be a number")),
            ],
        );
        each_argument(
            "EDATE",
            &[date, ("1", Some("EDATE requires months as number"))],
        );
        each_argument(
            "EOMONTH",
            &[date, ("1", Some("EOMONTH requires months as number"))],
        );
    }

    /// A date past 9999-12-31 is #NUM! from DATE, and the end of a month
    /// past the last date there is cannot be found
    #[test]
    fn past_the_last_date() {
        use crate::core::array_calculator::evaluator::tests::fails_with;
        use chrono::NaiveDate;
        fails_with(
            "DATE(9999, 13, 1)",
            "#NUM!: DATE gives +10000-01-01, after 9999-12-31, the last date Excel holds",
        );
        let base = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
        let last = (NaiveDate::MAX - base).num_days();
        fails_with(
            &format!("EOMONTH({last}, 0)"),
            "EOMONTH: Invalid date result",
        );
    }
}
