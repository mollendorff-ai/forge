//! Statistical functions: VAR, VARP, STDEV, STDEVP, MODE, PERCENTILE, QUARTILE, CORREL, LARGE, SMALL, RANK

use super::{collect_numeric_values, evaluate, paired_numbers, require_args, require_args_range};
use super::{EvalContext, EvalError, Expr, Value};
use crate::core::array_calculator::evaluator::number_or;
use crate::core::precision::same;
use num_traits::ToPrimitive;

/// Try to evaluate a statistical function. Returns None if function not recognized.
#[allow(clippy::too_many_lines)]
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
                let count = values.iter().filter(|&&w| same(w, v)).count();
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
            Value::Number(inclusive(&values, k))
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
                .trunc();
            if !(0.0..=4.0).contains(&quart) {
                return Err(EvalError::new("QUARTILE quart must be 0, 1, 2, 3, or 4"));
            }
            values.sort_by(f64::total_cmp);
            let k = quart / 4.0;
            let n = values.len();
            if n == 1 {
                return Ok(Some(Value::Number(values[0])));
            }
            Value::Number(inclusive(&values, k))
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
            // From 1 to n, so the value below it is at rank - 1 from 0. At
            // k = 1/(n+1) the product can round to just below 1
            // (0.9999999999999999 for 48 values), which is the first value.
            let rank = (k * (n + 1.0)).clamp(1.0, n);
            let lower = rank.floor();
            let frac = rank - lower;
            let below = lower.to_usize().unwrap_or(1);
            let low = values[below - 1];
            if frac == 0.0 {
                Value::Number(low)
            } else {
                Value::Number(low.mul_add(1.0 - frac, values[below] * frac))
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
            let (x_vals, y_vals) = paired_numbers(name, &args[0], &args[1], ctx)?;
            if x_vals.len() < 2 {
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
            // Excel rounds a fractional k up for LARGE (LARGE(x, 1.5) is the
            // second largest; checked in Excel, 2026-11-14) and down for
            // SMALL: LARGE(k) is SMALL(n + 1 - k) with the index cut
            let k = evaluate(&args[1], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("LARGE: k must be a number"))?;
            let Some(k) = rank_in(k.ceil(), values.len()) else {
                return Err(EvalError::new(format!(
                    "LARGE: k={k} out of range (1..{})",
                    values.len()
                )));
            };
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
                .ok_or_else(|| EvalError::new("SMALL: k must be a number"))?;
            let Some(k) = rank_in(k.trunc(), values.len()) else {
                return Err(EvalError::new(format!(
                    "SMALL: k={k} out of range (1..{})",
                    values.len()
                )));
            };
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
            // Descending for 0 (the default), ascending for any other whole
            // number: the order is cut to one, as Excel cuts it
            let descending = if args.len() > 2 {
                number_or(&args[2], ctx, 0.0)?.trunc() == 0.0
            } else {
                true
            };

            // 1 + the values above it (below it, in ascending order), as in
            // Excel; the number must be one of the values (#N/A). Sorting with
            // total_cmp put -0 before 0 and ranked 0 one place too high.
            // Found exactly, as an exact lookup finds a number (ADR-065)
            if !values
                .iter()
                .any(|&x| crate::core::precision::same(x, number))
            {
                return Err(EvalError::new("#N/A: RANK: value not found in array"));
            }
            let ahead = values
                .iter()
                .filter(|&&x| if descending { x > number } else { x < number })
                .count();
            Value::Number((ahead + 1) as f64)
        },

        _ => return Ok(None),
    };

    Ok(Some(result))
}

/// The value at rank `k * (n - 1)` of sorted values, counted from 0, between
/// the two either side of it (PERCENTILE.INC, QUARTILE.INC); `k` is from 0
/// to 1, so the rank is a place in the values.
fn inclusive(sorted: &[f64], k: f64) -> f64 {
    let pos = k * (sorted.len() - 1) as f64;
    let lower = pos.floor();
    let frac = pos - lower;
    let at = lower.to_usize().unwrap_or(0);
    if frac == 0.0 {
        sorted[at]
    } else {
        sorted[at].mul_add(1.0 - frac, sorted[at + 1] * frac)
    }
}

/// `k` as the place of one of `n` values, from 1 to `n` (LARGE, SMALL):
/// `None` for any other number, which is #NUM! in Excel.
fn rank_in(k: f64, n: usize) -> Option<usize> {
    k.to_usize().filter(|k| (1..=n).contains(k))
}

#[cfg(test)]
mod tests {

    /// RANK finds the number in the list as an exact lookup does, as
    /// `LibreOffice` 26.2 does (not checked in Excel; ADR-065)
    #[test]
    fn rank_finds_the_number_exactly() {
        use crate::core::array_calculator::evaluator::tests as h;
        let ctx = h::table(&[
            ("v", h::numbers(&[0.3, 0.3])),
            ("z", h::numbers(&[0.0, 5.0])),
        ]);
        h::fails_in("RANK(0.1 + 0.2, t.v)", &ctx, "#N/A");
        h::fails_in("RANK(1E-12, t.z)", &ctx, "#N/A");
        h::is_exactly_in("RANK(0, t.z)", &ctx, 2.0);
    }
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
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );

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
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );

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
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );

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
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );

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
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );

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
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );

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

    /// Tests of the functions added in 10.1: schedules, conveniences, percentiles.
    /// Their expected values were computed outside Forge (R, Python, by hand); the
    /// Gnumeric suite (`statistics_finance_batch.yaml`) checks the same functions
    /// against two spreadsheet engines.
    mod batch_10_1 {
        use crate::core::array_calculator::evaluator::tests as h;

        #[test]
        fn geomean_percentile_and_quartile_exclusive() {
            let ctx = h::table(&[("v", h::numbers(&[3.5, 1.2, 9.9, 4.4, 7.7, 2.1, 8.8, 6.6]))]);
            // exp(mean(log(v))) and quantile(v, p, type = 6) in R
            assert!(h::near(h::number_in("GEOMEAN(0.5, 2, 8)", &ctx), 2.0));
            assert!(h::near(
                h::number_in("PERCENTILE.EXC(t.v, 0.3)", &ctx),
                3.08
            ));
            assert!(h::near(h::number_in("QUARTILE.EXC(t.v, 1)", &ctx), 2.45));
            assert!(h::near(
                h::number_in("PERCENTILE.INC(t.v, 0.3)", &ctx),
                3.59
            ));
            assert!(h::near(h::number_in("QUARTILE.INC(t.v, 2)", &ctx), 5.5));
            assert!(h::error_in("GEOMEAN(1, 0, 2)", &ctx).contains("#NUM!"));
            // With 8 values k has to lie between 1/9 and 8/9
            assert!(h::error_in("PERCENTILE.EXC(t.v, 0.05)", &ctx).contains("#NUM!"));
            assert!(h::error_in("PERCENTILE.EXC(t.v, 0.95)", &ctx).contains("#NUM!"));
            assert!(h::error_in("QUARTILE.EXC(t.v, 0)", &ctx).contains("#NUM!"));
            assert!(h::error_in("QUARTILE.EXC(t.v, 4)", &ctx).contains("#NUM!"));
        }
    }

    /// The five answers Excel gave differently from Forge when 40 formulas were
    /// checked in Excel on 2026-11-14
    /// (`validation/gnumeric/tests/edge/edge_excel_checks.yaml`), and what
    /// follows from each rule.
    mod checked_in_excel {
        use crate::core::array_calculator::evaluator::tests as h;

        #[test]
        fn large_rounds_k_up_and_small_down() {
            // Excel: LARGE(.., 1.5) is 5 and SMALL(.., 1.5) is 1
            h::is_exactly("LARGE({5,1,3,3,8}, 1.5)", 5.0);
            h::is_exactly("SMALL({5,1,3,3,8}, 1.5)", 1.0);
            // Both engines: 1.2 is rounded up too, and 0.5 is the largest
            h::is_exactly("LARGE({5,1,3,3,8}, 1.2)", 5.0);
            h::is_exactly("LARGE({5,1,3,3,8}, 0.5)", 8.0);
            h::fails("LARGE({5,1,3,3,8}, 5.5)");
            h::fails("LARGE({5,1,3,3,8}, 0)");
        }
    }

    /// Fixes for what the generated cases found (ADR-062) that the engines cannot
    /// check: answers Forge gives on its own, with the reason, and the exact
    /// values where every engine loses digits.
    mod generated_cases {
        use crate::core::array_calculator::evaluator::tests as h;
        use crate::core::array_calculator::evaluator::{EvalContext, Value};

        #[test]
        fn rank_counts_the_values_above_exactly() {
            let mut ctx = EvalContext::new();
            let v: Vec<Value> = [629.65, 1.005, 1e-12, -0.0, 0.0, -0.93]
                .iter()
                .map(|n| Value::Number(*n))
                .collect();
            ctx.tables.insert(
                "t".to_string(),
                std::iter::once(("v".to_string(), v.into())).collect(),
            );
            assert_eq!(
                h::eval("RANK.EQ(0, t.v)", &ctx).unwrap(),
                Value::Number(4.0)
            );
            assert_eq!(
                h::eval("RANK.EQ(0, t.v, 1)", &ctx).unwrap(),
                Value::Number(2.0)
            );
        }
    }

    /// Each k is cut (SMALL), rounded up (LARGE) or cut to a quarter
    /// (QUARTILE) as Excel does, and one no place holds is out of range,
    /// named as it was written.
    #[test]
    fn a_k_is_a_place_in_the_values_or_none() {
        use crate::core::array_calculator::evaluator::tests as h;
        let ctx = h::table(&[("v", h::numbers(&[1.0, 2.0, 3.0, 4.0, 5.0]))]);
        h::is_exactly_in("LARGE(t.v, 0.5)", &ctx, 5.0);
        h::is_exactly_in("SMALL(t.v, 3.9)", &ctx, 3.0);
        assert!(h::error_in("LARGE(t.v, -0.5)", &ctx).contains("k=-0.5 out of range"));
        assert!(h::error_in("SMALL(t.v, 0.5)", &ctx).contains("k=0.5 out of range"));
        assert!(h::error_in("SMALL(t.v, 1E+20)", &ctx).contains("out of range (1..5)"));
        assert!(h::error_in("LARGE(t.v, 6)", &ctx).contains("out of range (1..5)"));
        h::is_exactly_in("QUARTILE(t.v, 4.9)", &ctx, 5.0);
        h::is_exactly_in("QUARTILE(t.v, -0.5)", &ctx, 1.0);
        assert!(h::error_in("QUARTILE(t.v, 1E+20)", &ctx).contains("0, 1, 2, 3, or 4"));
        assert!(h::error_in("QUARTILE(t.v, -1)", &ctx).contains("0, 1, 2, 3, or 4"));
        h::is_exactly_in("PERCENTILE(t.v, 0.25)", &ctx, 2.0);
        assert!(h::near(h::number_in("PERCENTILE(t.v, 0.3)", &ctx), 2.2));
        // The order is cut too: 0.5 is 0, descending; -1 ascending
        h::is_exactly_in("RANK(4, t.v, 0.5)", &ctx, 2.0);
        h::is_exactly_in("RANK(4, t.v, -1)", &ctx, 4.0);
    }

    /// k = 1/(n+1) is the first value, as Excel gives, but k * (n + 1)
    /// rounds to 0.9999999999999999 for 48 values: the rank was cut to 0 and
    /// the value below it read at 0 - 1, which crashed.
    #[test]
    fn percentile_exc_at_its_lowest_k_is_the_first_value() {
        use crate::core::array_calculator::evaluator::tests as h;
        h::is_exactly("PERCENTILE.EXC(SEQUENCE(48), 1/49)", 1.0);
        h::is_exactly("PERCENTILE.EXC(SEQUENCE(48), 48/49)", 48.0);
        h::is_exactly("PERCENTILE.EXC(SEQUENCE(5), 1/6)", 1.0);
        h::is_exactly("PERCENTILE.EXC(SEQUENCE(5), 5/6)", 5.0);
    }

    /// Text where a number goes, or an argument that is an error, is an error
    #[test]
    fn an_argument_that_is_no_number_fails() {
        use crate::core::array_calculator::evaluator::tests as h;
        let ctx = h::table(&[("v", h::numbers(&[1.0, 2.0, 3.0]))]);
        assert!(
            h::error_in("LARGE(t.v, \"x\")", &ctx).contains("k must be a number"),
            "LARGE(t.v, \"x\")"
        );
        assert!(
            h::error_in("SMALL(t.v, \"x\")", &ctx).contains("k must be a number"),
            "SMALL(t.v, \"x\")"
        );
        assert!(
            h::error_in("RANK(2, t.v, 1/0)", &ctx).contains(""),
            "RANK(2, t.v, 1/0)"
        );
    }
}
