//! Date functions: TODAY, NOW, YEAR, MONTH, DAY, WEEKDAY, HOUR, MINUTE, SECOND, DATE, EDATE, EOMONTH, DATEDIF, DAYS, TIME, WORKDAY, etc.
//!
//! DEMO functions (7): TODAY, DATE, YEAR, MONTH, DAY, DATEDIF, EOMONTH
//! ENTERPRISE functions: NOW, WEEKDAY, HOUR, MINUTE, SECOND, TIME, DAYS, WORKDAY, EDATE, NETWORKDAYS, YEARFRAC

mod arithmetic;
mod components;
mod current;
mod datedif;
mod workdays;

use super::{evaluate, parse_date_value, require_args, EvalContext, EvalError, Expr, Value};
use chrono::Datelike;

use super::require_args_range;
use chrono::Timelike;

/// Try to evaluate a date function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    // Current date/time functions
    if let Some(result) = current::try_evaluate(name, args, ctx)? {
        return Ok(Some(result));
    }

    // Arithmetic functions (YEAR, MONTH, DAY, DATE, EDATE, EOMONTH, DAYS)
    if let Some(result) = arithmetic::try_evaluate(name, args, ctx)? {
        return Ok(Some(result));
    }

    // DATEDIF, YEARFRAC, TIME functions
    if let Some(result) = datedif::try_evaluate(name, args, ctx)? {
        return Ok(Some(result));
    }

    // Date components (WEEKDAY, HOUR, MINUTE, SECOND)
    if let Some(result) = components::try_evaluate(name, args, ctx)? {
        return Ok(Some(result));
    }

    // Workday functions (WORKDAY, NETWORKDAYS)
    if let Some(result) = workdays::try_evaluate(name, args, ctx)? {
        return Ok(Some(result));
    }

    Ok(None)
}
