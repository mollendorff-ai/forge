//! Logical functions: IF, AND, OR, NOT, XOR, TRUE, FALSE, IFERROR, IFNA, IFS
//!
//! DEMO functions (5): IF, AND, OR, NOT, IFERROR
//! ENTERPRISE functions: IFNA, XOR, TRUE, FALSE, IFS

use super::{evaluate, require_args, require_args_range, EvalContext, EvalError, Expr, Value};

/// Try to evaluate a logical function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    // ═══════════════════════════════════════════════════════════════════════════
    // DEMO FUNCTIONS (always available)
    // ═══════════════════════════════════════════════════════════════════════════
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
        },

        "AND" => {
            for arg in args {
                let val = evaluate(arg, ctx)?;
                if !val.is_truthy() {
                    return Ok(Some(Value::Boolean(false)));
                }
            }
            Value::Boolean(true)
        },

        "OR" => {
            for arg in args {
                let val = evaluate(arg, ctx)?;
                if val.is_truthy() {
                    return Ok(Some(Value::Boolean(true)));
                }
            }
            Value::Boolean(false)
        },

        "NOT" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            Value::Boolean(!val.is_truthy())
        },

        "IFERROR" => {
            require_args(name, args, 2)?;
            match evaluate(&args[0], ctx) {
                Ok(val) => val,
                Err(_) => evaluate(&args[1], ctx)?,
            }
        },

        // ═══════════════════════════════════════════════════════════════════════════
        // ENTERPRISE FUNCTIONS (only in full build)
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(not(feature = "demo"))]
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
        },

        #[cfg(not(feature = "demo"))]
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
        },

        #[cfg(not(feature = "demo"))]
        "TRUE" => {
            require_args(name, args, 0)?;
            Value::Boolean(true)
        },

        #[cfg(not(feature = "demo"))]
        "FALSE" => {
            require_args(name, args, 0)?;
            Value::Boolean(false)
        },

        #[cfg(not(feature = "demo"))]
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
        },

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

    #[test]
    fn test_if_edge_cases() {
        let ctx = EvalContext::new();
        // IF(2, 100, 200) = 100 (nonzero condition)
        assert_eq!(eval("IF(2, 100, 200)", &ctx).unwrap(), Value::Number(100.0));
        // IF(0, 100, 200) = 200 (zero condition)
        assert_eq!(eval("IF(0, 100, 200)", &ctx).unwrap(), Value::Number(200.0));
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
    fn test_iferror_edge_cases_comprehensive() {
        let ctx = EvalContext::new();

        // Edge case 1: IFERROR(1/0, -1) = -1 (div by zero caught)
        assert_eq!(eval("IFERROR(1/0, -1)", &ctx).unwrap(), Value::Number(-1.0));

        // Edge case 2: IFERROR(5, -1) = 5 (no error)
        assert_eq!(eval("IFERROR(5, -1)", &ctx).unwrap(), Value::Number(5.0));

        // Edge case 3: IFERROR(10/2, -1) = 5 (division ok)
        assert_eq!(eval("IFERROR(10/2, -1)", &ctx).unwrap(), Value::Number(5.0));

        // Edge case 6: IFERROR(SQRT(-1), -1) = -1 (sqrt negative caught)
        assert_eq!(
            eval("IFERROR(SQRT(-1), -1)", &ctx).unwrap(),
            Value::Number(-1.0)
        );

        // Edge case 7: IFERROR(LOG10(0), -1) = -1 (log zero caught)
        assert_eq!(
            eval("IFERROR(LOG10(0), -1)", &ctx).unwrap(),
            Value::Number(-1.0)
        );

        // Edge case 8: IFERROR(LN(0), -1) = -1 (ln zero caught)
        assert_eq!(
            eval("IFERROR(LN(0), -1)", &ctx).unwrap(),
            Value::Number(-1.0)
        );

        // Edge case 12: IFERROR(MOD(5, 0), -1) = -1 (mod by zero caught)
        assert_eq!(
            eval("IFERROR(MOD(5, 0), -1)", &ctx).unwrap(),
            Value::Number(-1.0)
        );
    }

    #[test]
    fn test_iferror_nested_edge_case() {
        let ctx = EvalContext::new();

        // Edge case 9: IFERROR(IFERROR(1/0, 1/0), -99) = -99 (nested)
        // Inner IFERROR tries to catch 1/0 but fallback is also 1/0 (error)
        // Outer IFERROR catches that error and returns -99
        assert_eq!(
            eval("IFERROR(IFERROR(1/0, 1/0), -99)", &ctx).unwrap(),
            Value::Number(-99.0)
        );

        // Additional nested test with valid fallback
        assert_eq!(
            eval("IFERROR(IFERROR(1/0, 42), 99)", &ctx).unwrap(),
            Value::Number(42.0)
        );
    }

    #[test]
    fn test_if_lazy_evaluation() {
        let ctx = EvalContext::new();

        // Edge case 4: IF(FALSE, 1/0, 5) = 5 (lazy eval - false branch not evaluated)
        assert_eq!(eval("IF(FALSE, 1/0, 5)", &ctx).unwrap(), Value::Number(5.0));

        // Edge case 5: IF(TRUE, 10, 1/0) = 10 (lazy eval - false branch not evaluated)
        assert_eq!(
            eval("IF(TRUE, 10, 1/0)", &ctx).unwrap(),
            Value::Number(10.0)
        );

        // Additional lazy eval test: IF(0, SQRT(-1), 100) = 100
        // True branch (error) should not be evaluated when condition is false
        assert_eq!(
            eval("IF(0, SQRT(-1), 100)", &ctx).unwrap(),
            Value::Number(100.0)
        );

        // Additional lazy eval test: IF(1, 200, MOD(5, 0)) = 200
        // False branch (error) should not be evaluated when condition is true
        assert_eq!(
            eval("IF(1, 200, MOD(5, 0))", &ctx).unwrap(),
            Value::Number(200.0)
        );

        // Additional lazy eval test: IF(1, LN(0), 300) = error (true branch is evaluated and has error)
        let result = eval("IF(1, LN(0), 300)", &ctx);
        assert!(result.is_err());

        // Additional lazy eval test: IF(0, 400, LOG10(0)) = error (false branch is evaluated and has error)
        let result = eval("IF(0, 400, LOG10(0))", &ctx);
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ENTERPRISE TESTS (only with full feature)
    // ═══════════════════════════════════════════════════════════════════════════

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

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_true_false() {
        let ctx = EvalContext::new();
        assert_eq!(eval("TRUE()", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("FALSE()", &ctx).unwrap(), Value::Boolean(false));
    }

    #[cfg(not(feature = "demo"))]
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

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_ifs_no_match() {
        let ctx = EvalContext::new();
        // No matching condition returns error
        let result = eval("IFS(0, \"no\", 0, \"nope\")", &ctx);
        assert!(result.is_err());
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_ifs_invalid_args() {
        let ctx = EvalContext::new();
        // Odd number of args is invalid
        let result = eval("IFS(1, \"yes\", 0)", &ctx);
        assert!(result.is_err());
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_ifna() {
        let mut ctx = EvalContext::new();
        ctx.scalars.insert("valid".to_string(), Value::Number(10.0));
        // Non-null value returns the value
        assert_eq!(eval("IFNA(valid, 0)", &ctx).unwrap(), Value::Number(10.0));
    }
}
