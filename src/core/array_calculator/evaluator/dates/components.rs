//! Time component extraction functions: WEEKDAY, WEEKNUM, ISOWEEKNUM, HOUR, MINUTE, SECOND

use super::{
    evaluate, parse_date_value, require_args, require_args_range, Datelike, EvalContext, EvalError,
    Expr, Timelike, Value,
};
use crate::core::array_calculator::evaluator::number_or;

/// The week of the year, counted from January 1
fn week_number(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range(name, args, 1, 2)?;
    let date = parse_date_value(&evaluate(&args[0], ctx)?)?;
    #[allow(clippy::cast_possible_truncation)]
    let return_type = if args.len() > 1 {
        number_or(&args[1], ctx, 1.0)? as i32
    } else {
        1
    };
    // The day a week starts on (0 = Sunday); 21 is the ISO week
    let start = match return_type {
        1 | 17 => 0,
        2 | 11 => 1,
        12 => 2,
        13 => 3,
        14 => 4,
        15 => 5,
        16 => 6,
        21 => return Ok(Value::Number(f64::from(date.iso_week().week()))),
        _ => {
            return Err(EvalError::new(format!(
                "WEEKNUM: the return type {return_type} is not one of 1, 2, 11 to 17, 21 (#NUM!)"
            )))
        },
    };
    // Week 1 is the week with January 1, counted from the day it starts on
    let jan1 = date.with_ordinal(1).unwrap_or(date);
    let offset = (jan1.weekday().num_days_from_sunday() + 7 - start) % 7;
    Ok(Value::Number(f64::from((date.ordinal0() + offset) / 7 + 1)))
}

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
                number_or(&args[1], ctx, 1.0)? as i32
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
            // 11 to 17: the week starts on Monday (11) ... Sunday (17), which is 1
            let result = match return_type {
                1 => f64::from(day + 1),
                2 => f64::from((day + 6) % 7 + 1),
                3 => f64::from((day + 6) % 7),
                11..=17 => {
                    let first = (return_type - 10).cast_unsigned() % 7; // Sunday = 0
                    f64::from((day + 7 - first) % 7 + 1)
                },
                _ => {
                    return Err(EvalError::new(format!(
                        "#NUM!: WEEKDAY return_type must be 1, 2, 3 or 11 to 17, got {return_type}"
                    )))
                },
            };
            Value::Number(result)
        },

        "WEEKNUM" => week_number(name, args, ctx)?,

        "ISOWEEKNUM" => {
            require_args(name, args, 1)?;
            let date = parse_date_value(&evaluate(&args[0], ctx)?)?;
            Value::Number(f64::from(date.iso_week().week()))
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
