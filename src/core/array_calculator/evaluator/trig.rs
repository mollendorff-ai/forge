//! Trigonometric functions: SIN, COS, TAN, ASIN, ACOS, ATAN, SINH, COSH, TANH, RADIANS, DEGREES
//!
//! DEMO functions (6): SIN, COS, TAN, ASIN, ACOS, ATAN
//! ENTERPRISE functions: SINH, COSH, TANH, RADIANS, DEGREES

use super::{evaluate, require_args, EvalContext, EvalError, Expr, Value};

/// Try to evaluate a trigonometric function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "SIN" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("SIN requires a number"))?;
            Value::Number(val.sin())
        },

        "COS" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("COS requires a number"))?;
            Value::Number(val.cos())
        },

        "TAN" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("TAN requires a number"))?;
            Value::Number(val.tan())
        },

        "ASIN" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ASIN requires a number"))?;
            if !(-1.0..=1.0).contains(&val) {
                return Err(EvalError::new("ASIN argument must be between -1 and 1"));
            }
            Value::Number(val.asin())
        },

        "ACOS" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ACOS requires a number"))?;
            if !(-1.0..=1.0).contains(&val) {
                return Err(EvalError::new("ACOS argument must be between -1 and 1"));
            }
            Value::Number(val.acos())
        },

        "ATAN" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("ATAN requires a number"))?;
            Value::Number(val.atan())
        },

        // ═══════════════════════════════════════════════════════════════════════════
        // ENTERPRISE FUNCTIONS (only in full build)
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(not(feature = "demo"))]
        "SINH" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("SINH requires a number"))?;
            Value::Number(val.sinh())
        },

        #[cfg(not(feature = "demo"))]
        "COSH" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("COSH requires a number"))?;
            Value::Number(val.cosh())
        },

        #[cfg(not(feature = "demo"))]
        "TANH" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("TANH requires a number"))?;
            Value::Number(val.tanh())
        },

        #[cfg(not(feature = "demo"))]
        "RADIANS" => {
            require_args(name, args, 1)?;
            let degrees = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("RADIANS requires a number"))?;
            Value::Number(degrees.to_radians())
        },

        #[cfg(not(feature = "demo"))]
        "DEGREES" => {
            require_args(name, args, 1)?;
            let radians = evaluate(&args[0], ctx)?
                .as_number()
                .ok_or_else(|| EvalError::new("DEGREES requires a number"))?;
            Value::Number(radians.to_degrees())
        },

        _ => return Ok(None),
    };

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::super::tests::eval;
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_sin() {
        let ctx = EvalContext::new();
        assert_eq!(eval("SIN(0)", &ctx).unwrap(), Value::Number(0.0));
        let result = eval("SIN(1.5707963267948966)", &ctx).unwrap(); // PI/2
        assert!(matches!(result, Value::Number(n) if (n - 1.0).abs() < 0.0001));
    }

    #[test]
    fn test_cos() {
        let ctx = EvalContext::new();
        assert_eq!(eval("COS(0)", &ctx).unwrap(), Value::Number(1.0));
        let result = eval("COS(3.141592653589793)", &ctx).unwrap(); // PI
        assert!(matches!(result, Value::Number(n) if (n - (-1.0)).abs() < 0.0001));
    }

    #[test]
    fn test_tan() {
        let ctx = EvalContext::new();
        assert_eq!(eval("TAN(0)", &ctx).unwrap(), Value::Number(0.0));
        let result = eval("TAN(0.7853981633974483)", &ctx).unwrap(); // PI/4
        assert!(matches!(result, Value::Number(n) if (n - 1.0).abs() < 0.0001));
    }

    #[test]
    fn test_asin() {
        let ctx = EvalContext::new();
        assert_eq!(eval("ASIN(0)", &ctx).unwrap(), Value::Number(0.0));
        let result = eval("ASIN(1)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n - PI / 2.0).abs() < 0.0001));
    }

    #[test]
    fn test_asin_out_of_range() {
        let ctx = EvalContext::new();
        assert!(eval("ASIN(2)", &ctx).is_err());
        assert!(eval("ASIN(-2)", &ctx).is_err());
    }

    #[test]
    fn test_acos() {
        let ctx = EvalContext::new();
        let result = eval("ACOS(1)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n.abs() < 0.0001));
        let result = eval("ACOS(0)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n - PI / 2.0).abs() < 0.0001));
    }

    #[test]
    fn test_acos_out_of_range() {
        let ctx = EvalContext::new();
        assert!(eval("ACOS(2)", &ctx).is_err());
        assert!(eval("ACOS(-2)", &ctx).is_err());
    }

    #[test]
    fn test_atan() {
        let ctx = EvalContext::new();
        assert_eq!(eval("ATAN(0)", &ctx).unwrap(), Value::Number(0.0));
        let result = eval("ATAN(1)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n - PI / 4.0).abs() < 0.0001));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ENTERPRISE TESTS (only with full feature)
    // ═══════════════════════════════════════════════════════════════════════════

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_sinh() {
        let ctx = EvalContext::new();
        assert_eq!(eval("SINH(0)", &ctx).unwrap(), Value::Number(0.0));
        let result = eval("SINH(1)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n - 1.1752011936438014).abs() < 0.0001));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_cosh() {
        let ctx = EvalContext::new();
        assert_eq!(eval("COSH(0)", &ctx).unwrap(), Value::Number(1.0));
        let result = eval("COSH(1)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n - 1.5430806348152437).abs() < 0.0001));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_tanh() {
        let ctx = EvalContext::new();
        assert_eq!(eval("TANH(0)", &ctx).unwrap(), Value::Number(0.0));
        let result = eval("TANH(1)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n - 0.7615941559557649).abs() < 0.0001));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_radians() {
        let ctx = EvalContext::new();
        let result = eval("RADIANS(180)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n - PI).abs() < 0.0001));
        let result = eval("RADIANS(90)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n - PI / 2.0).abs() < 0.0001));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_degrees() {
        let ctx = EvalContext::new();
        let result = eval("DEGREES(3.141592653589793)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n - 180.0).abs() < 0.0001));
        let result = eval("DEGREES(1.5707963267948966)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n - 90.0).abs() < 0.0001));
    }
}
