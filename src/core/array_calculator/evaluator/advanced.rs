//! Advanced functions: LAMBDA, LET, SWITCH

use super::{evaluate, values_equal, EvalContext, EvalError, Expr, Reference, Value};

/// Try to evaluate an advanced function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "LAMBDA" => {
            // LAMBDA(param1, param2, ..., body) - returns a lambda value
            if args.is_empty() {
                return Err(EvalError::new("LAMBDA requires at least a body"));
            }

            let mut params = Vec::new();
            for i in 0..args.len() - 1 {
                match &args[i] {
                    Expr::Reference(Reference::Scalar(name)) => {
                        params.push(name.clone());
                    }
                    _ => {
                        return Err(EvalError::new(format!(
                            "LAMBDA parameter {} must be an identifier",
                            i + 1
                        )));
                    }
                }
            }

            let body = args.last().unwrap().clone();

            Value::Lambda {
                params,
                body: Box::new(body),
            }
        }

        "LET" => {
            // LET(name1, value1, [name2, value2, ...], calculation)
            if args.len() < 3 || args.len().is_multiple_of(2) {
                return Err(EvalError::new(
                    "LET requires pairs of name/value plus a calculation",
                ));
            }

            let mut new_ctx = ctx.clone();

            let num_pairs = (args.len() - 1) / 2;
            for i in 0..num_pairs {
                let name_expr = &args[i * 2];
                let value_expr = &args[i * 2 + 1];

                let name = match name_expr {
                    Expr::Reference(Reference::Scalar(n)) => n.clone(),
                    _ => return Err(EvalError::new("LET variable name must be an identifier")),
                };

                let value = evaluate(value_expr, &new_ctx)?;
                new_ctx.scalars.insert(name, value);
            }

            evaluate(&args[args.len() - 1], &new_ctx)?
        }

        "SWITCH" => {
            // SWITCH(expression, value1, result1, [value2, result2], ..., [default])
            if args.len() < 2 {
                return Err(EvalError::new("SWITCH requires at least 2 arguments"));
            }

            let expr_val = evaluate(&args[0], ctx)?;
            let remaining = &args[1..];

            let mut i = 0;
            while i + 1 < remaining.len() {
                let check_val = evaluate(&remaining[i], ctx)?;
                if values_equal(&expr_val, &check_val) {
                    return Ok(Some(evaluate(&remaining[i + 1], ctx)?));
                }
                i += 2;
            }

            if remaining.len() % 2 == 1 {
                evaluate(&remaining[remaining.len() - 1], ctx)?
            } else {
                return Err(EvalError::new("SWITCH: No match found"));
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
    fn test_let() {
        let ctx = EvalContext::new();
        // LET(x, 10, y, 20, x + y)
        assert_eq!(
            eval("LET(x, 10, y, 20, x + y)", &ctx).unwrap(),
            Value::Number(30.0)
        );
    }

    #[test]
    fn test_switch() {
        let ctx = EvalContext::new();
        // SWITCH(2, 1, "one", 2, "two", 3, "three")
        assert_eq!(
            eval("SWITCH(2, 1, \"one\", 2, \"two\", 3, \"three\")", &ctx).unwrap(),
            Value::Text("two".to_string())
        );

        // SWITCH with default
        assert_eq!(
            eval("SWITCH(5, 1, \"one\", 2, \"two\", \"default\")", &ctx).unwrap(),
            Value::Text("default".to_string())
        );
    }

    #[test]
    fn test_lambda() {
        let ctx = EvalContext::new();
        // Create a lambda and call it: LAMBDA(x, x * 2)(5)
        let result = eval("LAMBDA(x, x * 2)(5)", &ctx).unwrap();
        assert_eq!(result, Value::Number(10.0));
    }

    #[test]
    fn test_nested_functions() {
        let ctx = EvalContext::new();
        // Test nested function calls still work
        assert_eq!(eval("LET(a, 5, a * 2)", &ctx).unwrap(), Value::Number(10.0));
    }
}
