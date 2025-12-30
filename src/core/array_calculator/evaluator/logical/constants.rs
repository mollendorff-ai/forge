//! Boolean constant functions: TRUE, FALSE
//!
//! ENTERPRISE functions: TRUE, FALSE

use crate::core::array_calculator::evaluator::{require_args, EvalError, Expr, Value};

/// Evaluate TRUE function - returns boolean true
pub fn eval_true(name: &str, args: &[Expr]) -> Result<Value, EvalError> {
    require_args(name, args, 0)?;
    Ok(Value::Boolean(true))
}

/// Evaluate FALSE function - returns boolean false
pub fn eval_false(name: &str, args: &[Expr]) -> Result<Value, EvalError> {
    require_args(name, args, 0)?;
    Ok(Value::Boolean(false))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use crate::core::array_calculator::evaluator::{EvalContext, Value};

    #[test]
    fn test_true_false() {
        let ctx = EvalContext::new();
        assert_eq!(eval("TRUE()", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("FALSE()", &ctx).unwrap(), Value::Boolean(false));
    }
}
