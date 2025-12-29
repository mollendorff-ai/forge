//! Conditional aggregation functions: SUMIF, COUNTIF, AVERAGEIF, SUMIFS, etc.
//!
//! This module provides Excel-compatible conditional aggregation functions:
//!
//! - **Single criteria**: SUMIF, COUNTIF, AVERAGEIF
//! - **Multiple criteria**: SUMIFS, COUNTIFS, AVERAGEIFS
//! - **Min/Max with criteria**: MINIFS, MAXIFS
//!
//! All functions support comparison operators in criteria strings:
//! `>`, `<`, `>=`, `<=`, `<>`, `=`

mod averageif;
mod countif;
mod minmax;
mod sumif;

use super::{EvalContext, EvalError, Expr, Value};

/// Try to evaluate a conditional aggregation function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "SUMIF" => sumif::eval_sumif(args, ctx)?,
        "SUMIFS" => sumif::eval_sumifs(args, ctx)?,

        "COUNTIF" => countif::eval_countif(args, ctx)?,
        "COUNTIFS" => countif::eval_countifs(args, ctx)?,

        "AVERAGEIF" => averageif::eval_averageif(args, ctx)?,
        "AVERAGEIFS" => averageif::eval_averageifs(args, ctx)?,

        "MAXIFS" => minmax::eval_maxifs(args, ctx)?,
        "MINIFS" => minmax::eval_minifs(args, ctx)?,

        _ => return Ok(None),
    };

    Ok(Some(result))
}
