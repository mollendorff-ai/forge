//! Math functions: ABS, ROUND, SQRT, POW, EXP, LN, LOG, etc.

use super::{evaluate, require_args, require_args_range, EvalContext, EvalError, Expr, Value};

/// Try to evaluate a math function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "ABS" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ABS requires a number"))?;
            Value::Number(val.abs())
        }

        "SQRT" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("SQRT requires a number"))?;
            if val < 0.0 {
                return Err(EvalError::new("SQRT of negative number"));
            }
            Value::Number(val.sqrt())
        }

        "ROUND" => {
            require_args_range(name, args, 1, 2)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ROUND requires a number"))?;
            let decimals = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0
            };
            let multiplier = 10_f64.powi(decimals);
            Value::Number((val * multiplier).round() / multiplier)
        }

        "ROUNDUP" => {
            require_args_range(name, args, 1, 2)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ROUNDUP requires a number"))?;
            let decimals = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0
            };
            let multiplier = 10_f64.powi(decimals);
            let sign = if val >= 0.0 { 1.0 } else { -1.0 };
            Value::Number(sign * (val.abs() * multiplier).ceil() / multiplier)
        }

        "ROUNDDOWN" => {
            require_args_range(name, args, 1, 2)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ROUNDDOWN requires a number"))?;
            let decimals = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0
            };
            let multiplier = 10_f64.powi(decimals);
            let sign = if val >= 0.0 { 1.0 } else { -1.0 };
            Value::Number(sign * (val.abs() * multiplier).floor() / multiplier)
        }

        "FLOOR" => {
            require_args_range(name, args, 1, 2)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("FLOOR requires a number"))?;
            let significance = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0)
            } else {
                1.0
            };
            if significance == 0.0 {
                Value::Number(0.0)
            } else {
                Value::Number((val / significance).floor() * significance)
            }
        }

        "CEILING" => {
            require_args_range(name, args, 1, 2)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("CEILING requires a number"))?;
            let significance = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(1.0)
            } else {
                1.0
            };
            if significance == 0.0 {
                Value::Number(0.0)
            } else {
                Value::Number((val / significance).ceil() * significance)
            }
        }

        "MOD" => {
            require_args(name, args, 2)?;
            let num = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("MOD requires numbers"))?;
            let divisor = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("MOD requires numbers"))?;
            if divisor == 0.0 {
                return Err(EvalError::new("MOD division by zero"));
            }
            Value::Number(num % divisor)
        }

        "POWER" => {
            require_args(name, args, 2)?;
            let base = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("POWER requires numbers"))?;
            let exp = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("POWER requires numbers"))?;
            Value::Number(base.powf(exp))
        }

        "EXP" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("EXP requires a number"))?;
            Value::Number(val.exp())
        }

        "LN" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("LN requires a number"))?;
            if val <= 0.0 {
                return Err(EvalError::new("LN of non-positive number"));
            }
            Value::Number(val.ln())
        }

        "LOG" | "LOG10" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("LOG requires a number"))?;
            if val <= 0.0 {
                return Err(EvalError::new("LOG of non-positive number"));
            }
            Value::Number(val.log10())
        }

        "INT" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("INT requires a number"))?;
            Value::Number(val.floor())
        }

        _ => return Ok(None),
    };

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::super::tests::eval;
    use super::*;

    #[test]
    fn test_math_functions() {
        let ctx = EvalContext::new();
        assert_eq!(eval("ABS(-5)", &ctx).unwrap(), Value::Number(5.0));
        assert_eq!(eval("SQRT(16)", &ctx).unwrap(), Value::Number(4.0));
        assert_eq!(eval("ROUND(3.567, 2)", &ctx).unwrap(), Value::Number(3.57));
        assert_eq!(eval("FLOOR(3.7)", &ctx).unwrap(), Value::Number(3.0));
        assert_eq!(eval("CEILING(3.2)", &ctx).unwrap(), Value::Number(4.0));
        assert_eq!(eval("MOD(10, 3)", &ctx).unwrap(), Value::Number(1.0));
        assert_eq!(eval("POWER(2, 3)", &ctx).unwrap(), Value::Number(8.0));
    }

    #[test]
    fn test_exp_ln_log() {
        let ctx = EvalContext::new();
        // e^1 ≈ 2.718...
        let exp_result = eval("EXP(1)", &ctx).unwrap();
        assert!(matches!(exp_result, Value::Number(n) if (n - std::f64::consts::E).abs() < 0.0001));

        // ln(e) = 1
        let ln_result = eval("LN(2.718281828)", &ctx).unwrap();
        assert!(matches!(ln_result, Value::Number(n) if (n - 1.0).abs() < 0.0001));

        // log10(100) = 2
        assert_eq!(eval("LOG10(100)", &ctx).unwrap(), Value::Number(2.0));
    }

    #[test]
    fn test_round_variants() {
        let ctx = EvalContext::new();
        assert_eq!(eval("ROUNDUP(3.2)", &ctx).unwrap(), Value::Number(4.0));
        assert_eq!(eval("ROUNDUP(-3.2)", &ctx).unwrap(), Value::Number(-4.0));
        assert_eq!(eval("ROUNDDOWN(3.9)", &ctx).unwrap(), Value::Number(3.0));
        assert_eq!(eval("ROUNDDOWN(-3.9)", &ctx).unwrap(), Value::Number(-3.0));
        assert_eq!(eval("INT(3.9)", &ctx).unwrap(), Value::Number(3.0));
    }
}
