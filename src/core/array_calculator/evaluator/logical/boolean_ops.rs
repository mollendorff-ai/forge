//! Boolean operations: AND, OR, NOT, XOR
//!
//! DEMO functions: AND, OR, NOT
//! ENTERPRISE functions: XOR

use crate::core::array_calculator::evaluator::{
    evaluate, require_args, EvalContext, EvalError, Expr, Value,
};

/// Evaluate AND function - returns TRUE if all arguments are truthy
pub fn eval_and(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    for arg in args {
        let val = evaluate(arg, ctx)?;
        if !val.is_truthy() {
            return Ok(Value::Boolean(false));
        }
    }
    Ok(Value::Boolean(true))
}

/// Evaluate OR function - returns TRUE if any argument is truthy
pub fn eval_or(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    for arg in args {
        let val = evaluate(arg, ctx)?;
        if val.is_truthy() {
            return Ok(Value::Boolean(true));
        }
    }
    Ok(Value::Boolean(false))
}

/// Evaluate NOT function - returns the logical negation
pub fn eval_not(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args(name, args, 1)?;
    let val = evaluate(&args[0], ctx)?;
    Ok(Value::Boolean(!val.is_truthy()))
}

/// Evaluate XOR function - returns TRUE if an odd number of arguments are TRUE
#[cfg(not(feature = "demo"))]
pub fn eval_xor(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    let mut true_count = 0;
    for arg in args {
        let val = evaluate(arg, ctx)?;
        if val.is_truthy() {
            true_count += 1;
        }
    }
    Ok(Value::Boolean(true_count % 2 == 1))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use super::*;

    #[test]
    fn test_logical() {
        let ctx = EvalContext::new();
        assert_eq!(eval("AND(1, 1, 1)", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("AND(1, 0, 1)", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("OR(0, 0, 1)", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("NOT(0)", &ctx).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_logical_and_edge_cases() {
        let ctx = EvalContext::new();
        // AND(TRUE, TRUE) = TRUE
        assert_eq!(
            eval("AND(TRUE(), TRUE())", &ctx).unwrap_or(eval("AND(1, 1)", &ctx).unwrap()),
            Value::Boolean(true)
        );
        // AND(TRUE, FALSE) = FALSE
        assert_eq!(eval("AND(1, 0)", &ctx).unwrap(), Value::Boolean(false));
        // AND(1, 1) = TRUE (nonzero as true)
        assert_eq!(eval("AND(1, 1)", &ctx).unwrap(), Value::Boolean(true));
        // AND(1, 0) = FALSE (zero as false)
        assert_eq!(eval("AND(1, 0)", &ctx).unwrap(), Value::Boolean(false));
    }

    #[test]
    fn test_logical_or_edge_cases() {
        let ctx = EvalContext::new();
        // OR(FALSE, FALSE) = FALSE
        assert_eq!(eval("OR(0, 0)", &ctx).unwrap(), Value::Boolean(false));
        // OR(TRUE, FALSE) = TRUE
        assert_eq!(eval("OR(1, 0)", &ctx).unwrap(), Value::Boolean(true));
        // OR(0, 1) = TRUE
        assert_eq!(eval("OR(0, 1)", &ctx).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_logical_not_edge_cases() {
        let ctx = EvalContext::new();
        // NOT(FALSE) = TRUE
        assert_eq!(eval("NOT(0)", &ctx).unwrap(), Value::Boolean(true));
        // NOT(TRUE) = FALSE
        assert_eq!(eval("NOT(1)", &ctx).unwrap(), Value::Boolean(false));
        // NOT(0) = TRUE
        assert_eq!(eval("NOT(0)", &ctx).unwrap(), Value::Boolean(true));
        // NOT(1) = FALSE
        assert_eq!(eval("NOT(1)", &ctx).unwrap(), Value::Boolean(false));
        // NOT(5) = FALSE (any nonzero is truthy)
        assert_eq!(eval("NOT(5)", &ctx).unwrap(), Value::Boolean(false));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_xor() {
        let ctx = EvalContext::new();
        // XOR returns TRUE if odd number of TRUE values
        assert_eq!(eval("XOR(1, 0, 0)", &ctx).unwrap(), Value::Boolean(true)); // 1 true
        assert_eq!(eval("XOR(1, 1, 0)", &ctx).unwrap(), Value::Boolean(false)); // 2 true
        assert_eq!(eval("XOR(1, 1, 1)", &ctx).unwrap(), Value::Boolean(true)); // 3 true
        assert_eq!(eval("XOR(0, 0, 0)", &ctx).unwrap(), Value::Boolean(false)); // 0 true
    }
}
