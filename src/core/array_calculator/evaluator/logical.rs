//! Logical functions: IF, AND, OR, NOT, XOR, TRUE, FALSE, IFERROR, IFNA, IFS

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

        "IFNA" => {
            require_args(name, args, 2)?;
            let val = evaluate(&args[0], ctx)?;
            // In Excel, IFNA returns value_if_na when the result is #N/A
            // Since we don't have a proper NA error type, treat Null as NA
            if matches!(val, Value::Null) {
                evaluate(&args[1], ctx)?
            } else {
                val
            }
        }

        "XOR" => {
            // XOR returns TRUE if an odd number of arguments are TRUE
            let mut true_count = 0;
            for arg in args {
                let val = evaluate(arg, ctx)?;
                if val.is_truthy() {
                    true_count += 1;
                }
            }
            Value::Boolean(true_count % 2 == 1)
        }

        "TRUE" => {
            require_args(name, args, 0)?;
            Value::Boolean(true)
        }

        "FALSE" => {
            require_args(name, args, 0)?;
            Value::Boolean(false)
        }

        "IFS" => {
            // IFS(condition1, value1, condition2, value2, ...)
            // Returns the value corresponding to the first TRUE condition
            if args.is_empty() || !args.len().is_multiple_of(2) {
                return Err(EvalError::new(
                    "IFS requires an even number of arguments (condition, value pairs)",
                ));
            }
            for pair in args.chunks(2) {
                let condition = evaluate(&pair[0], ctx)?;
                if condition.is_truthy() {
                    return Ok(Some(evaluate(&pair[1], ctx)?));
                }
            }
            return Err(EvalError::new(
                "IFS: No matching condition found (consider adding TRUE as final condition)",
            ));
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

    #[test]
    fn test_xor() {
        let ctx = EvalContext::new();
        // XOR returns TRUE if odd number of TRUE values
        assert_eq!(eval("XOR(1, 0, 0)", &ctx).unwrap(), Value::Boolean(true)); // 1 true
        assert_eq!(eval("XOR(1, 1, 0)", &ctx).unwrap(), Value::Boolean(false)); // 2 true
        assert_eq!(eval("XOR(1, 1, 1)", &ctx).unwrap(), Value::Boolean(true)); // 3 true
        assert_eq!(eval("XOR(0, 0, 0)", &ctx).unwrap(), Value::Boolean(false)); // 0 true
    }

    #[test]
    fn test_true_false() {
        let ctx = EvalContext::new();
        assert_eq!(eval("TRUE()", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("FALSE()", &ctx).unwrap(), Value::Boolean(false));
    }

    #[test]
    fn test_ifs() {
        let ctx = EvalContext::new();
        // First matching condition returns its value
        assert_eq!(
            eval(
                "IFS(5>10, \"big\", 5>3, \"medium\", TRUE(), \"small\")",
                &ctx
            )
            .unwrap(),
            Value::Text("medium".to_string())
        );
        // First condition matches
        assert_eq!(
            eval("IFS(1, \"first\", 1, \"second\")", &ctx).unwrap(),
            Value::Text("first".to_string())
        );
    }

    #[test]
    fn test_ifs_no_match() {
        let ctx = EvalContext::new();
        // No matching condition returns error
        let result = eval("IFS(0, \"no\", 0, \"nope\")", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_ifs_invalid_args() {
        let ctx = EvalContext::new();
        // Odd number of args is invalid
        let result = eval("IFS(1, \"yes\", 0)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_ifna() {
        let mut ctx = EvalContext::new();
        ctx.scalars.insert("valid".to_string(), Value::Number(10.0));
        // Non-null value returns the value
        assert_eq!(eval("IFNA(valid, 0)", &ctx).unwrap(), Value::Number(10.0));
    }
}
