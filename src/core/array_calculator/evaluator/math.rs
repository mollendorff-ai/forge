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
#[allow(clippy::approx_constant)]
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

    // ═══════════════════════════════════════════════════════════════════════════
    // INTEGRATION TESTS (from tests/math.rs)
    // ═══════════════════════════════════════════════════════════════════════════

    use crate::core::array_calculator::ArrayCalculator;
    #[allow(unused_imports)]
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_round_function() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.456, 2.789, 3.123, 4.555]),
        ));
        table.add_row_formula("rounded_1".to_string(), "=ROUND(values, 1)".to_string());
        table.add_row_formula("rounded_2".to_string(), "=ROUND(values, 2)".to_string());
        table.add_row_formula("rounded_0".to_string(), "=ROUND(values, 0)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let rounded_1 = result_table.columns.get("rounded_1").unwrap();
        match &rounded_1.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 1.5);
                assert_eq!(nums[1], 2.8);
                assert_eq!(nums[2], 3.1);
                assert_eq!(nums[3], 4.6);
            },
            _ => panic!("Expected Number array"),
        }

        let rounded_2 = result_table.columns.get("rounded_2").unwrap();
        match &rounded_2.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 1.46);
                assert_eq!(nums[1], 2.79);
                assert_eq!(nums[2], 3.12);
                assert_eq!(nums[3], 4.56);
            },
            _ => panic!("Expected Number array"),
        }

        let rounded_0 = result_table.columns.get("rounded_0").unwrap();
        match &rounded_0.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 1.0);
                assert_eq!(nums[1], 3.0);
                assert_eq!(nums[2], 3.0);
                assert_eq!(nums[3], 5.0);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_roundup_function() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.231, 2.678, 3.449]),
        ));
        table.add_row_formula("rounded_up".to_string(), "=ROUNDUP(values, 1)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let rounded_up = result_table.columns.get("rounded_up").unwrap();
        match &rounded_up.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 1.3);
                assert_eq!(nums[1], 2.7);
                assert_eq!(nums[2], 3.5);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_rounddown_function() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.789, 2.345, 3.999]),
        ));
        table.add_row_formula(
            "rounded_down".to_string(),
            "=ROUNDDOWN(values, 1)".to_string(),
        );

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let rounded_down = result_table.columns.get("rounded_down").unwrap();
        match &rounded_down.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 1.7);
                assert_eq!(nums[1], 2.3);
                assert_eq!(nums[2], 3.9);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_ceiling_function() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.1, 2.3, 4.7, 10.2]),
        ));
        table.add_row_formula("ceiling_1".to_string(), "=CEILING(values, 1)".to_string());
        table.add_row_formula("ceiling_5".to_string(), "=CEILING(values, 5)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let ceiling_1 = result_table.columns.get("ceiling_1").unwrap();
        match &ceiling_1.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 2.0);
                assert_eq!(nums[1], 3.0);
                assert_eq!(nums[2], 5.0);
                assert_eq!(nums[3], 11.0);
            },
            _ => panic!("Expected Number array"),
        }

        let ceiling_5 = result_table.columns.get("ceiling_5").unwrap();
        match &ceiling_5.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 5.0);
                assert_eq!(nums[1], 5.0);
                assert_eq!(nums[2], 5.0);
                assert_eq!(nums[3], 15.0);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_floor_function() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.9, 2.7, 4.3, 10.8]),
        ));
        table.add_row_formula("floor_1".to_string(), "=FLOOR(values, 1)".to_string());
        table.add_row_formula("floor_5".to_string(), "=FLOOR(values, 5)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let floor_1 = result_table.columns.get("floor_1").unwrap();
        match &floor_1.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 1.0);
                assert_eq!(nums[1], 2.0);
                assert_eq!(nums[2], 4.0);
                assert_eq!(nums[3], 10.0);
            },
            _ => panic!("Expected Number array"),
        }

        let floor_5 = result_table.columns.get("floor_5").unwrap();
        match &floor_5.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 0.0);
                assert_eq!(nums[1], 0.0);
                assert_eq!(nums[2], 0.0);
                assert_eq!(nums[3], 10.0);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_mod_function() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 15.0, 23.0, 7.0]),
        ));
        table.add_row_formula("mod_3".to_string(), "=MOD(values, 3)".to_string());
        table.add_row_formula("mod_5".to_string(), "=MOD(values, 5)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let mod_3 = result_table.columns.get("mod_3").unwrap();
        match &mod_3.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 1.0);
                assert_eq!(nums[1], 0.0);
                assert_eq!(nums[2], 2.0);
                assert_eq!(nums[3], 1.0);
            },
            _ => panic!("Expected Number array"),
        }

        let mod_5 = result_table.columns.get("mod_5").unwrap();
        match &mod_5.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 0.0);
                assert_eq!(nums[1], 0.0);
                assert_eq!(nums[2], 3.0);
                assert_eq!(nums[3], 2.0);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_sqrt_function() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![4.0, 9.0, 16.0, 25.0, 100.0]),
        ));
        table.add_row_formula("sqrt_values".to_string(), "=SQRT(values)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let sqrt_values = result_table.columns.get("sqrt_values").unwrap();
        match &sqrt_values.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 2.0);
                assert_eq!(nums[1], 3.0);
                assert_eq!(nums[2], 4.0);
                assert_eq!(nums[3], 5.0);
                assert_eq!(nums[4], 10.0);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_power_function() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "base".to_string(),
            ColumnValue::Number(vec![2.0, 3.0, 4.0, 5.0]),
        ));
        table.add_row_formula("power_2".to_string(), "=POWER(base, 2)".to_string());
        table.add_row_formula("power_3".to_string(), "=POWER(base, 3)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let power_2 = result_table.columns.get("power_2").unwrap();
        match &power_2.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 4.0);
                assert_eq!(nums[1], 9.0);
                assert_eq!(nums[2], 16.0);
                assert_eq!(nums[3], 25.0);
            },
            _ => panic!("Expected Number array"),
        }

        let power_3 = result_table.columns.get("power_3").unwrap();
        match &power_3.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 8.0);
                assert_eq!(nums[1], 27.0);
                assert_eq!(nums[2], 64.0);
                assert_eq!(nums[3], 125.0);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_abs_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![-10.0, 5.0, -3.0, 8.0]),
        ));
        data.row_formulas
            .insert("absolute".to_string(), "=ABS(values)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let abs_col = result
            .tables
            .get("data")
            .unwrap()
            .columns
            .get("absolute")
            .unwrap();
        if let ColumnValue::Number(values) = &abs_col.values {
            assert_eq!(values[0], 10.0);
            assert_eq!(values[1], 5.0);
            assert_eq!(values[2], 3.0);
            assert_eq!(values[3], 8.0);
        } else {
            panic!("Expected numeric column");
        }
    }

    #[test]
    fn test_round_functions() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "val".to_string(),
            Variable::new("val".to_string(), Some(3.567), None),
        );
        model.add_scalar(
            "rounded".to_string(),
            Variable::new(
                "rounded".to_string(),
                None,
                Some("=ROUND(val, 2)".to_string()),
            ),
        );
        model.add_scalar(
            "up".to_string(),
            Variable::new("up".to_string(), None, Some("=ROUNDUP(val, 1)".to_string())),
        );
        model.add_scalar(
            "down".to_string(),
            Variable::new(
                "down".to_string(),
                None,
                Some("=ROUNDDOWN(val, 1)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        assert!((result.scalars.get("rounded").unwrap().value.unwrap() - 3.57).abs() < 0.01);
        assert!((result.scalars.get("up").unwrap().value.unwrap() - 3.6).abs() < 0.01);
        assert!((result.scalars.get("down").unwrap().value.unwrap() - 3.5).abs() < 0.01);
    }

    #[test]
    fn test_power_and_sqrt() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "base".to_string(),
            Variable::new("base".to_string(), Some(2.0), None),
        );
        model.add_scalar(
            "squared".to_string(),
            Variable::new(
                "squared".to_string(),
                None,
                Some("=POWER(base, 2)".to_string()),
            ),
        );
        model.add_scalar(
            "root".to_string(),
            Variable::new("root".to_string(), None, Some("=SQRT(squared)".to_string())),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        assert!((result.scalars.get("squared").unwrap().value.unwrap() - 4.0).abs() < 0.01);
        assert!((result.scalars.get("root").unwrap().value.unwrap() - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_mod_scalar_formula() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=MOD(17, 5)".to_string())),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let val = result.scalars.get("result").unwrap().value.unwrap();
        assert!((val - 2.0).abs() < 0.01); // 17 mod 5 = 2
    }

    #[test]
    fn test_floor_and_ceiling() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "val".to_string(),
            Variable::new("val".to_string(), Some(4.3), None),
        );
        model.add_scalar(
            "floor".to_string(),
            Variable::new(
                "floor".to_string(),
                None,
                Some("=FLOOR(val, 1)".to_string()),
            ),
        );
        model.add_scalar(
            "ceil".to_string(),
            Variable::new(
                "ceil".to_string(),
                None,
                Some("=CEILING(val, 1)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        assert!((result.scalars.get("floor").unwrap().value.unwrap() - 4.0).abs() < 0.01);
        assert!((result.scalars.get("ceil").unwrap().value.unwrap() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_round_function_rowwise() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![3.14159]),
        ));
        data.row_formulas
            .insert("rounded".to_string(), "=ROUND(value, 2)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("rounded") {
            if let ColumnValue::Number(vals) = &col.values {
                assert!((vals[0] - 3.14).abs() < 0.001);
            }
        }
    }

    #[test]
    fn test_ceiling_function_rowwise() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![4.3]),
        ));
        data.row_formulas
            .insert("ceil".to_string(), "=CEILING(value, 1)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("ceil") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 5.0);
            }
        }
    }

    #[test]
    fn test_floor_function_rowwise() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![4.9]),
        ));
        data.row_formulas
            .insert("floor_val".to_string(), "=FLOOR(value, 1)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("floor_val") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 4.0);
            }
        }
    }

    #[test]
    fn test_mod_function_rowwise() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![10.0]),
        ));
        data.row_formulas
            .insert("remainder".to_string(), "=MOD(value, 3)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("remainder") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 1.0);
            }
        }
    }

    #[test]
    fn test_sqrt_function_rowwise() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![16.0]),
        ));
        data.row_formulas
            .insert("root".to_string(), "=SQRT(value)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("root") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 4.0);
            }
        }
    }

    #[test]
    fn test_power_function_rowwise() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "base".to_string(),
            ColumnValue::Number(vec![2.0]),
        ));
        data.row_formulas
            .insert("result".to_string(), "=POWER(base, 10)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("result") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 1024.0);
            }
        }
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_sln_function_coverage() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "depreciation".to_string(),
            Variable::new(
                "depreciation".to_string(),
                None,
                Some("=SLN(30000, 7500, 10)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // SLN(30000, 7500, 10) = (30000 - 7500) / 10 = 2250
        let val = result.scalars.get("depreciation").unwrap().value.unwrap();
        assert!((val - 2250.0).abs() < 0.01);
    }

    #[test]
    fn test_abs_function_rowwise() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![-10.0, 20.0, -30.0]),
        ));
        data.row_formulas
            .insert("abs_val".to_string(), "=ABS(value)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_exp_function_rowwise() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![0.0, 1.0, 2.0]),
        ));
        data.row_formulas
            .insert("exp_x".to_string(), "=EXP(x)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let table = result.tables.get("data").unwrap();

        if let Some(col) = table.columns.get("exp_x") {
            if let ColumnValue::Number(vals) = &col.values {
                assert!((vals[0] - 1.0).abs() < 0.001); // EXP(0) = 1
                assert!((vals[1] - std::f64::consts::E).abs() < 0.001); // EXP(1) = e ≈ 2.718
                assert!((vals[2] - (std::f64::consts::E * std::f64::consts::E)).abs() < 0.001);
                // EXP(2) = e^2 ≈ 7.389
            }
        }
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_ln_function_rowwise() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![1.0, 2.718, 10.0]),
        ));
        data.row_formulas
            .insert("ln_x".to_string(), "=LN(x)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let table = result.tables.get("data").unwrap();

        if let Some(col) = table.columns.get("ln_x") {
            if let ColumnValue::Number(vals) = &col.values {
                assert!((vals[0] - 0.0).abs() < 0.001); // LN(1) = 0
                assert!((vals[1] - 1.0).abs() < 0.01); // LN(e) ≈ 1
                assert!((vals[2] - 2.302585).abs() < 0.001); // LN(10) ≈ 2.302585
            }
        }
    }

    #[test]
    fn test_log_function_rowwise() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![10.0, 100.0, 1000.0]),
        ));
        data.row_formulas
            .insert("log_x".to_string(), "=LOG(x, 10)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();

        // LOG function is not currently implemented - test verifies error handling
        assert!(
            result.is_err(),
            "LOG function should error (not implemented)"
        );
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_sln_depreciation() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "depr".to_string(),
            Variable::new(
                "depr".to_string(),
                None,
                Some("=SLN(10000, 1000, 5)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // SLN(10000, 1000, 5) = (10000 - 1000) / 5 = 1800
        let val = result.scalars.get("depr").unwrap().value.unwrap();
        assert!((val - 1800.0).abs() < 0.01);
    }

    #[test]
    fn test_abs_negative_value() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=ABS(-5)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let val = result.scalars.get("result").unwrap().value.unwrap();
        assert!((val - 5.0).abs() < 0.01);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_exp_function() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=EXP(1)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // EXP(1) = e ≈ 2.718
        let val = result.scalars.get("result").unwrap().value.unwrap();
        assert!((val - std::f64::consts::E).abs() < 0.001);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_ln_function() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=LN(2.718)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // LN(e) ≈ 1.0
        let val = result.scalars.get("result").unwrap().value.unwrap();
        assert!((val - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_log_function() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LOG(100, 10)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();

        // LOG function is not currently implemented - test verifies error handling
        assert!(
            result.is_err(),
            "LOG function should error (not implemented)"
        );
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_log10_function() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=LOG10(1000)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // LOG10(1000) = 3.0
        let val = result.scalars.get("result").unwrap().value.unwrap();
        assert!((val - 3.0).abs() < 0.001);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_sign_function() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=SIGN(-5)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // SIGN(-5) = -1.0
        let val = result.scalars.get("result").unwrap().value.unwrap();
        assert!((val - (-1.0)).abs() < 0.001);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_int_function() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=INT(5.7)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // INT(5.7) = 5.0
        let val = result.scalars.get("result").unwrap().value.unwrap();
        assert!((val - 5.0).abs() < 0.001);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_trunc_function() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=TRUNC(5.789, 2)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // TRUNC(5.789, 2) = 5.78
        let val = result.scalars.get("result").unwrap().value.unwrap();
        assert!((val - 5.78).abs() < 0.001);
    }

    #[test]
    fn test_ceiling_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=CEILING(4.3, 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // CEILING(4.3, 1) = 5
        let val = result.scalars.get("result").unwrap().value.unwrap();
        assert!((val - 5.0).abs() < 0.0001);
    }

    #[test]
    fn test_floor_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=FLOOR(4.7, 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // FLOOR(4.7, 1) = 4
        let val = result.scalars.get("result").unwrap().value.unwrap();
        assert!((val - 4.0).abs() < 0.0001);
    }

    #[test]
    fn test_mod_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=MOD(10, 3)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // MOD(10, 3) = 1
        let val = result.scalars.get("result").unwrap().value.unwrap();
        assert!((val - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_sqrt_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=SQRT(16)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // SQRT(16) = 4
        let val = result.scalars.get("result").unwrap().value.unwrap();
        assert!((val - 4.0).abs() < 0.0001);
    }

    #[test]
    fn test_power_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=POWER(2, 8)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // POWER(2, 8) = 256
        let val = result.scalars.get("result").unwrap().value.unwrap();
        assert!((val - 256.0).abs() < 0.0001);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_variance_abs_value() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "actual".to_string(),
            Variable::new("actual".to_string(), Some(120.0), None),
        );
        model.add_scalar(
            "budget".to_string(),
            Variable::new("budget".to_string(), Some(100.0), None),
        );
        model.add_scalar(
            "var".to_string(),
            Variable::new(
                "var".to_string(),
                None,
                Some("=VARIANCE(actual, budget)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        // VARIANCE(120, 100) = 120 - 100 = 20.0
        let val = result.scalars.get("var").unwrap().value.unwrap();
        assert!((val - 20.0).abs() < 0.001);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_pi_constant() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "pi_value".to_string(),
            Variable::new("pi_value".to_string(), None, Some("=PI()".to_string())),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let pi_val = result.scalars.get("pi_value").unwrap().value.unwrap();
        assert!((pi_val - std::f64::consts::PI).abs() < 0.000001);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_pi_in_formula() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "circle_area".to_string(),
            Variable::new(
                "circle_area".to_string(),
                None,
                Some("=PI() * POWER(5, 2)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let area = result.scalars.get("circle_area").unwrap().value.unwrap();
        assert!((area - (std::f64::consts::PI * 25.0)).abs() < 0.0001);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_pi_array() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "radius".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_row_formula(
            "circumference".to_string(),
            "=2 * PI() * radius".to_string(),
        );

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let result_table = result.tables.get("data").unwrap();

        let circumference = result_table.columns.get("circumference").unwrap();
        match &circumference.values {
            ColumnValue::Number(nums) => {
                assert!((nums[0] - 2.0 * std::f64::consts::PI).abs() < 0.0001);
                assert!((nums[1] - 4.0 * std::f64::consts::PI).abs() < 0.0001);
                assert!((nums[2] - 6.0 * std::f64::consts::PI).abs() < 0.0001);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_e_constant() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "e_value".to_string(),
            Variable::new("e_value".to_string(), None, Some("=E()".to_string())),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let e_val = result.scalars.get("e_value").unwrap().value.unwrap();
        assert!((e_val - std::f64::consts::E).abs() < 0.000001);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_e_in_formula() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "e_squared".to_string(),
            Variable::new(
                "e_squared".to_string(),
                None,
                Some("=POWER(E(), 2)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let e_sq = result.scalars.get("e_squared").unwrap().value.unwrap();
        assert!((e_sq - (std::f64::consts::E * std::f64::consts::E)).abs() < 0.0001);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_e_array() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "multiplier".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_row_formula("e_multiple".to_string(), "=E() * multiplier".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let result_table = result.tables.get("data").unwrap();

        let e_multiple = result_table.columns.get("e_multiple").unwrap();
        match &e_multiple.values {
            ColumnValue::Number(nums) => {
                assert!((nums[0] - std::f64::consts::E).abs() < 0.0001);
                assert!((nums[1] - 2.0 * std::f64::consts::E).abs() < 0.0001);
                assert!((nums[2] - 3.0 * std::f64::consts::E).abs() < 0.0001);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_pow_function_scalar() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=POW(2, 8)".to_string())),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        assert!((result.scalars.get("result").unwrap().value.unwrap() - 256.0).abs() < 0.0001);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_pow_function_array() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "base".to_string(),
            ColumnValue::Number(vec![2.0, 3.0, 5.0]),
        ));
        table.add_row_formula("cubed".to_string(), "=POW(base, 3)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let result_table = result.tables.get("data").unwrap();

        let cubed = result_table.columns.get("cubed").unwrap();
        match &cubed.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 8.0);
                assert_eq!(nums[1], 27.0);
                assert_eq!(nums[2], 125.0);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_pow_negative_exponent() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=POW(2, -2)".to_string())),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        assert!((result.scalars.get("result").unwrap().value.unwrap() - 0.25).abs() < 0.0001);
    }

    #[test]
    #[cfg(not(feature = "demo"))]
    fn test_pow_fractional_exponent() {
        let mut model = ParsedModel::new();

        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=POW(16, 0.5)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        assert!((result.scalars.get("result").unwrap().value.unwrap() - 4.0).abs() < 0.0001);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // MATH EDGE CASE TESTS (from tests/math_edge_cases.rs)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_mod_positive_positive() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=MOD(5, 3)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
    }

    #[test]
    fn test_mod_negative_positive() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=MOD(-5, 3)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_mod_positive_negative() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=MOD(5, -3)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(-1.0));
    }

    #[test]
    fn test_mod_negative_negative() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=MOD(-5, -3)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(-2.0));
    }

    #[test]
    fn test_double_negative() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=--5".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
    }

    #[test]
    fn test_triple_negative() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=---5".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(-5.0));
    }

    #[test]
    fn test_power_operator_basic() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=2^10".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1024.0));
    }

    #[test]
    fn test_power_operator_negative() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=2^(-1)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.5));
    }

    #[test]
    fn test_zero_power_zero() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=0^0".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_power_fractional() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=ROUND(POWER(2, 0.5), 5)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.41421));
    }

    #[test]
    fn test_negative_multiply_negative() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=(-1)*(-1)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_division_fraction() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=10/4".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2.5));
    }

    #[test]
    fn test_round_third() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=ROUND(1/3, 5)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.33333));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // NUMERIC EDGE CASE TESTS (from tests/numeric_edge_cases.rs)
    // ═══════════════════════════════════════════════════════════════════════════

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_int_positive_fraction() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=INT(5.9)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_int_negative_large_fraction() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=INT(-5.9)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(-6.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_int_negative_small_fraction() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=INT(-5.1)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(-6.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_trunc_positive_fraction() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=TRUNC(5.9)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_trunc_negative_large_fraction() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=TRUNC(-5.9)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(-5.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_trunc_negative_small_fraction() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=TRUNC(-5.1)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(-5.0));
    }

    #[test]
    fn test_round_half_even_low() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=ROUND(2.5, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[test]
    fn test_round_half_odd_low() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=ROUND(3.5, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(4.0));
    }

    #[test]
    fn test_round_half_negative() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=ROUND(-2.5, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(-3.0));
    }

    #[test]
    fn test_round_negative_precision_decimal() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=ROUND(1234.5, -2)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1200.0));
    }

    #[test]
    fn test_round_negative_precision_exact() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=ROUND(1250, -2)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1300.0));
    }

    #[test]
    fn test_ceiling_positive_fraction() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=CEILING(2.1, 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[test]
    fn test_ceiling_negative_fraction() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=CEILING(-2.1, 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(-2.0));
    }

    #[test]
    fn test_floor_positive_fraction() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=FLOOR(2.9, 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
    }

    #[test]
    fn test_floor_negative_fraction() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=FLOOR(-2.9, 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(-3.0));
    }

    #[test]
    fn test_sqrt_zero() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=SQRT(0)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_abs_negative_zero() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=ABS(-0)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_sign_zero() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=SIGN(0)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_sign_positive() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=SIGN(100)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_sign_negative() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=SIGN(-100)".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(-1.0));
    }
}
