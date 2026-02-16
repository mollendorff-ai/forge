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

            let mut count = 0;
            let mut current = start_date;
            while current <= end_date {
                let weekday = current.weekday().num_days_from_monday();
                if weekday < 5 {
                    count += 1;
                }
                current = current.succ_opt().unwrap_or(current);
            }
            Value::Number(f64::from(count))
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
}
