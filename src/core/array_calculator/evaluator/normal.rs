//! The normal distribution: NORM.DIST, NORM.INV, NORM.S.DIST and NORM.S.INV

use super::{evaluate, require_args};
use super::{EvalContext, EvalError, Expr, Value};
use statrs::distribution::{ContinuousCDF, Normal};

/// `exp(-x * x)` without the error `x * x` has when x is large: x is split
/// into a part whose square is exact and a small remainder.
fn exp_neg_square(x: f64) -> f64 {
    let hi = (x * 16.0).floor() / 16.0;
    (-hi * hi).exp() * (-(x - hi) * (x + hi)).exp()
}

/// `exp(-z * z / 2)`, split the same way
fn exp_neg_half_square(z: f64) -> f64 {
    let hi = (z * 16.0).floor() / 16.0;
    (-hi * hi / 2.0).exp() * (-(z - hi) * (z + hi) / 2.0).exp()
}

/// erf(x) for x below 0.5: 2 / sqrt(pi) * exp(-x^2) * the sum of
/// 2^n x^(2n+1) / (1 * 3 * ... * (2n+1)), whose terms are all positive, so
/// nothing cancels.
fn erf_small(x: f64) -> f64 {
    if x == 0.0 {
        return 0.0;
    }
    let (mut term, mut sum) = (x, x);
    let mut n = 1.0;
    loop {
        term *= 2.0 * x * x / 2.0f64.mul_add(n, 1.0);
        sum += term;
        n += 1.0;
        if term.abs() <= 1e-17 * sum.abs() {
            break;
        }
    }
    2.0 / std::f64::consts::PI.sqrt() * exp_neg_square(x) * sum
}

/// erfc(x) * exp(x^2) for x of 0.5 and more: the continued fraction
/// 1 / (sqrt(pi) (x + (1/2) / (x + 1 / (x + (3/2) / (x + ...))))), evaluated
/// from the back. The depth the convergence needs grows as x gets smaller.
fn erfc_scaled(x: f64) -> f64 {
    let depth: u32 = if x < 1.0 {
        1500
    } else if x < 2.0 {
        300
    } else if x < 4.0 {
        100
    } else {
        60
    };
    let mut denominator = x;
    for k in (1..=depth).rev() {
        denominator = x + (f64::from(k) / 2.0) / denominator;
    }
    1.0 / (std::f64::consts::PI.sqrt() * denominator)
}

/// The standard normal distribution function, to about 3 units in the last
/// place everywhere (the `statrs` crate's is right to only 11 digits, which
/// Excel, Gnumeric and R are not). The tails are `exp(-z^2/2)` times a smooth
/// factor, with z squared exactly, because squaring a rounded z is what
/// loses digits far out.
fn standard_cdf(z: f64) -> f64 {
    if z.is_nan() {
        return f64::NAN;
    }
    let x = z.abs() / std::f64::consts::SQRT_2;
    if x < 0.5 {
        return 0.5f64.mul_add(erf_small(z / std::f64::consts::SQRT_2), 0.5);
    }
    let tail = 0.5 * exp_neg_half_square(z.abs()) * erfc_scaled(x);
    if z < 0.0 {
        tail
    } else {
        1.0 - tail
    }
}

/// The standard normal density
fn standard_pdf(z: f64) -> f64 {
    exp_neg_half_square(z) / (2.0 * std::f64::consts::PI).sqrt()
}

/// The argument at `index` as a number
fn number(name: &str, args: &[Expr], index: usize, ctx: &EvalContext) -> Result<f64, EvalError> {
    evaluate(&args[index], ctx)?
        .as_cell_number()
        .ok_or_else(|| EvalError::new(format!("{name} needs a number as argument {}", index + 1)))
}

/// A flag such as `cumulative`: TRUE, FALSE or a number (0 is false)
fn flag(name: &str, args: &[Expr], index: usize, ctx: &EvalContext) -> Result<bool, EvalError> {
    match evaluate(&args[index], ctx)? {
        Value::Boolean(b) => Ok(b),
        other => other.as_number().map(|n| n != 0.0).ok_or_else(|| {
            EvalError::new(format!(
                "{name} needs TRUE or FALSE as argument {}",
                index + 1
            ))
        }),
    }
}

fn positive_std_dev(name: &str, std_dev: f64) -> Result<f64, EvalError> {
    if std_dev > 0.0 {
        Ok(std_dev)
    } else {
        Err(EvalError::new(format!(
            "{name}: the standard deviation must be above 0 (#NUM!)"
        )))
    }
}

/// The z where the standard normal distribution function is `p`
fn standard_inverse(name: &str, p: f64) -> Result<f64, EvalError> {
    Normal::new(0.0, 1.0)
        .map(|normal| normal.inverse_cdf(p))
        .map_err(|e| EvalError::new(format!("{name}: {e}")))
}

fn probability(name: &str, p: f64) -> Result<f64, EvalError> {
    if p <= 0.0 || p >= 1.0 {
        return Err(EvalError::new(format!(
            "{name}: the probability must be between 0 and 1, not 0 or 1 (#NUM!)"
        )));
    }
    Ok(p)
}

/// Try to evaluate a normal-distribution function. Returns None if not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "NORM.DIST" => {
            require_args(name, args, 4)?;
            let x = number(name, args, 0, ctx)?;
            let mean = number(name, args, 1, ctx)?;
            let std_dev = positive_std_dev(name, number(name, args, 2, ctx)?)?;
            let z = (x - mean) / std_dev;
            Value::Number(if flag(name, args, 3, ctx)? {
                standard_cdf(z)
            } else {
                standard_pdf(z) / std_dev
            })
        },

        "NORM.S.DIST" => {
            require_args(name, args, 2)?;
            let z = number(name, args, 0, ctx)?;
            Value::Number(if flag(name, args, 1, ctx)? {
                standard_cdf(z)
            } else {
                standard_pdf(z)
            })
        },

        "NORM.INV" => {
            require_args(name, args, 3)?;
            let p = probability(name, number(name, args, 0, ctx)?)?;
            let mean = number(name, args, 1, ctx)?;
            let std_dev = positive_std_dev(name, number(name, args, 2, ctx)?)?;
            Value::Number(std_dev.mul_add(standard_inverse(name, p)?, mean))
        },

        "NORM.S.INV" => {
            require_args(name, args, 1)?;
            let p = probability(name, number(name, args, 0, ctx)?)?;
            Value::Number(standard_inverse(name, p)?)
        },

        _ => return Ok(None),
    };
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::super::tests::eval;
    use super::super::{EvalContext, Value};

    fn number(formula: &str) -> f64 {
        match eval(formula, &EvalContext::new()).unwrap() {
            Value::Number(n) => n,
            other => panic!("{formula}: {other:?}"),
        }
    }

    fn close(a: f64, b: f64, tolerance: f64) -> bool {
        (a - b).abs() <= tolerance * b.abs().max(1.0)
    }

    #[test]
    fn known_values_of_the_standard_normal() {
        // Phi(0) = 0.5, Phi(1.96) = 0.9750021048517795, phi(0) = 1 / sqrt(2 pi)
        assert!(close(number("NORM.S.DIST(0, TRUE)"), 0.5, 1e-15));
        assert!(close(
            number("NORM.S.DIST(1.96, TRUE)"),
            0.975_002_104_851_779_5,
            1e-13
        ));
        assert!(close(
            number("NORM.S.DIST(0, FALSE)"),
            0.398_942_280_401_432_7,
            1e-13
        ));
        // The 97.5th percentile is 1.959963984540054
        assert!(close(
            number("NORM.S.INV(0.975)"),
            1.959_963_984_540_054,
            1e-12
        ));
        assert!(close(number("NORM.S.INV(0.5)"), 0.0, 1e-15));
    }

    #[test]
    fn far_tails_are_right_to_the_last_digits() {
        // pnorm(-10) and dnorm(-10) in R: 7.6198530241605261e-24, 7.6945986267064199e-23
        assert!(close(
            number("NORM.S.DIST(-10, TRUE)"),
            7.619_853_024_160_526e-24,
            1e-14
        ));
        assert!(close(
            number("NORM.S.DIST(-10, FALSE)"),
            7.694_598_626_706_42e-23,
            1e-14
        ));
        // pnorm(-30) in R: 4.9067139271481867e-198
        assert!(close(
            number("NORM.S.DIST(-30, TRUE)"),
            4.906_713_927_148_187e-198,
            1e-14
        ));
        // and qnorm of the far tail: qnorm(1e-300) = -37.0471...
        assert!(close(
            number("NORM.S.INV(1e-10)"),
            -6.361_340_902_404_056,
            1e-13
        ));
        // The upper half is 1 minus the lower
        assert!(close(
            number("NORM.S.DIST(3, TRUE)"),
            0.998_650_101_968_369_9,
            1e-15
        ));
    }

    #[test]
    fn mean_and_standard_deviation_shift_and_scale() {
        // N(100, 15): P(X <= 115) is Phi(1)
        assert!(close(
            number("NORM.DIST(115, 100, 15, TRUE)"),
            0.841_344_746_068_542_9,
            1e-13
        ));
        assert!(close(
            number("NORM.INV(0.841344746068543, 100, 15)"),
            115.0,
            1e-9
        ));
        assert!(close(
            number("NORM.DIST(100, 100, 15, FALSE)"),
            0.026_596_152_026_762_18,
            1e-13
        ));
    }

    #[test]
    fn invalid_arguments_are_errors() {
        let ctx = EvalContext::new();
        for formula in [
            "NORM.DIST(1, 0, 0, TRUE)",
            "NORM.DIST(1, 0, -2, TRUE)",
            "NORM.INV(0, 0, 1)",
            "NORM.INV(1, 0, 1)",
            "NORM.S.INV(1.5)",
            "NORM.INV(0.5, 0, 0)",
        ] {
            let message = eval(formula, &ctx).unwrap_err().to_string();
            assert!(message.contains("#NUM!"), "{formula}: {message}");
        }
    }
}
