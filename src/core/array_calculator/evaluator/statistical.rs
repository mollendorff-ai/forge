//! Statistical functions: VAR, VARP, STDEV, STDEVP, PERCENTILE, QUARTILE, CORREL, LARGE, SMALL, RANK

use super::{
    collect_numeric_values, evaluate, require_args, require_args_range, EvalContext, EvalError,
    Expr, Value,
};

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

        "VAR.P" | "VARP" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("VARP requires at least 1 value"));
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

        "STDEV.P" | "STDEVP" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("STDEVP requires at least 1 value"));
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

        "LARGE" => {
            require_args(name, args, 2)?;
            let mut values = collect_numeric_values(&args[..1], ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("LARGE: array is empty"));
            }
            let k = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("LARGE: k must be a number"))?
                as usize;
            if k == 0 || k > values.len() {
                return Err(EvalError::new(format!(
                    "LARGE: k={} out of range (1..{})",
                    k,
                    values.len()
                )));
            }
            // Sort descending and get k-th largest
            values.sort_by(|a, b| b.partial_cmp(a).unwrap());
            Value::Number(values[k - 1])
        }

        "SMALL" => {
            require_args(name, args, 2)?;
            let mut values = collect_numeric_values(&args[..1], ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("SMALL: array is empty"));
            }
            let k = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("SMALL: k must be a number"))?
                as usize;
            if k == 0 || k > values.len() {
                return Err(EvalError::new(format!(
                    "SMALL: k={} out of range (1..{})",
                    k,
                    values.len()
                )));
            }
            // Sort ascending and get k-th smallest
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            Value::Number(values[k - 1])
        }

        "RANK" | "RANK.EQ" => {
            require_args_range(name, args, 2, 3)?;
            let number = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("RANK: number must be a number"))?;
            let values = collect_numeric_values(&args[1..2], ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("RANK: array is empty"));
            }
            let order = if args.len() > 2 {
                evaluate(&args[2], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0 // Default: descending (0)
            };

            // Clone and sort
            let mut sorted = values.clone();
            if order == 0 {
                // Descending order (largest = rank 1)
                sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
            } else {
                // Ascending order (smallest = rank 1)
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            }

            // Find rank (1-based)
            let rank = sorted.iter().position(|&x| (x - number).abs() < 1e-10);
            match rank {
                Some(pos) => Value::Number((pos + 1) as f64),
                None => return Err(EvalError::new("RANK: value not found in array")),
            }
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

    #[test]
    fn test_varp_alias() {
        let ctx = EvalContext::new();
        // VARP should work same as VAR.P
        let varp_result = eval("VARP(2, 4, 4, 4, 5, 5, 7, 9)", &ctx).unwrap();
        assert!(matches!(varp_result, Value::Number(n) if (n - 4.0).abs() < 0.01));
    }

    #[test]
    fn test_stdevp_alias() {
        let ctx = EvalContext::new();
        // STDEVP should work same as STDEV.P
        let stdevp_result = eval("STDEVP(2, 4, 4, 4, 5, 5, 7, 9)", &ctx).unwrap();
        assert!(matches!(stdevp_result, Value::Number(n) if (n - 2.0).abs() < 0.01));
    }

    #[test]
    fn test_large() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "data".to_string(),
            vec![
                Value::Number(3.0),
                Value::Number(1.0),
                Value::Number(4.0),
                Value::Number(1.0),
                Value::Number(5.0),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        // LARGE(data, 1) = 5 (largest)
        assert_eq!(eval("LARGE(t.data, 1)", &ctx).unwrap(), Value::Number(5.0));
        // LARGE(data, 2) = 4 (second largest)
        assert_eq!(eval("LARGE(t.data, 2)", &ctx).unwrap(), Value::Number(4.0));
    }

    #[test]
    fn test_small() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "data".to_string(),
            vec![
                Value::Number(3.0),
                Value::Number(1.0),
                Value::Number(4.0),
                Value::Number(1.0),
                Value::Number(5.0),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        // SMALL(data, 1) = 1 (smallest)
        assert_eq!(eval("SMALL(t.data, 1)", &ctx).unwrap(), Value::Number(1.0));
        // SMALL(data, 2) = 1 (second smallest, also 1)
        assert_eq!(eval("SMALL(t.data, 2)", &ctx).unwrap(), Value::Number(1.0));
        // SMALL(data, 3) = 3 (third smallest)
        assert_eq!(eval("SMALL(t.data, 3)", &ctx).unwrap(), Value::Number(3.0));
    }

    #[test]
    fn test_rank() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "data".to_string(),
            vec![
                Value::Number(3.0),
                Value::Number(1.0),
                Value::Number(4.0),
                Value::Number(5.0),
                Value::Number(2.0),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        // RANK(5, data) = 1 (largest, descending default)
        assert_eq!(eval("RANK(5, t.data)", &ctx).unwrap(), Value::Number(1.0));
        // RANK(1, data) = 5 (smallest, descending)
        assert_eq!(eval("RANK(1, t.data)", &ctx).unwrap(), Value::Number(5.0));
        // RANK(1, data, 1) = 1 (smallest, ascending)
        assert_eq!(
            eval("RANK(1, t.data, 1)", &ctx).unwrap(),
            Value::Number(1.0)
        );
    }
}
