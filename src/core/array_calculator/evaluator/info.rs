//! Information functions: IS*, NA, TYPE, N

// Info function casts: TYPE returns f64 type codes from small bounded integers.
#![allow(clippy::cast_possible_truncation)]

use super::{evaluate, is_na_error, require_args, EvalContext, EvalError, Expr, Value};

/// Try to evaluate an info function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "ISBLANK" => {
            require_args(name, args, 1)?;
            // An IS function never fails: an error is FALSE here, as in Excel
            Value::Boolean(matches!(evaluate(&args[0], ctx), Ok(Value::Null)))
        },

        "ISERROR" => {
            require_args(name, args, 1)?;
            // Every error, #N/A among them; an empty cell is not one (ADR-059)
            Value::Boolean(evaluate(&args[0], ctx).is_err())
        },

        "ISNA" => {
            require_args(name, args, 1)?;
            // #N/A only: NA() and a lookup that finds nothing
            Value::Boolean(matches!(evaluate(&args[0], ctx), Err(e) if is_na_error(&e)))
        },

        "ISNUMBER" => {
            require_args(name, args, 1)?;
            Value::Boolean(matches!(evaluate(&args[0], ctx), Ok(Value::Number(_))))
        },

        "ISTEXT" => {
            require_args(name, args, 1)?;
            Value::Boolean(matches!(evaluate(&args[0], ctx), Ok(Value::Text(_))))
        },

        "ISLOGICAL" => {
            require_args(name, args, 1)?;
            Value::Boolean(matches!(evaluate(&args[0], ctx), Ok(Value::Boolean(_))))
        },

        "ISEVEN" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("ISEVEN requires a number"))?;
            let int_val = val.trunc() as i64;
            Value::Boolean(int_val % 2 == 0)
        },

        "ISODD" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("ISODD requires a number"))?;
            let int_val = val.trunc() as i64;
            Value::Boolean(int_val % 2 != 0)
        },

        "ISREF" => {
            // In Forge, references are resolved before we get here
            // For now, always return FALSE (we don't have unresolved refs)
            require_args(name, args, 1)?;
            Value::Boolean(false)
        },

        "ISFORMULA" => {
            // In Forge, formulas are always evaluated, so we can't easily detect them
            // This returns FALSE for now (would need metadata support)
            require_args(name, args, 1)?;
            Value::Boolean(false)
        },

        "NA" => {
            require_args(name, args, 0)?;
            // #N/A, an error like any other: IFERROR, IFNA, ISERROR and ISNA
            // see it, and a formula that reads it fails, as in Excel (an empty
            // cell, Value::Null, is not an error: ADR-059)
            return Err(EvalError::new("#N/A: NA()"));
        },

        "TYPE" => {
            require_args(name, args, 1)?;
            // Excel TYPE function returns:
            // 1 = number (an empty cell too), 2 = text, 4 = logical, 16 = error, 64 = array
            let type_num = match evaluate(&args[0], ctx) {
                Ok(Value::Number(_) | Value::Null) => 1.0,
                Ok(Value::Text(_)) => 2.0,
                Ok(Value::Boolean(_)) => 4.0,
                Ok(Value::Array(_)) => 64.0,
                Err(_) | Ok(Value::Lambda { .. }) => 16.0, // a Lambda has no Excel equivalent
            };
            Value::Number(type_num)
        },

        "N" => {
            require_args(name, args, 1)?;
            let val = evaluate(&args[0], ctx)?;
            // N converts value to number: numbers stay as-is, TRUE=1, FALSE=0, others=0
            let num = match val {
                Value::Number(n) => n,
                Value::Boolean(true) => 1.0,
                _ => 0.0, // FALSE, text, null, arrays all become 0
            };
            Value::Number(num)
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
    fn test_isnumber() {
        let ctx = EvalContext::new();
        assert_eq!(eval("ISNUMBER(5)", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(
            eval("ISNUMBER(\"text\")", &ctx).unwrap(),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_istext() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("ISTEXT(\"hello\")", &ctx).unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(eval("ISTEXT(123)", &ctx).unwrap(), Value::Boolean(false));
    }

    #[test]
    fn test_islogical() {
        let ctx = EvalContext::new();
        assert_eq!(
            eval("ISLOGICAL(TRUE())", &ctx).unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(
            eval("ISLOGICAL(FALSE())", &ctx).unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(eval("ISLOGICAL(1)", &ctx).unwrap(), Value::Boolean(false));
    }

    #[test]
    fn test_iseven() {
        let ctx = EvalContext::new();
        assert_eq!(eval("ISEVEN(4)", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("ISEVEN(5)", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("ISEVEN(0)", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("ISEVEN(-2)", &ctx).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_isodd() {
        let ctx = EvalContext::new();
        assert_eq!(eval("ISODD(5)", &ctx).unwrap(), Value::Boolean(true));
        assert_eq!(eval("ISODD(4)", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("ISODD(-3)", &ctx).unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_iserror() {
        let ctx = EvalContext::new();
        // Division by zero is an error
        assert_eq!(eval("ISERROR(1/0)", &ctx).unwrap(), Value::Boolean(true));
        // Valid expression is not an error
        assert_eq!(eval("ISERROR(1/2)", &ctx).unwrap(), Value::Boolean(false));
    }

    #[test]
    fn test_iserror_edge_cases() {
        let ctx = EvalContext::new();

        // Edge case 10: ISERROR(1/0) = TRUE
        assert_eq!(eval("ISERROR(1/0)", &ctx).unwrap(), Value::Boolean(true));

        // Edge case 11: ISERROR(5) = FALSE
        assert_eq!(eval("ISERROR(5)", &ctx).unwrap(), Value::Boolean(false));

        // Additional edge cases for ISERROR
        // ISERROR with SQRT of negative = TRUE
        assert_eq!(
            eval("ISERROR(SQRT(-1))", &ctx).unwrap(),
            Value::Boolean(true)
        );

        // ISERROR with LOG of zero = TRUE
        assert_eq!(
            eval("ISERROR(LOG10(0))", &ctx).unwrap(),
            Value::Boolean(true)
        );

        // ISERROR with LN of zero = TRUE
        assert_eq!(eval("ISERROR(LN(0))", &ctx).unwrap(), Value::Boolean(true));

        // ISERROR with MOD by zero = TRUE
        assert_eq!(
            eval("ISERROR(MOD(5, 0))", &ctx).unwrap(),
            Value::Boolean(true)
        );

        // ISERROR with valid calculation = FALSE
        assert_eq!(
            eval("ISERROR(10 * 2)", &ctx).unwrap(),
            Value::Boolean(false)
        );

        // ISERROR with valid function call = FALSE
        assert_eq!(
            eval("ISERROR(SQRT(16))", &ctx).unwrap(),
            Value::Boolean(false)
        );

        // ISERROR with text = FALSE
        assert_eq!(
            eval("ISERROR(\"text\")", &ctx).unwrap(),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_na() {
        let ctx = EvalContext::new();
        // NA() is the error #N/A, not an empty value (ADR-059: an empty cell
        // is Null, and the two must differ for IFERROR and ISBLANK)
        let error = eval("NA()", &ctx).unwrap_err().to_string();
        assert!(error.contains("#N/A"), "{error}");
    }

    #[test]
    fn test_isna() {
        let ctx = EvalContext::new();
        // ISNA sees the #N/A of NA()
        assert_eq!(eval("ISNA(NA())", &ctx).unwrap(), Value::Boolean(true));
        // Regular number is not NA
        assert_eq!(eval("ISNA(5)", &ctx).unwrap(), Value::Boolean(false));
    }

    #[test]
    fn test_iserror_detects_na() {
        let ctx = EvalContext::new();
        // ISERROR should detect NA() as an error (Excel-compatible behavior)
        // In Excel, #N/A is an error type and ISERROR returns TRUE for ALL errors
        assert_eq!(eval("ISERROR(NA())", &ctx).unwrap(), Value::Boolean(true));
        // Also test in IF context (common pattern)
        assert_eq!(
            eval("IF(ISERROR(NA()), 1, 0)", &ctx).unwrap(),
            Value::Number(1.0)
        );
    }

    #[test]
    fn test_isblank() {
        let ctx = EvalContext::new();
        // An error is not an empty cell (Excel: ISBLANK(NA()) is FALSE)
        assert_eq!(eval("ISBLANK(NA())", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("ISBLANK(0)", &ctx).unwrap(), Value::Boolean(false));
        assert_eq!(eval("ISBLANK(\"\")", &ctx).unwrap(), Value::Boolean(false));
        // Empty string is not blank
    }

    #[test]
    fn test_type() {
        let ctx = EvalContext::new();
        assert_eq!(eval("TYPE(5)", &ctx).unwrap(), Value::Number(1.0)); // number
        assert_eq!(eval("TYPE(\"text\")", &ctx).unwrap(), Value::Number(2.0)); // text
        assert_eq!(eval("TYPE(TRUE())", &ctx).unwrap(), Value::Number(4.0)); // logical
    }

    #[test]
    fn test_n() {
        let ctx = EvalContext::new();
        assert_eq!(eval("N(5)", &ctx).unwrap(), Value::Number(5.0));
        assert_eq!(eval("N(TRUE())", &ctx).unwrap(), Value::Number(1.0));
        assert_eq!(eval("N(FALSE())", &ctx).unwrap(), Value::Number(0.0));
        assert_eq!(eval("N(\"text\")", &ctx).unwrap(), Value::Number(0.0));
    }

    #[test]
    fn test_isref() {
        let ctx = EvalContext::new();
        // ISREF always returns FALSE in current implementation
        assert_eq!(eval("ISREF(5)", &ctx).unwrap(), Value::Boolean(false));
    }

    #[test]
    fn test_isformula() {
        let ctx = EvalContext::new();
        // ISFORMULA always returns FALSE in current implementation
        assert_eq!(eval("ISFORMULA(5)", &ctx).unwrap(), Value::Boolean(false));
    }

    /// Fixes for what the generated cases found (ADR-062) that the engines cannot
    /// check: answers Forge gives on its own, with the reason, and the exact
    /// values where every engine loses digits.
    mod generated_cases {
        use crate::core::array_calculator::evaluator::tests as h;
        use crate::core::array_calculator::evaluator::{EvalContext, Value};

        #[test]
        fn na_is_an_error_and_an_empty_cell_is_not() {
            h::is_exactly("TYPE(NA())", 16.0);
            assert_eq!(
                h::eval("ISBLANK(NA())", &EvalContext::new()).unwrap(),
                Value::Boolean(false)
            );
            h::fails_with("NA() + 1", "#N/A");
            h::is_exactly("IFNA(NA(), 5)", 5.0);
            h::fails_with("IFNA(1/0, 5)", "");
        }
    }
}
