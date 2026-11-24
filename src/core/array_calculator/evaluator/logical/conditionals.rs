//! Conditional functions: IF, IFERROR, IFNA, IFS
//!
//! Functions: IF, IFERROR, IFNA, IFS

use crate::core::array_calculator::evaluator::{
    evaluate, is_na_error, require_args, require_args_range, EvalContext, EvalError, Expr, Value,
};

/// Evaluate IF function - conditional evaluation
pub fn eval_if(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range(name, args, 2, 3)?;
    let condition = evaluate(&args[0], ctx)?;
    if let Value::Array(conditions) = &condition {
        if conditions.len() != 1 {
            return eval_if_array(conditions, args, ctx);
        }
    }
    if condition.is_truthy() {
        evaluate(&args[1], ctx)
    } else if args.len() > 2 {
        evaluate(&args[2], ctx)
    } else {
        Ok(Value::Boolean(false))
    }
}

/// Evaluate IF with an array condition, choosing a result for each element.
///
/// With no else branch a false element becomes Null, which aggregates skip,
/// so `AVERAGE(IF(a > 1, a))` averages only the matching values.
///
/// Excel and `LibreOffice` give each element its own result, so an error in
/// one element of a branch is an error only where that branch is chosen:
/// `SUM(IF(a > 0, 10 / a, 0))` with a = 0, 1, 2 is 15, and a branch no
/// element chooses is never an error. A branch that is an error as a whole
/// is evaluated again for each element that chooses it, at that element's
/// row (Forge gave the error, as it has no error inside an array).
fn eval_if_array(
    conditions: &[Value],
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    let rows = conditions.len();
    let when_true = evaluate(&args[1], ctx);
    let when_false = args.get(2).map(|arg| (arg, evaluate(arg, ctx)));
    let mut by_row = None;
    conditions
        .iter()
        .enumerate()
        .map(|(row, condition)| {
            let mut chosen = |arg, whole| chosen_element(arg, whole, row, rows, ctx, &mut by_row);
            if condition.is_truthy() {
                chosen(&args[1], &when_true)
            } else if let Some((arg, whole)) = &when_false {
                chosen(arg, whole)
            } else {
                Ok(Value::Null)
            }
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|items| Value::Array(items.into()))
}

/// The element at `row` of the branch `arg`, whose value as a whole is
/// `whole`: when that is an error, the branch at that row alone (in a context
/// made once, in `by_row`), unless a row formula is already at one row
fn chosen_element(
    arg: &Expr,
    whole: &Result<Value, EvalError>,
    row: usize,
    rows: usize,
    ctx: &EvalContext,
    by_row: &mut Option<EvalContext>,
) -> Result<Value, EvalError> {
    match whole {
        Ok(value) => element(value, row, rows),
        Err(error) if ctx.row().is_some() => Err(error.clone()),
        Err(_) => {
            let row_ctx = by_row.get_or_insert_with(|| {
                let mut row_ctx = ctx.clone();
                row_ctx.row_count = Some(rows);
                row_ctx.hidden_row.set(0);
                row_ctx.whole_args.borrow_mut().clear();
                row_ctx
            });
            row_ctx.current_row = Some(row);
            element(&evaluate(arg, row_ctx)?, row, rows)
        },
    }
}

/// The element at `row` of a branch's value: an array of one value per
/// condition, one value for all, or a scalar
fn element(branch: &Value, row: usize, rows: usize) -> Result<Value, EvalError> {
    match branch {
        Value::Array(items) if items.len() == rows => Ok(items[row].clone()),
        Value::Array(items) if items.len() == 1 => Ok(items[0].clone()),
        Value::Array(items) => Err(EvalError::new(format!(
            "IF: condition has {rows} values but a result has {}",
            items.len()
        ))),
        scalar => Ok(scalar.clone()),
    }
}

/// Evaluate IFERROR function - returns fallback value on error
/// In Excel, IFERROR catches ALL errors including #N/A
pub fn eval_iferror(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args(name, args, 2)?;
    // An empty cell is no error: it stays empty, which reads as 0 (ADR-059)
    evaluate(&args[0], ctx).or_else(|_| evaluate(&args[1], ctx))
}

/// Evaluate IFNA function - returns fallback value when result is NA (null)
pub fn eval_ifna(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args(name, args, 2)?;
    // #N/A (NA() or a lookup that finds nothing) gives the fallback; any
    // other error stays an error, as in Excel
    match evaluate(&args[0], ctx) {
        Err(e) if is_na_error(&e) => evaluate(&args[1], ctx),
        other => other,
    }
}

/// Evaluate IFS function - multiple condition checks
pub fn eval_ifs(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    // IFS(condition1, value1, condition2, value2, ...)
    // Returns the value corresponding to the first TRUE condition
    if args.is_empty() || !args.len().is_multiple_of(2) {
        return Err(EvalError::new(
            "IFS requires an even number of arguments (condition, value pairs)",
        ));
    }
    for pair in args.chunks(2) {
        let condition = evaluate(&pair[0], ctx)?;
        if matches!(&condition, Value::Array(items) if items.len() != 1) {
            return Err(EvalError::new("IFS: a condition must be a single value"));
        }
        if condition.is_truthy() {
            return evaluate(&pair[1], ctx);
        }
    }
    Err(EvalError::new(
        "#N/A: IFS: no condition is TRUE (consider adding TRUE as the last condition)",
    ))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use super::*;
    use crate::core::array_calculator::evaluator::tests::array_ctx;

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
    fn test_iferror_catches_na() {
        let ctx = EvalContext::new();
        // IFERROR should catch NA() as an error (Excel-compatible)
        assert_eq!(
            eval("IFERROR(NA(), 10)", &ctx).unwrap(),
            Value::Number(10.0)
        );
        // Nested: IFERROR catches NA, returns 10, ISERROR(10) = FALSE
        assert_eq!(
            eval("IF(ISERROR(IFERROR(NA(), 10)), 1, 0)", &ctx).unwrap(),
            Value::Number(0.0)
        );
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

    #[test]
    fn test_if_with_array_condition() {
        let ctx = array_ctx();
        assert_eq!(
            eval("SUM(IF(p.a > 1, p.b, 0))", &ctx).unwrap(),
            Value::Number(50.0)
        );
        assert_eq!(
            eval("SUM(IF(p.a > 1, 1, p.b))", &ctx).unwrap(),
            Value::Number(12.0)
        );
        // No else branch: false elements are skipped, not counted as zero
        assert_eq!(
            eval("AVERAGE(IF(p.a > 1, p.b))", &ctx).unwrap(),
            Value::Number(25.0)
        );
    }

    /// A condition of one value is that value; with no else, false is FALSE
    #[test]
    fn a_one_value_condition_and_no_else() {
        use crate::core::array_calculator::evaluator::tests as h;
        h::is_exactly("IF({TRUE}, 1, 2)", 1.0);
        assert!(!h::truth("IF(FALSE, 1)"));
    }

    /// With a condition of several values, a one-value result is used for
    /// each, a result of as many values is taken value by value, and a result
    /// of another length is an error; an error in a result is the error
    #[test]
    fn array_conditions_and_their_results() {
        use crate::core::array_calculator::evaluator::tests as h;
        use crate::core::array_calculator::evaluator::{EvalContext, Value};
        let ctx = EvalContext::new();
        let n = Value::Number;
        assert_eq!(
            h::eval("IF({1, 0}, {5}, {7, 8})", &ctx).unwrap(),
            Value::Array(vec![n(5.0), n(8.0)].into())
        );
        h::fails_with(
            "IF({1, 0, 1}, {1, 2}, 0)",
            "IF: condition has 3 values but a result has 2",
        );
        h::fails_with("IF({1, 0}, 1/0, 2)", "Division by zero");
        h::fails_with("IF({1, 0}, 1, 1/0)", "Division by zero");
        h::fails_with("IFS(1/0, 1)", "Division by zero");
    }

    /// An array IF chooses each element's result, so an error in a branch
    /// is an error only where that branch is chosen, as in Excel and
    /// `LibreOffice`: with a = 0, 1, 2, `10 / a` is 10 and 5 in the rows that
    /// choose it (15 by hand), and `1 / 0` that no row chooses is no error
    #[test]
    fn an_error_in_a_branch_counts_only_where_it_is_chosen() {
        use crate::core::array_calculator::evaluator::tests as h;
        let ctx = h::table(&[("a", h::numbers(&[0.0, 1.0, 2.0]))]);
        h::is_exactly_in("SUM(IF(t.a > 0, 10 / t.a, 0))", &ctx, 15.0);
        h::is_exactly_in("SUM(IF(t.a = 0, 0, 10 / t.a))", &ctx, 15.0);
        h::is_exactly_in("SUM(IF(t.a > 5, 1 / 0, 1))", &ctx, 3.0);
        // A row that chooses the error is the error
        h::fails_in("SUM(IF(t.a >= 0, 10 / t.a, 0))", &ctx, "Division by zero");
        // In a row formula, a whole column read by SUM is read the same way
        let row = ctx.with_row(0, 3);
        h::is_exactly_in("SUM(IF(t.a > 0, 10 / t.a, 0))", &row, 15.0);
        // An array condition in a row formula, outside a whole column: its
        // branches are already at the formula's row, so their error stays
        h::fails_in("IF({1, 0}, 1 / t.a, 2)", &row, "Division by zero");
    }
}
