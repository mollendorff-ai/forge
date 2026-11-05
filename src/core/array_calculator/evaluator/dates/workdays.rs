//! Workday calculation functions: WORKDAY, NETWORKDAYS (enterprise only)

// Workday casts: f64 day counts to i64 (bounded by calendar range).
#![allow(clippy::cast_possible_truncation)]

use super::super::{
    evaluate, parse_date_value, require_args_range, EvalContext, EvalError, Expr, Value,
};
use chrono::Datelike;

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
                .as_number()
                .ok_or_else(|| EvalError::new("WORKDAY: days must be a number"))?
                as i32;

            let mut current = parse_date_value(&start)?;
            let direction = if days >= 0 { 1 } else { -1 };
            let mut remaining = days.abs();

            while remaining > 0 {
                current = if direction > 0 {
                    current.succ_opt().unwrap_or(current)
                } else {
                    current.pred_opt().unwrap_or(current)
                };
                let weekday = current.weekday().num_days_from_monday();
                if weekday < 5 {
                    // Monday-Friday
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
                let weekday = current.weekday().num_days_from_monday();
                if weekday < 5 {
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
}
