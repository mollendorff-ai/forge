//! Statistical functions: VAR, STDEV, PERCENTILE, QUARTILE, CORREL

use super::{collect_numeric_values, evaluate, require_args, EvalContext, EvalError, Expr, Value};

/// Try to evaluate a statistical function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "VAR" | "VAR.S" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.len() < 2 {
                return Err(EvalError::new("VAR requires at least 2 values"));
            }
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let sum_sq: f64 = values.iter().map(|x| (x - mean).powi(2)).sum();
            Value::Number(sum_sq / (values.len() - 1) as f64)
        }

        "VAR.P" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("VAR.P requires at least 1 value"));
            }
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let sum_sq: f64 = values.iter().map(|x| (x - mean).powi(2)).sum();
            Value::Number(sum_sq / values.len() as f64)
        }

        "STDEV" | "STDEV.S" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.len() < 2 {
                return Err(EvalError::new("STDEV requires at least 2 values"));
            }
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let sum_sq: f64 = values.iter().map(|x| (x - mean).powi(2)).sum();
            let variance = sum_sq / (values.len() - 1) as f64;
            Value::Number(variance.sqrt())
        }

        "STDEV.P" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("STDEV.P requires at least 1 value"));
            }
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let sum_sq: f64 = values.iter().map(|x| (x - mean).powi(2)).sum();
            let variance = sum_sq / values.len() as f64;
            Value::Number(variance.sqrt())
        }

        "PERCENTILE" => {
            require_args(name, args, 2)?;
            let mut values = collect_numeric_values(&args[..1], ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("PERCENTILE of empty set"));
            }
            let k = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("PERCENTILE k must be a number"))?;
            if !(0.0..=1.0).contains(&k) {
                return Err(EvalError::new("PERCENTILE k must be between 0 and 1"));
            }
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let n = values.len();
            if n == 1 {
                return Ok(Some(Value::Number(values[0])));
            }
            let pos = k * (n - 1) as f64;
            let lower = pos.floor() as usize;
            let upper = pos.ceil() as usize;
            let frac = pos - lower as f64;
            if lower == upper {
                Value::Number(values[lower])
            } else {
                Value::Number(values[lower] * (1.0 - frac) + values[upper] * frac)
            }
        }

        "QUARTILE" => {
            require_args(name, args, 2)?;
            let mut values = collect_numeric_values(&args[..1], ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("QUARTILE of empty set"));
            }
            let quart = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("QUARTILE quart must be a number"))?
                as i32;
            if !(0..=4).contains(&quart) {
                return Err(EvalError::new("QUARTILE quart must be 0, 1, 2, 3, or 4"));
            }
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let k = quart as f64 / 4.0;
            let n = values.len();
            if n == 1 {
                return Ok(Some(Value::Number(values[0])));
            }
            let pos = k * (n - 1) as f64;
            let lower = pos.floor() as usize;
            let upper = pos.ceil() as usize;
            let frac = pos - lower as f64;
            if lower == upper {
                Value::Number(values[lower])
            } else {
                Value::Number(values[lower] * (1.0 - frac) + values[upper] * frac)
            }
        }

        "CORREL" => {
            require_args(name, args, 2)?;
            let x_vals = collect_numeric_values(&args[..1], ctx)?;
            let y_vals = collect_numeric_values(&args[1..2], ctx)?;
            if x_vals.len() != y_vals.len() || x_vals.len() < 2 {
                return Err(EvalError::new(
                    "CORREL requires two arrays of equal length >= 2",
                ));
            }
            let n = x_vals.len() as f64;
            let x_mean = x_vals.iter().sum::<f64>() / n;
            let y_mean = y_vals.iter().sum::<f64>() / n;
            let mut cov = 0.0;
            let mut var_x = 0.0;
            let mut var_y = 0.0;
            for (x, y) in x_vals.iter().zip(y_vals.iter()) {
                let dx = x - x_mean;
                let dy = y - y_mean;
                cov += dx * dy;
                var_x += dx * dx;
                var_y += dy * dy;
            }
            if var_x == 0.0 || var_y == 0.0 {
                return Err(EvalError::new("CORREL: zero variance"));
            }
            Value::Number(cov / (var_x.sqrt() * var_y.sqrt()))
        }

        _ => return Ok(None),
    };

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::super::tests::eval;
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_variance() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "data".to_string(),
            vec![
                Value::Number(2.0),
                Value::Number(4.0),
                Value::Number(4.0),
                Value::Number(4.0),
                Value::Number(5.0),
                Value::Number(5.0),
                Value::Number(7.0),
                Value::Number(9.0),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        // Sample variance
        let var_result = eval("VAR(t.data)", &ctx).unwrap();
        assert!(matches!(var_result, Value::Number(n) if (n - 4.571).abs() < 0.01));
    }

    #[test]
    fn test_stdev() {
        let ctx = EvalContext::new();
        let stdev_result = eval("STDEV(2, 4, 4, 4, 5, 5, 7, 9)", &ctx).unwrap();
        assert!(matches!(stdev_result, Value::Number(n) if (n - 2.138).abs() < 0.01));
    }

    #[test]
    fn test_percentile() {
        // 50th percentile of [1, 2, 3, 4, 5] is 3
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "data".to_string(),
            vec![
                Value::Number(1.0),
                Value::Number(2.0),
                Value::Number(3.0),
                Value::Number(4.0),
                Value::Number(5.0),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        assert_eq!(
            eval("PERCENTILE(t.data, 0.5)", &ctx).unwrap(),
            Value::Number(3.0)
        );
    }

    #[test]
    fn test_quartile() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "data".to_string(),
            vec![
                Value::Number(1.0),
                Value::Number(2.0),
                Value::Number(3.0),
                Value::Number(4.0),
                Value::Number(5.0),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        assert_eq!(
            eval("QUARTILE(t.data, 2)", &ctx).unwrap(),
            Value::Number(3.0)
        ); // Q2 = median
    }
}
