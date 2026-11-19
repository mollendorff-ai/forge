//! Workday calculation functions: WORKDAY, NETWORKDAYS

use super::super::{
    evaluate, parse_date_value, require_args_range, EvalContext, EvalError, Expr, Value,
};
use chrono::{Datelike, NaiveDate};
use num_traits::ToPrimitive;
use std::collections::BTreeSet;

/// The holidays argument of WORKDAY and NETWORKDAYS: a date, a day number or
/// a column of them. Empty cells are skipped, as Excel skips empty cells of
/// its range.
fn holidays(args: &[Expr], ctx: &EvalContext) -> Result<BTreeSet<NaiveDate>, EvalError> {
    let mut found = BTreeSet::new();
    let Some(arg) = args.get(2) else {
        return Ok(found);
    };
    let value = evaluate(arg, ctx)?;
    let items = match value {
        Value::Array(items) => std::sync::Arc::unwrap_or_clone(items),
        other => vec![other],
    };
    for item in &items {
        if matches!(item, Value::Null) {
            continue;
        }
        found.insert(parse_date_value(item)?);
    }
    Ok(found)
}

fn is_workday(date: NaiveDate, holidays: &BTreeSet<NaiveDate>) -> bool {
    date.weekday().num_days_from_monday() < 5 && !holidays.contains(&date)
}

/// Try to evaluate a workday function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "WORKDAY" => {
            require_args_range(name, args, 2, 3)?;
            let start = evaluate(&args[0], ctx)?;
            let days = evaluate(&args[1], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("WORKDAY: days must be a number"))?;

            let mut current = parse_date_value(&start)?;
            let holidays = holidays(args, ctx)?;
            // Whole days, as Excel cuts them. Counting past 9999-12-31, or
            // back before 1900-01-01, leaves the dates Excel holds (#NUM!, as
            // in Gnumeric), and ends a count no calendar has room for.
            let past = || {
                EvalError::new(format!(
                    "#NUM!: WORKDAY: {days} workdays from {start_date} is past the dates Excel holds",
                    start_date = start.as_text()
                ))
            };
            let first = NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
            let last = NaiveDate::from_ymd_opt(9999, 12, 31).unwrap();
            let forward = days > 0.0;
            let mut remaining = days.to_i64().ok_or_else(past)?.unsigned_abs();

            while remaining > 0 {
                current = if forward {
                    current.succ_opt().unwrap_or(current)
                } else {
                    current.pred_opt().unwrap_or(current)
                };
                if (forward && current > last) || (!forward && current < first) {
                    return Err(past());
                }
                if is_workday(current, &holidays) {
                    remaining -= 1;
                }
            }

            Value::Text(current.format("%Y-%m-%d").to_string())
        },

        "NETWORKDAYS" => {
            require_args_range(name, args, 2, 3)?;
            let start = evaluate(&args[0], ctx)?;
            let end = evaluate(&args[1], ctx)?;

            let start_date = parse_date_value(&start)?;
            let end_date = parse_date_value(&end)?;
            let holidays = holidays(args, ctx)?;

            // Excel: a start date after the end date counts the same days and
            // returns the result negated.
            let (first, last, sign) = if start_date <= end_date {
                (start_date, end_date, 1.0)
            } else {
                (end_date, start_date, -1.0)
            };
            let mut count = 0;
            let mut current = first;
            while current <= last {
                if is_workday(current, &holidays) {
                    count += 1;
                }
                current = current.succ_opt().unwrap_or(current);
            }
            Value::Number(sign * f64::from(count))
        },

        _ => return Ok(None),
    };

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{ParsedModel, Variable};

    #[test]
    fn test_workday_function() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=WORKDAY(\"2024-01-01\", 5)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_networkdays_function() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=NETWORKDAYS(\"2024-01-01\", \"2024-01-31\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    fn networkdays(formula: &str) -> f64 {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some(formula.to_string())),
        );
        let result = ArrayCalculator::new(model)
            .calculate_all()
            .expect("should calculate");
        result.scalars.get("result").unwrap().value.unwrap()
    }

    #[test]
    fn test_networkdays_counts_both_ends() {
        // Mon 2024-01-15 to Fri 2024-01-19 is five working days
        assert!((networkdays("=NETWORKDAYS(\"2024-01-15\", \"2024-01-19\")") - 5.0).abs() < 1e-9);
        // Same working day counts as one
        assert!((networkdays("=NETWORKDAYS(\"2024-01-15\", \"2024-01-15\")") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_networkdays_reverse_range_is_negative() {
        // Excel: start after end returns the negated count
        assert!((networkdays("=NETWORKDAYS(\"2024-01-22\", \"2024-01-15\")") + 6.0).abs() < 1e-9);
        assert!((networkdays("=NETWORKDAYS(\"2024-01-19\", \"2024-01-15\")") + 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_holidays_are_not_working_days() {
        // 2023-12-25 to 2024-01-12: 15 weekdays; 2024-01-01 is a Monday,
        // 2024-01-06 a Saturday (not counted twice)
        let n = |f: &str| networkdays(f);
        assert!(
            (n("=NETWORKDAYS(\"2023-12-25\", \"2024-01-12\", \"2024-01-01\")") - 14.0).abs() < 1e-9
        );
        assert!(
            (n("=NETWORKDAYS(\"2023-12-25\", \"2024-01-12\", \"2024-01-06\")") - 15.0).abs() < 1e-9
        );
        assert!(
            (n("=NETWORKDAYS(\"2024-01-12\", \"2023-12-25\", DATE(2024, 1, 1))") + 14.0).abs()
                < 1e-9
        );
        // WORKDAY steps over the holiday: Friday 2023-12-29 plus one working
        // day is Tuesday 2024-01-02 (day number 45293)
        assert!((n("=WORKDAY(\"2023-12-29\", 1, \"2024-01-01\")") - 45293.0).abs() < 1e-9);
    }

    /// Counting past 9999-12-31, or back before 1900-01-01, is #NUM!, as in
    /// Gnumeric: WORKDAY gave a day in the year 10000, or a day number below
    /// 1, and a count past every calendar ran for billions of days. The days
    /// are cut to a whole number.
    #[test]
    fn workday_stays_in_the_dates_excel_holds() {
        use crate::core::array_calculator::evaluator::tests as h;
        assert_eq!(h::text("WORKDAY(\"9999-12-30\", 1)"), "9999-12-31");
        assert_eq!(h::text("WORKDAY(\"1900-01-02\", -1)"), "1900-01-01");
        h::fails_with("WORKDAY(\"9999-12-30\", 5)", "#NUM!");
        h::fails_with("WORKDAY(\"1900-01-01\", -1)", "#NUM!");
        h::fails_with("WORKDAY(\"2020-01-01\", 1E+10)", "#NUM!");
        h::fails_with("WORKDAY(\"2020-01-01\", 1E+20)", "#NUM!");
        assert_eq!(h::text("WORKDAY(\"2020-01-01\", 2.9)"), "2020-01-03");
        assert_eq!(h::text("WORKDAY(\"2020-01-01\", -2.9)"), "2019-12-30");
        assert_eq!(h::text("WORKDAY(\"2020-01-04\", 0)"), "2020-01-04");
    }

    /// Text where a number goes, or an argument that is an error, is an error
    #[test]
    fn an_argument_that_is_no_number_fails() {
        use crate::core::array_calculator::evaluator::tests as h;
        h::fails_with("WORKDAY(\"2020-01-01\", \"x\")", "days must be a number");
    }
}
