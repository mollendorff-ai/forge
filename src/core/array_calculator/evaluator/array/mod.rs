//! Array functions: UNIQUE, COUNTUNIQUE, SORT, FILTER, SEQUENCE, RANDARRAY

mod filter;
mod generators;
mod sort;
mod unique;

use super::{EvalContext, EvalError, Expr, Value};

/// Try to evaluate an array function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "UNIQUE" => unique::eval_unique(args, ctx)?,
        "COUNTUNIQUE" => unique::eval_countunique(args, ctx)?,
        "SORT" => sort::eval_sort(args, ctx)?,
        "FILTER" => filter::eval_filter(args, ctx)?,
        "SEQUENCE" => generators::eval_sequence(args, ctx)?,
        "RANDARRAY" => generators::eval_randarray(args, ctx)?,
        _ => return Ok(None),
    };

    Ok(Some(result))
}
