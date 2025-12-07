//! Logical functions: IF, AND, OR, NOT, IFERROR

use super::{evaluate, require_args, require_args_range, EvalContext, EvalError, Expr, Value};

/// Try to evaluate a logical function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "IF" => {
            require_args_range(name, args, 2, 3)?;
            let condition = evaluate(&args[0], ctx)?;
            if condition.is_truthy() {
                evaluate(&args[1], ctx)?
            } else if args.len() > 2 {
                evaluate(&args[2], ctx)?
            } else {
                Value::Boolean(false)
            }
        }

        "AND" => {
            for arg in args {
                let val = evaluate(arg, ctx)?;
                if !val.is_truthy() {
                    return Ok(Some(Value::Boolean(false)));
                }
            }
            Value::Boolean(true)
        }

        "OR" => {
            for arg in args {
                let val = evaluate(arg, ctx)?;
                if val.is_truthy() {
                    return Ok(Some(Value::Boolean(true)));
                }
            }
            Value::Boolean(false)
        }

        "NOT" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Value::Boolean(!val.is_truthy())
        }

        "IFERROR" => {
            require_args(name, args, 2)?;
            match evaluate(&args[0], ctx) {
                Ok(val) => val,
                Err(_) => evaluate(&args[1], ctx)?,
            }
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
    fn test_if() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("IF(5 > 3, \"yes\", \"no\")", &ctx).unwrap(),
            Value::Text("yes".to_string())
        );
        assert_eq!(
            eval("IF(5 < 3, \"yes\", \"no\")", &ctx).unwrap(),
            Value::Text("no".to_string())
        );
    }

    #[test]
    fn test_logical() {
        let ctx = EvalContext::new();
        assert_eq!(eval("AND(1, 1, 1)", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("AND(1, 0, 1)", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("OR(0, 0, 1)", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("NOT(0)", &ctx).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_iferror() {
        let ctx = EvalContext::new();
        // Division by zero returns the fallback value
        assert_eq!(eval("IFERROR(1/0, 0)", &ctx).unwrap(), Value::Number(0.0));
        // No error returns the original value
        assert_eq!(eval("IFERROR(10/2, 0)", &ctx).unwrap(), Value::Number(5.0));
    }
}
