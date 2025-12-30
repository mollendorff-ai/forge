//! Math functions: ABS, ROUND, SQRT, POW, EXP, LN, LOG, etc.
//!
//! DEMO functions (16): ROUND, ROUNDUP, ROUNDDOWN, ABS, SQRT, POWER, MOD, CEILING, FLOOR, EXP, LN, LOG10, INT, SIGN, TRUNC, PI
//! ENTERPRISE functions: POW, E, LOG, RAND, RANDBETWEEN

mod basic;
mod logarithm;
mod random;
mod rounding;

use super::{EvalContext, EvalError, Expr, Value};

/// Try to evaluate a math function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        // Basic math functions (DEMO)
        "ABS" => basic::eval_abs(args, ctx)?,
        "SQRT" => basic::eval_sqrt(args, ctx)?,
        "POWER" => basic::eval_power(args, ctx)?,
        "MOD" => basic::eval_mod(args, ctx)?,
        "SIGN" => basic::eval_sign(args, ctx)?,
        "PI" => basic::eval_pi(args, ctx)?,

        // Rounding functions (DEMO)
        "ROUND" => rounding::eval_round(args, ctx)?,
        "ROUNDUP" => rounding::eval_roundup(args, ctx)?,
        "ROUNDDOWN" => rounding::eval_rounddown(args, ctx)?,
        "FLOOR" => rounding::eval_floor(args, ctx)?,
        "CEILING" => rounding::eval_ceiling(args, ctx)?,
        "TRUNC" => rounding::eval_trunc(args, ctx)?,
        "INT" => rounding::eval_int(args, ctx)?,

        // Logarithmic functions (DEMO)
        "EXP" => logarithm::eval_exp(args, ctx)?,
        "LN" => logarithm::eval_ln(args, ctx)?,
        "LOG10" => logarithm::eval_log10(args, ctx)?,

        // ENTERPRISE FUNCTIONS
        "LOG" => logarithm::eval_log(args, ctx)?,
        "POW" => basic::eval_pow(args, ctx)?,
        "E" => basic::eval_e(args, ctx)?,
        "RAND" => random::eval_rand(args, ctx)?,
        "RANDBETWEEN" => random::eval_randbetween(args, ctx)?,

        _ => return Ok(None),
    };

    Ok(Some(result))
}
