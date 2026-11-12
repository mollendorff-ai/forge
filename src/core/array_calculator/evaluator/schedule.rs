//! Growth, loan and depreciation schedules: RRI, CUMIPMT, CUMPRINC, SYD and VDB

// Periods, types and lives are whole numbers held in f64, so comparing and counting
// them is exact.
#![allow(clippy::float_cmp, clippy::while_float)]

use super::financial::{annuity_ipmt, annuity_pmt};
use super::{evaluate, number_or, require_args, require_args_range};
use super::{EvalContext, EvalError, Expr, Value};

/// The argument at `index` as a number
fn number(name: &str, args: &[Expr], index: usize, ctx: &EvalContext) -> Result<f64, EvalError> {
    evaluate(&args[index], ctx)?
        .as_number()
        .ok_or_else(|| EvalError::new(format!("{name} needs a number as argument {}", index + 1)))
}

/// `#NUM!`, Excel's error for an argument outside what a function accepts
fn num_error(name: &str, why: &str) -> EvalError {
    EvalError::new(format!("{name}: {why} (#NUM!)"))
}

/// The total of `part` (interest or principal) of a loan's payments from
/// period `start` to `end`: `CUMIPMT` and `CUMPRINC`.
fn cumulative(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
    part: impl Fn(f64, f64, f64, f64, f64) -> f64,
) -> Result<f64, EvalError> {
    require_args(name, args, 6)?;
    let rate = number(name, args, 0, ctx)?;
    let nper = number(name, args, 1, ctx)?;
    let pv = number(name, args, 2, ctx)?;
    let start = number(name, args, 3, ctx)?.trunc();
    let end = number(name, args, 4, ctx)?.trunc();
    let kind = number(name, args, 5, ctx)?;
    if rate <= 0.0 || nper < 1.0 || pv <= 0.0 {
        return Err(num_error(
            name,
            "the rate, the number of periods and the loan must be above 0",
        ));
    }
    if start < 1.0 || end < start || end > nper.trunc() {
        return Err(num_error(
            name,
            "the periods must satisfy 1 <= start <= end <= number of periods",
        ));
    }
    if kind != 0.0 && kind != 1.0 {
        return Err(num_error(name, "the type must be 0 or 1"));
    }
    let nper = nper.trunc();
    let mut total = 0.0;
    let mut per = start;
    while per <= end {
        total += part(rate, per, nper, pv, kind);
        per += 1.0;
    }
    Ok(total)
}

/// Depreciation of one whole period by the declining balance, as `DDB` gives it
fn declining(cost: f64, salvage: f64, life: f64, period: f64, factor: f64) -> f64 {
    let mut rate = factor / life;
    let before = if rate >= 1.0 {
        rate = 1.0;
        if period == 1.0 {
            cost
        } else {
            0.0
        }
    } else {
        cost * (1.0 - rate).powf(period - 1.0)
    };
    let after = cost * (1.0 - rate).powf(period);
    let depreciation = if after < salvage {
        before - salvage
    } else {
        before - after
    };
    depreciation.max(0.0)
}

/// Depreciation over the first `periods` periods of an asset of this `cost`,
/// by the declining balance until the straight line over what is left is
/// larger, and then by the straight line. `remaining_life` is the life the
/// straight line counts from the start.
fn switching(
    cost: f64,
    salvage: f64,
    life: f64,
    remaining_life: f64,
    periods: f64,
    factor: f64,
) -> f64 {
    let whole = periods.ceil();
    let mut total = 0.0;
    let mut left = cost - salvage;
    let mut straight_line = None;
    let mut period = 1.0;
    while period <= whole {
        let mut term = straight_line.unwrap_or_else(|| {
            let fall = declining(cost, salvage, life, period, factor);
            let level = left / (remaining_life - (period - 1.0));
            if level > fall {
                straight_line = Some(level);
                level
            } else {
                left -= fall;
                fall
            }
        });
        if period == whole {
            // The last period counts only as far as `periods` reaches into it
            term *= periods + 1.0 - whole;
        }
        total += term;
        period += 1.0;
    }
    total
}

/// `VDB`: depreciation
/// between two points of an asset's life
fn vdb(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<f64, EvalError> {
    require_args_range(name, args, 5, 7)?;
    let cost = number(name, args, 0, ctx)?;
    let salvage = number(name, args, 1, ctx)?;
    let life = number(name, args, 2, ctx)?;
    let mut start = number(name, args, 3, ctx)?;
    let mut end = number(name, args, 4, ctx)?;
    let factor = if args.len() > 5 {
        number_or(&args[5], ctx, 2.0)?
    } else {
        2.0
    };
    let no_switch = if args.len() > 6 {
        match evaluate(&args[6], ctx)? {
            Value::Boolean(b) => b,
            other => other.as_number().is_some_and(|n| n != 0.0),
        }
    } else {
        false
    };
    if cost < 0.0
        || salvage < 0.0
        || life <= 0.0
        || start < 0.0
        || end < start
        || end > life
        || factor < 0.0
    {
        return Err(num_error(name, "an argument is out of range"));
    }
    if no_switch {
        let first = start.floor();
        let last = end.ceil();
        let mut total = 0.0;
        let mut period = first + 1.0;
        while period <= last {
            let mut term = declining(cost, salvage, life, period, factor);
            // The first and the last period count only the part inside start..end
            if period == first + 1.0 {
                term *= end.min(first + 1.0) - start;
            } else if period == last {
                term *= end + 1.0 - last;
            }
            total += term;
            period += 1.0;
        }
        return Ok(total);
    }
    let mut remaining_life = life;
    if start != start.floor() && factor > 1.0 && start >= life / 2.0 {
        let part = start - life / 2.0;
        start = life / 2.0;
        end -= part;
        remaining_life += 1.0;
    }
    let spent = switching(cost, salvage, life, remaining_life, start, factor);
    Ok(switching(
        cost - spent,
        salvage,
        life,
        life - start,
        end - start,
        factor,
    ))
}

/// Try to evaluate a schedule function. Returns None if not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "RRI" => {
            // The growth rate per period that takes pv to fv in nper periods
            require_args(name, args, 3)?;
            let nper = number(name, args, 0, ctx)?;
            let pv = number(name, args, 1, ctx)?;
            let fv = number(name, args, 2, ctx)?;
            if nper <= 0.0 {
                return Err(num_error(name, "the number of periods must be above 0"));
            }
            if pv == 0.0 {
                return Err(EvalError::new("RRI: the present value is 0 (#DIV/0!)"));
            }
            let ratio = fv / pv;
            if ratio < 0.0 {
                return Err(num_error(
                    name,
                    "the present and future values must have the same sign",
                ));
            }
            Value::Number(ratio.powf(1.0 / nper) - 1.0)
        },

        "SYD" => {
            require_args(name, args, 4)?;
            let cost = number(name, args, 0, ctx)?;
            let salvage = number(name, args, 1, ctx)?;
            let life = number(name, args, 2, ctx)?;
            let per = number(name, args, 3, ctx)?;
            if life <= 0.0 || per <= 0.0 || per > life {
                return Err(num_error(name, "the period must be between 1 and the life"));
            }
            Value::Number((cost - salvage) * (life - per + 1.0) * 2.0 / (life * (life + 1.0)))
        },

        "CUMIPMT" => Value::Number(cumulative(name, args, ctx, |rate, per, nper, pv, kind| {
            annuity_ipmt(rate, per, nper, pv, 0.0, kind)
        })?),

        "CUMPRINC" => Value::Number(cumulative(name, args, ctx, |rate, per, nper, pv, kind| {
            annuity_pmt(rate, nper, pv, 0.0, kind) - annuity_ipmt(rate, per, nper, pv, 0.0, kind)
        })?),

        "VDB" => Value::Number(vdb(name, args, ctx)?),

        _ => return Ok(None),
    };
    Ok(Some(result))
}
