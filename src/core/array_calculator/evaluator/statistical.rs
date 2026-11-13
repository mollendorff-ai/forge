//! Statistical functions: VAR, VARP, STDEV, STDEVP, MODE, PERCENTILE, QUARTILE, CORREL, LARGE, SMALL, RANK

use super::{collect_numeric_values, evaluate, require_args, require_args_range};
use super::{EvalContext, EvalError, Expr, Value};
use crate::core::array_calculator::evaluator::number_or;

/// Try to evaluate a statistical function. Returns None if function not recognized.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]
// Spreadsheet arrays are bounded well within f64 mantissa (< 2^52 elements).
// Index/length casts between f64, usize, and i32 are safe for practical workloads.
// 10 statistical functions in a single dispatch — splitting would fragment related logic.
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
        },

        "VAR.P" | "VARP" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("VARP requires at least 1 value"));
            }
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let sum_sq: f64 = values.iter().map(|x| (x - mean).powi(2)).sum();
            Value::Number(sum_sq / values.len() as f64)
        },

        "STDEV" | "STDEV.S" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.len() < 2 {
                return Err(EvalError::new("STDEV requires at least 2 values"));
            }
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let sum_sq: f64 = values.iter().map(|x| (x - mean).powi(2)).sum();
            let variance = sum_sq / (values.len() - 1) as f64;
            Value::Number(variance.sqrt())
        },

        "STDEV.P" | "STDEVP" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("STDEVP requires at least 1 value"));
            }
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let sum_sq: f64 = values.iter().map(|x| (x - mean).powi(2)).sum();
            let variance = sum_sq / values.len() as f64;
            Value::Number(variance.sqrt())
        },

        "MODE" | "MODE.SNGL" => {
            let values = collect_numeric_values(args, ctx)?;
            // The most frequent value; the first one met when several tie.
            let mut best: Option<(f64, usize)> = None;
            for &v in &values {
                // Equal means exactly equal, as in Excel.
                #[allow(clippy::float_cmp)]
                let count = values.iter().filter(|&&w| w == v).count();
                if count > 1 && best.is_none_or(|(_, c)| count > c) {
                    best = Some((v, count));
                }
            }
            match best {
                Some((v, _)) => Value::Number(v),
                None => return Err(EvalError::new("MODE: no value occurs twice (#N/A)")),
            }
        },

        "PERCENTILE" | "PERCENTILE.INC" => {
            require_args(name, args, 2)?;
            let mut values = collect_numeric_values(&args[..1], ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("PERCENTILE of empty set"));
            }
            let k = evaluate(&args[1], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("PERCENTILE k must be a number"))?;
            if !(0.0..=1.0).contains(&k) {
                return Err(EvalError::new("PERCENTILE k must be between 0 and 1"));
            }
            values.sort_by(f64::total_cmp);
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
                Value::Number(values[lower].mul_add(1.0 - frac, values[upper] * frac))
            }
        },

        "QUARTILE" | "QUARTILE.INC" => {
            require_args(name, args, 2)?;
            let mut values = collect_numeric_values(&args[..1], ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("QUARTILE of empty set"));
            }
            let quart = evaluate(&args[1], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("QUARTILE quart must be a number"))?
                as i32;
            if !(0..=4).contains(&quart) {
                return Err(EvalError::new("QUARTILE quart must be 0, 1, 2, 3, or 4"));
            }
            values.sort_by(f64::total_cmp);
            let k = f64::from(quart) / 4.0;
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
                Value::Number(values[lower].mul_add(1.0 - frac, values[upper] * frac))
            }
        },

        "PERCENTILE.EXC" | "QUARTILE.EXC" => {
            require_args(name, args, 2)?;
            let mut values = collect_numeric_values(&args[..1], ctx)?;
            if values.is_empty() {
                return Err(EvalError::new(format!("{name} of empty set")));
            }
            let arg = evaluate(&args[1], ctx)?.as_cell_number().ok_or_else(|| {
                EvalError::new(format!("{name}: the second argument must be a number"))
            })?;
            // A quartile is the percentile of a quarter, three and no more
            let k = if name == "QUARTILE.EXC" {
                let quart = arg.trunc();
                if !(1.0..=3.0).contains(&quart) {
                    return Err(EvalError::new(
                        "QUARTILE.EXC quart must be 1, 2 or 3 (#NUM!)",
                    ));
                }
                quart / 4.0
            } else {
                arg
            };
            values.sort_by(f64::total_cmp);
            let n = values.len() as f64;
            // Rank k * (n + 1), counted from 1, so k must leave room for it
            if k < 1.0 / (n + 1.0) || k > n / (n + 1.0) {
                return Err(EvalError::new(format!(
                    "{name}: k must be between 1/(n+1) and n/(n+1) for {n} values (#NUM!)"
                )));
            }
            let rank = k * (n + 1.0);
            let lower = rank.floor();
            let frac = rank - lower;
            let low = values[lower as usize - 1];
            if frac == 0.0 {
                Value::Number(low)
            } else {
                Value::Number(low.mul_add(1.0 - frac, values[lower as usize] * frac))
            }
        },

        "GEOMEAN" => {
            let values = collect_numeric_values(args, ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("GEOMEAN of empty set"));
            }
            if values.iter().any(|v| *v <= 0.0) {
                return Err(EvalError::new("GEOMEAN needs numbers above 0 (#NUM!)"));
            }
            let mean_ln = values.iter().map(|v| v.ln()).sum::<f64>() / values.len() as f64;
            Value::Number(mean_ln.exp())
        },

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
                cov = dx.mul_add(dy, cov);
                var_x = dx.mul_add(dx, var_x);
                var_y = dy.mul_add(dy, var_y);
            }
            if var_x == 0.0 || var_y == 0.0 {
                return Err(EvalError::new("CORREL: zero variance"));
            }
            Value::Number(cov / (var_x.sqrt() * var_y.sqrt()))
        },

        "LARGE" => {
            require_args(name, args, 2)?;
            let mut values = collect_numeric_values(&args[..1], ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("LARGE: array is empty"));
            }
            let k = evaluate(&args[1], ctx)?
                .as_cell_number()
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
            values.sort_by(|a, b| b.total_cmp(a));
            Value::Number(values[k - 1])
        },

        "SMALL" => {
            require_args(name, args, 2)?;
            let mut values = collect_numeric_values(&args[..1], ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("SMALL: array is empty"));
            }
            let k = evaluate(&args[1], ctx)?
                .as_cell_number()
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
            values.sort_by(f64::total_cmp);
            Value::Number(values[k - 1])
        },

        "RANK" | "RANK.EQ" => {
            require_args_range(name, args, 2, 3)?;
            let number = evaluate(&args[0], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("RANK: number must be a number"))?;
            let values = collect_numeric_values(&args[1..2], ctx)?;
            if values.is_empty() {
                return Err(EvalError::new("RANK: array is empty"));
            }
            let order = if args.len() > 2 {
                number_or(&args[2], ctx, 0.0)? as i32
            } else {
                0 // Default: descending (0)
            };

            // Clone and sort
            let mut sorted = values;
            if order == 0 {
                // Descending order (largest = rank 1)
                sorted.sort_by(|a, b| b.total_cmp(a));
            } else {
                // Ascending order (smallest = rank 1)
                sorted.sort_by(f64::total_cmp);
            }

            // Find rank (1-based)
            let rank = sorted.iter().position(|&x| (x - number).abs() < 1e-10);
            match rank {
                Some(pos) => Value::Number((pos + 1) as f64),
                None => return Err(EvalError::new("RANK: value not found in array")),
            }
        },

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
        ctx.tables
            .insert("t".to_string(), table.into_iter().collect());

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
        ctx.tables
            .insert("t".to_string(), table.into_iter().collect());

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
        ctx.tables
            .insert("t".to_string(), table.into_iter().collect());

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
    fn test_mode() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("MODE(2, 4, 4, 4, 5, 5, 7, 9)", &ctx).unwrap(),
            Value::Number(4.0)
        );
        // A tie goes to the value met first, as in Excel.
        assert_eq!(eval("MODE(5, 1, 1, 5)", &ctx).unwrap(), Value::Number(5.0));
        assert_eq!(
            eval("MODE.SNGL(3, 3, 1)", &ctx).unwrap(),
            Value::Number(3.0)
        );
        // No value twice: an error, as Excel's #N/A.
        assert!(eval("MODE(1, 2, 3)", &ctx)
            .unwrap_err()
            .to_string()
            .contains("#N/A"));
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
        ctx.tables
            .insert("t".to_string(), table.into_iter().collect());

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
        ctx.tables
            .insert("t".to_string(), table.into_iter().collect());

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
        ctx.tables
            .insert("t".to_string(), table.into_iter().collect());

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
