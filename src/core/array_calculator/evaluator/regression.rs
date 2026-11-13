//! Two arrays against each other: SLOPE, INTERCEPT, RSQ, FORECAST, FORECAST.LINEAR,
//! COVARIANCE.S and COVARIANCE.P

use super::{collect_numeric_values, evaluate, require_args};
use super::{EvalContext, EvalError, Expr, Value};

/// The sums a straight line through the points needs.
struct Fit {
    n: f64,
    mean_x: f64,
    mean_y: f64,
    /// Sum of (x - mean x)^2
    sxx: f64,
    /// Sum of (y - mean y)^2
    syy: f64,
    /// Sum of (x - mean x)(y - mean y)
    sxy: f64,
}

impl Fit {
    /// The points of two arrays given in the order `first`, `second`.
    fn of(
        name: &str,
        first: &[Expr],
        second: &[Expr],
        ctx: &EvalContext,
    ) -> Result<Self, EvalError> {
        let ys = collect_numeric_values(first, ctx)?;
        let xs = collect_numeric_values(second, ctx)?;
        if xs.len() != ys.len() {
            return Err(EvalError::new(format!(
                "{name} needs two arrays of the same size, got {} and {}",
                ys.len(),
                xs.len()
            )));
        }
        if xs.len() < 2 {
            return Err(EvalError::new(format!("{name} needs at least 2 points")));
        }
        let n = xs.len() as f64;
        let mean_x = xs.iter().sum::<f64>() / n;
        let mean_y = ys.iter().sum::<f64>() / n;
        let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
        for (x, y) in xs.iter().zip(&ys) {
            let (dx, dy) = (x - mean_x, y - mean_y);
            sxx = dx.mul_add(dx, sxx);
            syy = dy.mul_add(dy, syy);
            sxy = dx.mul_add(dy, sxy);
        }
        Ok(Self {
            n,
            mean_x,
            mean_y,
            sxx,
            syy,
            sxy,
        })
    }

    fn slope(&self, name: &str) -> Result<f64, EvalError> {
        if self.sxx == 0.0 {
            return Err(EvalError::new(format!(
                "{name}: the x values are all the same, so there is no line (#DIV/0!)"
            )));
        }
        Ok(self.sxy / self.sxx)
    }
}

/// Try to evaluate a function of two arrays. Returns None if not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "SLOPE" => {
            require_args(name, args, 2)?;
            let fit = Fit::of(name, &args[..1], &args[1..2], ctx)?;
            Value::Number(fit.slope(name)?)
        },

        "INTERCEPT" => {
            require_args(name, args, 2)?;
            let fit = Fit::of(name, &args[..1], &args[1..2], ctx)?;
            let slope = fit.slope(name)?;
            Value::Number(slope.mul_add(-fit.mean_x, fit.mean_y))
        },

        "RSQ" => {
            require_args(name, args, 2)?;
            let fit = Fit::of(name, &args[..1], &args[1..2], ctx)?;
            if fit.sxx == 0.0 || fit.syy == 0.0 {
                return Err(EvalError::new("RSQ: zero variance (#DIV/0!)"));
            }
            Value::Number((fit.sxy / fit.sxx) * (fit.sxy / fit.syy))
        },

        "FORECAST" | "FORECAST.LINEAR" => {
            require_args(name, args, 3)?;
            let x = evaluate(&args[0], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new(format!("{name}: x must be a number")))?;
            let fit = Fit::of(name, &args[1..2], &args[2..3], ctx)?;
            let slope = fit.slope(name)?;
            Value::Number(slope.mul_add(x - fit.mean_x, fit.mean_y))
        },

        "COVARIANCE.P" | "COVARIANCE.S" => {
            require_args(name, args, 2)?;
            let fit = Fit::of(name, &args[..1], &args[1..2], ctx)?;
            let divisor = if name == "COVARIANCE.P" {
                fit.n
            } else {
                fit.n - 1.0
            };
            Value::Number(fit.sxy / divisor)
        },

        _ => return Ok(None),
    };
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::super::tests::eval;
    use super::super::{EvalContext, Value};
    use std::collections::HashMap;

    /// A table `t` with the columns given.
    fn context(columns: &[(&str, &[f64])]) -> EvalContext {
        let mut ctx = EvalContext::new();
        let table: HashMap<String, Vec<Value>> = columns
            .iter()
            .map(|(name, values)| {
                (
                    (*name).to_string(),
                    values.iter().map(|v| Value::Number(*v)).collect(),
                )
            })
            .collect();
        ctx.tables
            .insert("t".to_string(), table.into_iter().collect());
        ctx
    }

    fn number(formula: &str, ctx: &EvalContext) -> f64 {
        match eval(formula, ctx).unwrap() {
            Value::Number(n) => n,
            other => panic!("{formula}: {other:?}"),
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12 * b.abs().max(1.0)
    }

    /// Five points near y = 2x + 1. By hand: mean x 3, mean y 7.02, sum of
    /// squares of x 10, of y 39.708, of the products 19.9.
    fn points() -> EvalContext {
        context(&[
            ("y", &[3.1, 4.9, 7.2, 8.8, 11.1]),
            ("x", &[1.0, 2.0, 3.0, 4.0, 5.0]),
        ])
    }

    #[test]
    fn slope_intercept_rsq_and_forecast_on_a_line_with_noise() {
        let ctx = points();
        assert!(close(number("SLOPE(t.y, t.x)", &ctx), 1.99));
        assert!(close(number("INTERCEPT(t.y, t.x)", &ctx), 1.05));
        assert!(close(
            number("RSQ(t.y, t.x)", &ctx),
            19.9 * 19.9 / (10.0 * 39.708)
        ));
        assert!(close(number("FORECAST(6, t.y, t.x)", &ctx), 12.99));
        assert!(close(number("FORECAST.LINEAR(0, t.y, t.x)", &ctx), 1.05));
    }

    #[test]
    fn covariance_population_and_sample() {
        let ctx = points();
        assert!(close(number("COVARIANCE.P(t.y, t.x)", &ctx), 3.98));
        assert!(close(number("COVARIANCE.S(t.y, t.x)", &ctx), 4.975));
    }

    #[test]
    fn errors_for_unequal_arrays_few_points_and_a_vertical_line() {
        let message = |ctx: &EvalContext, f: &str| eval(f, ctx).unwrap_err().to_string();
        let unequal = context(&[("a", &[1.0, 2.0, 3.0]), ("b", &[1.0, 2.0])]);
        assert!(message(&unequal, "SLOPE(t.a, t.b)").contains("same size"));
        let one = context(&[("a", &[1.0]), ("b", &[1.0])]);
        assert!(message(&one, "SLOPE(t.a, t.b)").contains("at least 2"));
        let flat = context(&[("a", &[1.0, 2.0, 3.0]), ("b", &[4.0, 4.0, 4.0])]);
        assert!(message(&flat, "SLOPE(t.a, t.b)").contains("#DIV/0!"));
        assert!(message(&flat, "RSQ(t.b, t.a)").contains("zero variance"));
    }
}
