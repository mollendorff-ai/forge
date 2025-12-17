//! Math functions: ABS, ROUND, SQRT, POW, EXP, LN, LOG, etc.
//!
//! DEMO functions (16): ROUND, ROUNDUP, ROUNDDOWN, ABS, SQRT, POWER, MOD, CEILING, FLOOR, EXP, LN, LOG10, INT, SIGN, TRUNC, PI
//! ENTERPRISE functions: POW, E, LOG

use super::{evaluate, require_args, require_args_range, EvalContext, EvalError, Expr, Value};

/// Try to evaluate a math function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    // ═══════════════════════════════════════════════════════════════════════════
    // DEMO FUNCTIONS (always available)
    // ═══════════════════════════════════════════════════════════════════════════
    let result = match name {
        "ABS" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ABS requires a number"))?;
            Value::Number(val.abs())
        },

        "SQRT" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("SQRT requires a number"))?;
            if val < 0.0 {
                return Err(EvalError::new("SQRT of negative number"));
            }
            Value::Number(val.sqrt())
        },

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
        },

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
        },

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
        },

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
        },

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
        },

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
            // Excel uses floored division: MOD(n, d) = n - d * FLOOR(n/d)
            Value::Number(num - divisor * (num / divisor).floor())
        },

        "POWER" => {
            require_args(name, args, 2)?;
            let base = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("POWER requires numbers"))?;
            let exp = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("POWER requires numbers"))?;
            Value::Number(base.powf(exp))
        },

        "EXP" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("EXP requires a number"))?;
            Value::Number(val.exp())
        },

        "LN" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("LN requires a number"))?;
            if val <= 0.0 {
                return Err(EvalError::new("LN of non-positive number"));
            }
            Value::Number(val.ln())
        },

        "LOG10" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("LOG10 requires a number"))?;
            if val <= 0.0 {
                return Err(EvalError::new("LOG10 of non-positive number"));
            }
            Value::Number(val.log10())
        },

        "INT" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("INT requires a number"))?;
            Value::Number(val.floor())
        },

        "SIGN" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("SIGN requires a number"))?;
            Value::Number(if val > 0.0 {
                1.0
            } else if val < 0.0 {
                -1.0
            } else {
                0.0
            })
        },

        "TRUNC" => {
            require_args_range(name, args, 1, 2)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("TRUNC requires a number"))?;
            let decimals = if args.len() > 1 {
                evaluate(&args[1], ctx)?.as_number().unwrap_or(0.0) as i32
            } else {
                0
            };
            let multiplier = 10_f64.powi(decimals);
            Value::Number(val.signum() * (val.abs() * multiplier).floor() / multiplier)
        },

        "PI" => {
            require_args(name, args, 0)?;
            Value::Number(std::f64::consts::PI)
        },

        // ═══════════════════════════════════════════════════════════════════════════
        // ENTERPRISE FUNCTIONS (only in full build)
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(not(feature = "demo"))]
        "LOG" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("LOG requires a number"))?;
            if val <= 0.0 {
                return Err(EvalError::new("LOG of non-positive number"));
            }
            Value::Number(val.log10())
        },

        #[cfg(not(feature = "demo"))]
        "POW" => {
            // Alias for POWER
            require_args(name, args, 2)?;
            let base = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("POW requires numbers"))?;
            let exp = evaluate(&args[1], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("POW requires numbers"))?;
            Value::Number(base.powf(exp))
        },

        #[cfg(not(feature = "demo"))]
        "E" => {
            require_args(name, args, 0)?;
            Value::Number(std::f64::consts::E)
        },

        _ => return Ok(None),
    };

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::super::tests::eval;
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // DEMO TESTS (always run)
    // ═══════════════════════════════════════════════════════════════════════════

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
    fn test_round_variants() {
        let ctx = EvalContext::new();
        assert_eq!(eval("ROUNDUP(3.2)", &ctx).unwrap(), Value::Number(4.0));
        assert_eq!(eval("ROUNDUP(-3.2)", &ctx).unwrap(), Value::Number(-4.0));
        assert_eq!(eval("ROUNDDOWN(3.9)", &ctx).unwrap(), Value::Number(3.0));
        assert_eq!(eval("ROUNDDOWN(-3.9)", &ctx).unwrap(), Value::Number(-3.0));
    }

    #[test]
    fn test_sqrt_negative() {
        let ctx = EvalContext::new();
        let result = eval("SQRT(-4)", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("negative"));
    }

    #[test]
    fn test_sqrt_edge_cases() {
        let ctx = EvalContext::new();
        // SQRT(-1) error case (edge case from edge_errors.yaml)
        let result = eval("SQRT(-1)", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("negative"));

        // SQRT of 0 = 0 (valid)
        assert_eq!(eval("SQRT(0)", &ctx).unwrap(), Value::Number(0.0));

        // SQRT of positive = valid
        assert_eq!(eval("SQRT(4)", &ctx).unwrap(), Value::Number(2.0));

        // SQRT of fractional positive = valid
        assert_eq!(eval("SQRT(0.25)", &ctx).unwrap(), Value::Number(0.5));
    }

    #[test]
    fn test_floor_zero_significance() {
        let ctx = EvalContext::new();
        assert_eq!(eval("FLOOR(3.5, 0)", &ctx).unwrap(), Value::Number(0.0));
    }

    #[test]
    fn test_ceiling_zero_significance() {
        let ctx = EvalContext::new();
        assert_eq!(eval("CEILING(3.5, 0)", &ctx).unwrap(), Value::Number(0.0));
    }

    #[test]
    fn test_mod_division_by_zero() {
        let ctx = EvalContext::new();
        let result = eval("MOD(10, 0)", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("zero"));
    }

    #[test]
    fn test_mod_edge_cases() {
        let ctx = EvalContext::new();
        // Edge case 12: IFERROR(MOD(5, 0), -1) = -1 (mod by zero caught)
        // Testing the error case directly
        let result = eval("MOD(5, 0)", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("zero"));

        // MOD with valid divisor = valid
        assert_eq!(eval("MOD(10, 3)", &ctx).unwrap(), Value::Number(1.0));

        // MOD with negative dividend
        assert_eq!(eval("MOD(-5, 3)", &ctx).unwrap(), Value::Number(1.0));

        // MOD with negative divisor
        assert_eq!(eval("MOD(5, -3)", &ctx).unwrap(), Value::Number(-1.0));

        // MOD with both negative
        assert_eq!(eval("MOD(-5, -3)", &ctx).unwrap(), Value::Number(-2.0));

        // MOD with zero dividend
        assert_eq!(eval("MOD(0, 5)", &ctx).unwrap(), Value::Number(0.0));
    }

    #[test]
    fn test_mod_negative_dividend() {
        let ctx = EvalContext::new();
        // MOD(-5, 3) = 1 (Excel uses floored division)
        assert_eq!(eval("MOD(-5, 3)", &ctx).unwrap(), Value::Number(1.0));
    }

    #[test]
    fn test_mod_negative_divisor() {
        let ctx = EvalContext::new();
        // MOD(5, -3) = -1 (Excel uses floored division)
        assert_eq!(eval("MOD(5, -3)", &ctx).unwrap(), Value::Number(-1.0));
    }

    #[test]
    fn test_mod_both_negative() {
        let ctx = EvalContext::new();
        // MOD(-5, -3) = -2 (Excel uses floored division)
        assert_eq!(eval("MOD(-5, -3)", &ctx).unwrap(), Value::Number(-2.0));
    }

    #[test]
    fn test_round_default_decimals() {
        let ctx = EvalContext::new();
        assert_eq!(eval("ROUND(3.567)", &ctx).unwrap(), Value::Number(4.0));
    }

    #[test]
    fn test_roundup_with_decimals() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("ROUNDUP(3.141, 2)", &ctx).unwrap(),
            Value::Number(3.15)
        );
    }

    #[test]
    fn test_rounddown_with_decimals() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("ROUNDDOWN(3.789, 2)", &ctx).unwrap(),
            Value::Number(3.78)
        );
    }

    #[test]
    fn test_floor_with_significance() {
        let ctx = EvalContext::new();
        assert_eq!(eval("FLOOR(17, 5)", &ctx).unwrap(), Value::Number(15.0));
    }

    #[test]
    fn test_ceiling_with_significance() {
        let ctx = EvalContext::new();
        assert_eq!(eval("CEILING(13, 5)", &ctx).unwrap(), Value::Number(15.0));
    }

    #[test]
    fn test_exp_ln_log10() {
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
    fn test_int() {
        let ctx = EvalContext::new();
        assert_eq!(eval("INT(3.9)", &ctx).unwrap(), Value::Number(3.0));
    }

    #[test]
    fn test_ln_non_positive() {
        let ctx = EvalContext::new();
        let result = eval("LN(0)", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-positive"));

        let result = eval("LN(-1)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_ln_edge_cases() {
        let ctx = EvalContext::new();
        // Edge case 8: IFERROR(LN(0), -1) = -1 (ln zero caught)
        // Testing the error case directly
        let result = eval("LN(0)", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-positive"));

        // LN of negative number = error
        let result = eval("LN(-5)", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-positive"));

        // LN of positive number = valid
        assert!(eval("LN(1)", &ctx).is_ok()); // LN(1) = 0
        assert!(eval("LN(2.718281828)", &ctx).is_ok()); // LN(e) ≈ 1

        // LN of fractional positive = valid
        assert!(eval("LN(0.5)", &ctx).is_ok());
    }

    #[test]
    fn test_log10_non_positive() {
        let ctx = EvalContext::new();
        let result = eval("LOG10(0)", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-positive"));
    }

    #[test]
    fn test_log10_edge_cases() {
        let ctx = EvalContext::new();
        // Edge case 7: IFERROR(LOG10(0), -1) = -1 (log zero caught)
        // Testing the error case directly
        let result = eval("LOG10(0)", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-positive"));

        // LOG10 of negative number = error
        let result = eval("LOG10(-5)", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-positive"));

        // LOG10 of positive number = valid
        assert_eq!(eval("LOG10(1)", &ctx).unwrap(), Value::Number(0.0)); // LOG10(1) = 0
        assert_eq!(eval("LOG10(10)", &ctx).unwrap(), Value::Number(1.0)); // LOG10(10) = 1
        assert_eq!(eval("LOG10(100)", &ctx).unwrap(), Value::Number(2.0)); // LOG10(100) = 2

        // LOG10 of fractional positive = valid
        assert!(eval("LOG10(0.5)", &ctx).is_ok());
    }

    #[test]
    fn test_sign() {
        let ctx = EvalContext::new();
        assert_eq!(eval("SIGN(5)", &ctx).unwrap(), Value::Number(1.0));
        assert_eq!(eval("SIGN(-5)", &ctx).unwrap(), Value::Number(-1.0));
        assert_eq!(eval("SIGN(0)", &ctx).unwrap(), Value::Number(0.0));
    }

    #[test]
    fn test_trunc() {
        let ctx = EvalContext::new();
        assert_eq!(eval("TRUNC(3.9)", &ctx).unwrap(), Value::Number(3.0));
        assert_eq!(eval("TRUNC(-3.9)", &ctx).unwrap(), Value::Number(-3.0));
        assert_eq!(eval("TRUNC(3.567, 2)", &ctx).unwrap(), Value::Number(3.56));
    }

    #[test]
    fn test_pi() {
        let ctx = EvalContext::new();
        let result = eval("PI()", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n - std::f64::consts::PI).abs() < 0.0001));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // NUMERIC EDGE CASES (from forge-demo edge_numeric.yaml)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_int_edge_cases() {
        let ctx = EvalContext::new();
        // INT(5.9) = 5 (floor toward negative infinity)
        assert_eq!(eval("INT(5.9)", &ctx).unwrap(), Value::Number(5.0));
        // INT(-5.9) = -6
        assert_eq!(eval("INT(-5.9)", &ctx).unwrap(), Value::Number(-6.0));
        // INT(-5.1) = -6
        assert_eq!(eval("INT(-5.1)", &ctx).unwrap(), Value::Number(-6.0));
    }

    #[test]
    fn test_trunc_edge_cases() {
        let ctx = EvalContext::new();
        // TRUNC(5.9) = 5 (truncate toward zero)
        assert_eq!(eval("TRUNC(5.9)", &ctx).unwrap(), Value::Number(5.0));
        // TRUNC(-5.9) = -5
        assert_eq!(eval("TRUNC(-5.9)", &ctx).unwrap(), Value::Number(-5.0));
        // TRUNC(-5.1) = -5
        assert_eq!(eval("TRUNC(-5.1)", &ctx).unwrap(), Value::Number(-5.0));
    }

    #[test]
    fn test_round_edge_cases_half_up() {
        let ctx = EvalContext::new();
        // ROUND(2.5, 0) = 3 (round half up)
        assert_eq!(eval("ROUND(2.5, 0)", &ctx).unwrap(), Value::Number(3.0));
        // ROUND(3.5, 0) = 4
        assert_eq!(eval("ROUND(3.5, 0)", &ctx).unwrap(), Value::Number(4.0));
        // ROUND(-2.5, 0) = -3
        assert_eq!(eval("ROUND(-2.5, 0)", &ctx).unwrap(), Value::Number(-3.0));
    }

    #[test]
    fn test_round_edge_cases_negative_precision() {
        let ctx = EvalContext::new();
        // ROUND(1234.5, -2) = 1200 (negative precision)
        assert_eq!(
            eval("ROUND(1234.5, -2)", &ctx).unwrap(),
            Value::Number(1200.0)
        );
        // ROUND(1250, -2) = 1300
        assert_eq!(
            eval("ROUND(1250, -2)", &ctx).unwrap(),
            Value::Number(1300.0)
        );
    }

    #[test]
    fn test_ceiling_edge_cases() {
        let ctx = EvalContext::new();
        // CEILING(2.1, 1) = 3
        assert_eq!(eval("CEILING(2.1, 1)", &ctx).unwrap(), Value::Number(3.0));
        // CEILING(-2.1, 1) = -2
        assert_eq!(eval("CEILING(-2.1, 1)", &ctx).unwrap(), Value::Number(-2.0));
    }

    #[test]
    fn test_floor_edge_cases() {
        let ctx = EvalContext::new();
        // FLOOR(2.9, 1) = 2
        assert_eq!(eval("FLOOR(2.9, 1)", &ctx).unwrap(), Value::Number(2.0));
        // FLOOR(-2.9, 1) = -3
        assert_eq!(eval("FLOOR(-2.9, 1)", &ctx).unwrap(), Value::Number(-3.0));
    }

    #[test]
    fn test_sqrt_zero_edge_case() {
        let ctx = EvalContext::new();
        // SQRT(0) = 0
        assert_eq!(eval("SQRT(0)", &ctx).unwrap(), Value::Number(0.0));
    }

    #[test]
    fn test_abs_negative_zero_edge_case() {
        let ctx = EvalContext::new();
        // ABS(-0) = 0
        assert_eq!(eval("ABS(-0)", &ctx).unwrap(), Value::Number(0.0));
    }

    #[test]
    fn test_sign_edge_cases() {
        let ctx = EvalContext::new();
        // SIGN(0) = 0
        assert_eq!(eval("SIGN(0)", &ctx).unwrap(), Value::Number(0.0));
        // SIGN(100) = 1
        assert_eq!(eval("SIGN(100)", &ctx).unwrap(), Value::Number(1.0));
        // SIGN(-100) = -1
        assert_eq!(eval("SIGN(-100)", &ctx).unwrap(), Value::Number(-1.0));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ENTERPRISE TESTS (only with full feature)
    // ═══════════════════════════════════════════════════════════════════════════

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_log_alias() {
        let ctx = EvalContext::new();
        assert_eq!(eval("LOG(100)", &ctx).unwrap(), Value::Number(2.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_pow() {
        let ctx = EvalContext::new();
        assert_eq!(eval("POW(2, 3)", &ctx).unwrap(), Value::Number(8.0));
        assert_eq!(eval("POW(3, 2)", &ctx).unwrap(), Value::Number(9.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_e() {
        let ctx = EvalContext::new();
        let result = eval("E()", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n - std::f64::consts::E).abs() < 0.0001));
    }
}
