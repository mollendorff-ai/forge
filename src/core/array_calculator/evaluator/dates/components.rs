//! Time component extraction functions: WEEKDAY, HOUR, MINUTE, SECOND (enterprise only)

use super::{
    evaluate, parse_date_value, require_args, require_args_range, Datelike, EvalContext, EvalError,
    Expr, Timelike, Value,
};

/// Try to evaluate a time component function.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
// Time component casts: f64 date serials to i64 (bounded), i64 seconds to f64 (fits in mantissa).
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "WEEKDAY" => {
            require_args_range(name, args, 1, 2)?;
            let val = evaluate(&args[0], ctx)?;
            let return_type = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0) as i32
            } else {
                1
            };

            let date = parse_date_value(&val)?;
            // Monday = 0 in chrono
            let day = date.weekday().num_days_from_sunday();

            // Excel WEEKDAY return types:
            // 1 (default): Sunday=1, Saturday=7
            // 2: Monday=1, Sunday=7
            // 3: Monday=0, Sunday=6
            let result = match return_type {
                2 => f64::from((day + 6) % 7 + 1),
                3 => f64::from((day + 6) % 7),
                _ => f64::from(day + 1), // Type 1 (default): Sunday=1
            };
            Value::Number(result)
        },

        "HOUR" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            // Parse time from various formats
            let time_str = val.as_text();
            if let Some(time_part) = time_str.split(' ').nth(1) {
                // DateTime format "YYYY-MM-DD HH:MM:SS"
                if let Ok(parsed) = chrono::NaiveTime::parse_from_str(time_part, "%H:%M:%S") {
                    return Ok(Some(Value::Number(f64::from(parsed.hour()))));
                }
            }
            // Try as time only "HH:MM:SS"
            if let Ok(parsed) = chrono::NaiveTime::parse_from_str(&time_str, "%H:%M:%S") {
                return Ok(Some(Value::Number(f64::from(parsed.hour()))));
            }
            // Try as fraction of day (Excel serial time)
            if let Some(n) = val.as_number() {
                let frac = n.fract();
                let total_seconds = (frac * 86400.0).round() as i64;
                return Ok(Some(Value::Number((total_seconds / 3600) as f64)));
            }
            return Err(EvalError::new("HOUR: Could not parse time"));
        },

        "MINUTE" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            let time_str = val.as_text();
            if let Some(time_part) = time_str.split(' ').nth(1) {
                if let Ok(parsed) = chrono::NaiveTime::parse_from_str(time_part, "%H:%M:%S") {
                    return Ok(Some(Value::Number(f64::from(parsed.minute()))));
                }
            }
            if let Ok(parsed) = chrono::NaiveTime::parse_from_str(&time_str, "%H:%M:%S") {
                return Ok(Some(Value::Number(f64::from(parsed.minute()))));
            }
            if let Some(n) = val.as_number() {
                let frac = n.fract();
                let total_seconds = (frac * 86400.0).round() as i64;
                return Ok(Some(Value::Number(((total_seconds / 60) % 60) as f64)));
            }
            return Err(EvalError::new("MINUTE: Could not parse time"));
        },

        "SECOND" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            let time_str = val.as_text();
            if let Some(time_part) = time_str.split(' ').nth(1) {
                if let Ok(parsed) = chrono::NaiveTime::parse_from_str(time_part, "%H:%M:%S") {
                    return Ok(Some(Value::Number(f64::from(parsed.second()))));
                }
            }
            if let Ok(parsed) = chrono::NaiveTime::parse_from_str(&time_str, "%H:%M:%S") {
                return Ok(Some(Value::Number(f64::from(parsed.second()))));
            }
            if let Some(n) = val.as_number() {
                let frac = n.fract();
                let total_seconds = (frac * 86400.0).round() as i64;
                return Ok(Some(Value::Number((total_seconds % 60) as f64)));
            }
            return Err(EvalError::new("SECOND: Could not parse time"));
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
    use crate::types::{ParsedModel, Variable};

    #[test]
    fn test_weekday() {
        let ctx = EvalContext::new();
        // 2024-01-01 was a Monday
        // Type 1 (default): Sunday=1, Monday=2
        assert_eq!(
            eval("WEEKDAY(\"2024-01-01\")", &ctx).unwrap(),
            Value::Number(2.0)
        );
        // Type 2: Monday=1
        assert_eq!(
            eval("WEEKDAY(\"2024-01-01\", 2)", &ctx).unwrap(),
            Value::Number(1.0)
        );
        // Type 3: Monday=0
        assert_eq!(
            eval("WEEKDAY(\"2024-01-01\", 3)", &ctx).unwrap(),
            Value::Number(0.0)
        );
    }

    #[test]
    fn test_hour_minute_second() {
        let ctx = EvalContext::new();
        // Test with time string
        assert_eq!(
            eval("HOUR(\"14:30:45\")", &ctx).unwrap(),
            Value::Number(14.0)
        );
        assert_eq!(
            eval("MINUTE(\"14:30:45\")", &ctx).unwrap(),
            Value::Number(30.0)
        );
        assert_eq!(
            eval("SECOND(\"14:30:45\")", &ctx).unwrap(),
            Value::Number(45.0)
        );
    }

    #[test]
    fn test_hour_from_serial() {
        let ctx = EvalContext::new();
        // 0.5 = noon = 12:00
        assert_eq!(eval("HOUR(0.5)", &ctx).unwrap(), Value::Number(12.0));
        // 0.75 = 6pm = 18:00
        assert_eq!(eval("HOUR(0.75)", &ctx).unwrap(), Value::Number(18.0));
    }

    #[test]
    fn test_weekday_function_datedif() {
        let mut model = ParsedModel::new();
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
    fn test_hour_function_datedif() {
        let mut model = ParsedModel::new();
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
    fn test_minute_function_datedif() {
        let mut model = ParsedModel::new();
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
    fn test_second_function_datedif() {
        let mut model = ParsedModel::new();
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
}
