//! Financial functions: PMT, FV, PV, NPV, IRR, NPER, RATE, SLN, DB, DDB, MIRR, XIRR, XNPV,
//! PPMT, IPMT, EFFECT, NOMINAL, PRICEDISC, YIELDDISC, ACCRINT

use super::{collect_numeric_values, evaluate, require_args, require_args_range};
use super::{EvalContext, EvalError, Expr, Value};
use crate::core::array_calculator::evaluator::day_count::{self, Basis};
use crate::core::array_calculator::evaluator::{number_or, parse_date_value};
use crate::core::precision;
use num_traits::ToPrimitive;

/// Return an error unless the cash flows contain both signs.
///
/// Without a negative and a positive flow there is no rate where NPV is zero,
/// which Excel reports as `#NUM!`.
fn require_sign_change(name: &str, values: &[f64]) -> Result<(), EvalError> {
    let has_positive = values.iter().any(|v| *v > 0.0);
    let has_negative = values.iter().any(|v| *v < 0.0);
    if has_positive && has_negative {
        Ok(())
    } else {
        Err(EvalError::new(format!(
            "{name} needs at least one positive and one negative cash flow"
        )))
    }
}

/// Rates checked for a sign change when Newton's method does not converge
const RATE_GRID: [f64; 22] = [
    -0.9999, -0.999, -0.99, -0.95, -0.9, -0.75, -0.5, -0.3, -0.2, -0.1, -0.05, -0.01, 0.01, 0.05,
    0.1, 0.2, 0.3, 0.5, 1.0, 3.0, 10.0, 100.0,
];

/// Find a rate above -100% where `npv_at(rate)` is zero.
///
/// Tries Newton's method from `guess` first. If that does not converge, it
/// looks for a sign change on a fixed grid of rates and bisects the bracket
/// closest to `guess`. `scale` (the size of the cash flows) sets the
/// tolerance. Returns an error rather than an unconverged rate.
fn solve_rate(
    name: &str,
    guess: f64,
    scale: f64,
    npv_at: impl Fn(f64) -> f64,
) -> Result<f64, EvalError> {
    let tolerance = 1e-9 * scale.max(1.0);
    let close_enough = |rate: f64| {
        let npv = npv_at(rate);
        npv.is_finite() && npv.abs() <= tolerance
    };

    // Newton's method with a numerical derivative
    let mut rate = if guess > -1.0 { guess } else { 0.1 };
    for _ in 0..100 {
        let npv = npv_at(rate);
        if !npv.is_finite() {
            break;
        }
        // A residual within the tolerance is not yet the root: with a large
        // loan the tolerance is large, so iterate until the step is nothing
        // (RATE(276, -100, 1E15) stopped at -0.0725; the root is -0.0951)
        if npv == 0.0 {
            return Ok(rate);
        }
        let step = 1e-6 * (1.0 + rate.abs());
        let slope = (npv_at(rate + step) - npv_at(rate - step)) / (2.0 * step);
        if !slope.is_finite() || slope == 0.0 {
            break;
        }
        let mut next = rate - npv / slope;
        if next <= -1.0 {
            // Stay above -100%: move halfway towards it instead
            next = (rate - 1.0) / 2.0;
        }
        if (next - rate).abs() <= 1e-15 * (1.0 + rate.abs()) {
            rate = next;
            break;
        }
        rate = next;
    }
    if close_enough(rate) {
        return Ok(rate);
    }

    // Bracketing: find sign changes on a grid and bisect the nearest one
    let points: Vec<(f64, f64)> = RATE_GRID
        .iter()
        .map(|&r| (r, npv_at(r)))
        .filter(|(_, npv)| npv.is_finite())
        .collect();
    let bracket = points
        .windows(2)
        .filter(|pair| pair[0].1.signum() != pair[1].1.signum())
        .map(|pair| (pair[0], pair[1]))
        .min_by(|a, b| {
            let distance = |pair: &((f64, f64), (f64, f64))| {
                let mid = f64::midpoint(pair.0 .0, pair.1 .0);
                (mid - guess).abs()
            };
            distance(a).total_cmp(&distance(b))
        });
    if let Some(((mut low, mut low_npv), (mut high, _))) = bracket {
        for _ in 0..200 {
            let mid = f64::midpoint(low, high);
            let mid_npv = npv_at(mid);
            if !mid_npv.is_finite() {
                break;
            }
            if mid_npv.abs() <= tolerance || (high - low).abs() <= 1e-14 {
                return Ok(mid);
            }
            if mid_npv.signum() == low_npv.signum() {
                low = mid;
                low_npv = mid_npv;
            } else {
                high = mid;
            }
        }
    }

    Err(EvalError::new(format!(
        "{name} did not converge: no rate found where NPV is zero"
    )))
}

/// Optional numeric argument `index`, or `default` when it is not given.
fn optional_number(
    args: &[Expr],
    index: usize,
    ctx: &EvalContext,
    default: f64,
) -> Result<f64, EvalError> {
    args.get(index)
        .map_or(Ok(default), |arg| number_or(arg, ctx, default))
}

/// Excel's `type` argument: 0 = payments at the end of each period, anything else = at the start.
fn payment_timing(args: &[Expr], index: usize, ctx: &EvalContext) -> Result<f64, EvalError> {
    Ok(if optional_number(args, index, ctx, 0.0)? == 0.0 {
        0.0
    } else {
        1.0
    })
}

/// `(1 + rate)^nper` and `(1 + rate)^nper - 1`, the second without the loss
/// of digits that subtracting 1 gives at a rate near 0 (`PMT(1e-9, 24, …)`).
fn growth(rate: f64, nper: f64) -> (f64, f64) {
    let exponent = nper * rate.ln_1p();
    (exponent.exp(), exponent.exp_m1())
}

/// Future value of a present value and a level payment (Excel `FV`).
fn annuity_fv(rate: f64, nper: f64, pmt: f64, pv: f64, due: f64) -> f64 {
    if rate == 0.0 {
        return -pmt.mul_add(nper, pv);
    }
    let (growth, grown) = growth(rate, nper);
    -pv.mul_add(growth, pmt * due.mul_add(rate, 1.0) * grown / rate)
}

/// Present value of a future value and a level payment (Excel `PV`).
fn annuity_pv(rate: f64, nper: f64, pmt: f64, fv: f64, due: f64) -> f64 {
    if rate == 0.0 {
        return -pmt.mul_add(nper, fv);
    }
    let (growth, grown) = growth(rate, nper);
    -(pmt * due.mul_add(rate, 1.0)).mul_add(grown / rate, fv) / growth
}

/// Level payment that turns `pv` into `fv` over `nper` periods (Excel `PMT`).
pub(super) fn annuity_pmt(rate: f64, nper: f64, pv: f64, fv: f64, due: f64) -> f64 {
    if rate == 0.0 {
        return -(pv + fv) / nper;
    }
    let (growth, grown) = growth(rate, nper);
    -pv.mul_add(growth, fv) * rate / (grown * due.mul_add(rate, 1.0))
}

/// Interest part of the payment in period `per` (Excel `IPMT`).
pub(super) fn annuity_ipmt(rate: f64, per: f64, nper: f64, pv: f64, fv: f64, due: f64) -> f64 {
    if rate == 0.0 || (due != 0.0 && (per - 1.0).abs() < f64::EPSILON) {
        return 0.0;
    }
    let pmt = annuity_pmt(rate, nper, pv, fv, due);
    let interest = annuity_fv(rate, per - 1.0, pmt, pv, due) * rate;
    if due == 0.0 {
        interest
    } else {
        interest / (1.0 + rate)
    }
}

/// A date argument given as an Excel serial number or a date string
fn date_arg(arg: &Expr, ctx: &EvalContext, what: &str) -> Result<chrono::NaiveDate, EvalError> {
    parse_date_value(&evaluate(arg, ctx)?)
        .map_err(|e| EvalError::new(format!("{what}: {}", e.message)))
}

fn basis_arg(
    args: &[Expr],
    index: usize,
    ctx: &EvalContext,
    name: &str,
) -> Result<Basis, EvalError> {
    Basis::from_number(optional_number(args, index, ctx, 0.0)?, name)
}

/// The cash flows and dates of XNPV and XIRR. A pair whose cash flow is an
/// empty cell is left out with its date, as `LibreOffice` does for both and
/// Gnumeric for XIRR (ADR-059).
fn dated_cash_flows(
    name: &str,
    values: &Expr,
    dates: &Expr,
    ctx: &EvalContext,
) -> Result<(Vec<f64>, Vec<f64>), EvalError> {
    let dates = match evaluate(dates, ctx)? {
        Value::Array(arr) => std::sync::Arc::unwrap_or_clone(arr),
        Value::Number(n) => vec![Value::Number(n)],
        _ => return Err(EvalError::new(format!("{name} requires dates"))),
    };
    let values = match evaluate(values, ctx)? {
        Value::Array(arr) => std::sync::Arc::unwrap_or_clone(arr),
        single => vec![single],
    };
    if values.len() == dates.len() {
        let (kept_values, kept_dates) = values
            .iter()
            .zip(&dates)
            .filter(|(v, _)| !matches!(v, Value::Null))
            .filter_map(|(v, d)| Some((v.as_number()?, d.as_number()?)))
            .unzip();
        return Ok((kept_values, kept_dates));
    }
    Ok((
        values.iter().filter_map(Value::as_number).collect(),
        dates.iter().filter_map(Value::as_number).collect(),
    ))
}

/// Try to evaluate a financial function. Returns None if function not recognized.
#[allow(clippy::too_many_lines)]
// Splitting this 600-line match would fragment the function dispatch table.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    let result = match name {
        "PMT" => {
            require_args_range(name, args, 3, 5)?;
            let rate = number_or(&args[0], ctx, 0.0)?;
            let nper = number_or(&args[1], ctx, 0.0)?;
            let pv = number_or(&args[2], ctx, 0.0)?;
            let fv = optional_number(args, 3, ctx, 0.0)?;
            let due = payment_timing(args, 4, ctx)?;
            Value::Number(annuity_pmt(rate, nper, pv, fv, due))
        },

        "FV" => {
            require_args_range(name, args, 3, 5)?;
            let rate = number_or(&args[0], ctx, 0.0)?;
            let nper = number_or(&args[1], ctx, 0.0)?;
            let pmt = number_or(&args[2], ctx, 0.0)?;
            let pv = optional_number(args, 3, ctx, 0.0)?;
            let due = payment_timing(args, 4, ctx)?;
            Value::Number(annuity_fv(rate, nper, pmt, pv, due))
        },

        "PV" => {
            require_args_range(name, args, 3, 5)?;
            let rate = number_or(&args[0], ctx, 0.0)?;
            let nper = number_or(&args[1], ctx, 0.0)?;
            let pmt = number_or(&args[2], ctx, 0.0)?;
            let fv = optional_number(args, 3, ctx, 0.0)?;
            let due = payment_timing(args, 4, ctx)?;
            Value::Number(annuity_pv(rate, nper, pmt, fv, due))
        },

        "NPV" => {
            require_args_range(name, args, 2, 255)?;
            let rate = evaluate(&args[0], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("NPV rate must be a number"))?;

            let mut npv = 0.0;
            let mut period = 1;

            for arg in &args[1..] {
                let val = evaluate(arg, ctx)?;
                match val {
                    Value::Array(arr) => {
                        for v in arr.iter() {
                            if let Some(n) = v.as_number() {
                                npv += n / (1.0 + rate).powi(period);
                                period += 1;
                            }
                        }
                    },
                    Value::Number(n) => {
                        npv += n / (1.0 + rate).powi(period);
                        period += 1;
                    },
                    _ => {},
                }
            }

            Value::Number(npv)
        },

        "IRR" => {
            // IRR(values, [guess]). Older models pass the flows as separate
            // arguments, so a second argument is a guess only when the first
            // is an array of several values.
            let first = match args.first() {
                Some(arg) => evaluate(arg, ctx)?,
                None => return Err(EvalError::new("IRR requires cash flows")),
            };
            let has_guess = args.len() == 2 && matches!(&first, Value::Array(a) if a.len() > 1);
            let (values, guess) = if has_guess {
                let guess = evaluate(&args[1], ctx)?
                    .as_cell_number()
                    .ok_or_else(|| EvalError::new("IRR guess must be a number"))?;
                (collect_numeric_values(&args[..1], ctx)?, guess)
            } else {
                (collect_numeric_values(args, ctx)?, 0.1)
            };
            if values.is_empty() {
                return Err(EvalError::new("IRR requires cash flows"));
            }
            require_sign_change("IRR", &values)?;
            let scale: f64 = values.iter().map(|v| v.abs()).sum();
            let rate = solve_rate("IRR", guess, scale, |rate| {
                // Each cash flow's period, counted in an i32 for powi
                (0..=i32::MAX)
                    .zip(&values)
                    .map(|(i, cf)| cf / (1.0 + rate).powi(i))
                    .sum()
            })?;
            Value::Number(rate)
        },

        "XIRR" => {
            require_args_range(name, args, 2, 3)?;
            let (values, dates) = dated_cash_flows("XIRR", &args[0], &args[1], ctx)?;
            if values.len() != dates.len() || values.is_empty() {
                return Err(EvalError::new(
                    "XIRR: values and dates must have same length",
                ));
            }
            require_sign_change("XIRR", &values)?;
            let guess = if args.len() > 2 {
                evaluate(&args[2], ctx)?
                    .as_cell_number()
                    .ok_or_else(|| EvalError::new("XIRR guess must be a number"))?
            } else {
                0.1
            };
            let base_date = dates[0];
            let year_fracs: Vec<f64> = dates.iter().map(|d| (d - base_date) / 365.0).collect();
            let scale: f64 = values.iter().map(|v| v.abs()).sum();
            let rate = solve_rate("XIRR", guess, scale, |rate| {
                values
                    .iter()
                    .zip(&year_fracs)
                    .map(|(cf, t)| cf / (1.0 + rate).powf(*t))
                    .sum()
            })?;
            Value::Number(rate)
        },

        "XNPV" => {
            require_args(name, args, 3)?;
            let rate = evaluate(&args[0], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("XNPV requires rate"))?;
            let (values, dates) = dated_cash_flows("XNPV", &args[1], &args[2], ctx)?;
            if values.len() != dates.len() || values.is_empty() {
                return Err(EvalError::new(
                    "XNPV: values and dates must have same length",
                ));
            }
            let base_date = dates[0];
            let mut npv = 0.0;
            for (i, cf) in values.iter().enumerate() {
                let t = (dates[i] - base_date) / 365.0;
                npv += cf / (1.0 + rate).powf(t);
            }
            Value::Number(npv)
        },

        "NPER" => {
            require_args_range(name, args, 3, 5)?;
            let rate = evaluate(&args[0], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("NPER requires rate"))?;
            let pmt = evaluate(&args[1], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("NPER requires payment"))?;
            let pv = evaluate(&args[2], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("NPER requires present value"))?;
            let fv = optional_number(args, 3, ctx, 0.0)?;
            let due = payment_timing(args, 4, ctx)?;

            if rate == 0.0 {
                Value::Number(-(pv + fv) / pmt)
            } else {
                // ln((z - fv) / (z + pv)); when the ratio is near 1 (a rate
                // near 0, z large) as ln_1p of its distance from 1, which
                // keeps the digits that subtracting 1 would lose
                let z = pmt * due.mul_add(rate, 1.0) / rate;
                let off_one = -(fv + pv) / (z + pv);
                let log = if off_one.abs() < 0.5 {
                    off_one.ln_1p()
                } else {
                    ((z - fv) / (z + pv)).ln()
                };
                Value::Number(log / rate.ln_1p())
            }
        },

        "RATE" => {
            require_args_range(name, args, 3, 6)?;
            let nper = evaluate(&args[0], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("RATE requires nper"))?;
            let pmt = evaluate(&args[1], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("RATE requires payment"))?;
            let pv = evaluate(&args[2], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("RATE requires present value"))?;
            let fv = if args.len() > 3 {
                number_or(&args[3], ctx, 0.0)?
            } else {
                0.0
            };
            let pmt_type = if args.len() > 4 {
                number_or(&args[4], ctx, 0.0)?
            } else {
                0.0
            };
            let guess = if args.len() > 5 {
                number_or(&args[5], ctx, 0.1)?
            } else {
                0.1
            };
            let due: f64 = if pmt_type == 0.0 { 0.0 } else { 1.0 };
            let scale = pmt.abs().mul_add(nper.abs(), pv.abs() + fv.abs());
            let rate = solve_rate("RATE", guess, scale, |rate| {
                if rate.abs() < 1e-12 {
                    return pmt.mul_add(nper, pv) + fv;
                }
                let (growth, grown) = growth(rate, nper);
                pv.mul_add(growth, pmt * due.mul_add(rate, 1.0) * grown / rate) + fv
            })?;
            Value::Number(rate)
        },

        "SLN" => {
            require_args(name, args, 3)?;
            let cost = evaluate(&args[0], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("SLN requires cost"))?;
            let salvage = evaluate(&args[1], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("SLN requires salvage"))?;
            let life = evaluate(&args[2], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("SLN requires life"))?;

            if life == 0.0 {
                return Err(EvalError::new("SLN: life cannot be zero"));
            }
            Value::Number((cost - salvage) / life)
        },

        "DB" => {
            require_args_range(name, args, 4, 5)?;
            let cost = evaluate(&args[0], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("DB requires cost"))?;
            let salvage = evaluate(&args[1], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("DB requires salvage"))?;
            let life = evaluate(&args[2], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("DB requires life"))?;
            let period = evaluate(&args[3], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("DB requires period"))?;
            // Excel drops a fraction of a month: DB(10000, 1000, 5, 1, 6.5) is
            // 1845, the value for 6 (checked in Excel, 2026-11-14)
            let month = if args.len() > 4 {
                number_or(&args[4], ctx, 12.0)?.trunc()
            } else {
                12.0
            };

            // LibreOffice's checks, which Excel's documentation does not
            // contradict: a period past the life (and the part year after
            // it) has no depreciation to give (#NUM!)
            if cost <= 0.0
                || salvage < 0.0
                || salvage > cost
                || life <= 0.0
                || period <= 0.0
                || period > life + 1.0
                || !(1.0..=12.0).contains(&month)
            {
                return Err(EvalError::new(
                    "#NUM!: DB needs 0 <= salvage <= cost, a life and a period above 0, a period at most the life + 1, and a month from 1 to 12",
                ));
            }

            let rate = 1.0 - (salvage / cost).powf(1.0 / life);
            let rate = (rate * 1000.0).round() / 1000.0;

            let mut depreciation = 0.0;
            let mut remaining = cost;

            // Whole periods, as the period is cut; the part year after the
            // life is the one past its whole years. A period no i32 holds
            // needs a life of as many years (#NUM!).
            let Some(periods) = period.to_i32() else {
                return Err(EvalError::new(format!(
                    "#NUM!: DB: period {period} is more than it counts"
                )));
            };
            let part_year = life.to_i64().map(|years| years + 1);
            for p in 1..=periods {
                if p == 1 {
                    depreciation = cost * rate * month / 12.0;
                } else if Some(i64::from(p)) == part_year {
                    depreciation = remaining * rate * (12.0 - month) / 12.0;
                } else {
                    depreciation = remaining * rate;
                }
                remaining -= depreciation;
            }

            Value::Number(depreciation)
        },

        "DDB" => {
            require_args_range(name, args, 4, 5)?;
            let cost = evaluate(&args[0], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("DDB requires cost"))?;
            let salvage = evaluate(&args[1], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("DDB requires salvage"))?;
            let life = evaluate(&args[2], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("DDB requires life"))?;
            let period = evaluate(&args[3], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("DDB requires period"))?;
            let factor = if args.len() > 4 {
                number_or(&args[4], ctx, 2.0)?
            } else {
                2.0
            };

            if life == 0.0 {
                return Err(EvalError::new("DDB: life cannot be zero"));
            }
            // Excel: a period before the first or after the life is #NUM!
            if period <= 0.0 || period > life {
                return Err(EvalError::new(format!(
                    "#NUM!: DDB period must be from 1 to the life ({life}), got {period}"
                )));
            }

            // The closed form VDB uses, as both engines do: a fractional
            // period is read as it is, and once the value is down to the
            // salvage there is nothing more to take (the loop over whole
            // periods gave 1.10 for DDB(202.845, 35.9, 21, 18.7), where both
            // engines give 0)
            Value::Number(super::schedule::declining(
                cost, salvage, life, period, factor,
            ))
        },

        "MIRR" => {
            require_args(name, args, 3)?;
            let values = collect_numeric_values(&args[..1], ctx)?;
            let finance_rate = evaluate(&args[1], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("MIRR requires finance rate"))?;
            let reinvest_rate = evaluate(&args[2], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("MIRR requires reinvest rate"))?;

            if values.is_empty() {
                return Err(EvalError::new("MIRR requires cash flows"));
            }

            let n = precision::count(values.len());
            let mut npv_neg = 0.0;
            let mut npv_pos = 0.0;

            // Each cash flow's period and the periods after it to the last,
            // counted in an i32 for powi
            let last = i32::try_from(values.len() - 1).unwrap_or(i32::MAX);
            for (i, &cf) in (0..=last).zip(&values) {
                if cf < 0.0 {
                    npv_neg += cf / (1.0 + finance_rate).powi(i);
                } else {
                    npv_pos = cf.mul_add((1.0 + reinvest_rate).powi(last - i), npv_pos);
                }
            }

            if npv_neg == 0.0 || npv_pos == 0.0 {
                return Err(EvalError::new(
                    "MIRR requires both positive and negative cash flows",
                ));
            }

            let mirr = (-npv_pos / npv_neg).powf(1.0 / (n - 1.0)) - 1.0;
            Value::Number(mirr)
        },

        "PPMT" => {
            // Principal payment for a given period
            // PPMT(rate, per, nper, pv, [fv], [type])
            require_args_range(name, args, 4, 6)?;
            let rate = evaluate(&args[0], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("PPMT requires rate"))?;
            let per = evaluate(&args[1], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("PPMT requires period"))?;
            let nper = evaluate(&args[2], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("PPMT requires nper"))?;
            let pv = evaluate(&args[3], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("PPMT requires present value"))?;
            let fv = optional_number(args, 4, ctx, 0.0)?;
            let due = payment_timing(args, 5, ctx)?;
            // Excel: a period outside 1 to nper is #NUM!, whatever the rate
            if per < 1.0 || per > nper {
                return Err(EvalError::new(format!(
                    "#NUM!: PPMT period must be from 1 to nper ({nper}), got {per}"
                )));
            }
            let payment = annuity_pmt(rate, nper, pv, fv, due);
            Value::Number(payment - annuity_ipmt(rate, per, nper, pv, fv, due))
        },

        "IPMT" => {
            // Interest payment for a given period
            // IPMT(rate, per, nper, pv, [fv], [type])
            require_args_range(name, args, 4, 6)?;
            let rate = evaluate(&args[0], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("IPMT requires rate"))?;
            let per = evaluate(&args[1], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("IPMT requires period"))?;
            let nper = evaluate(&args[2], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("IPMT requires nper"))?;
            let pv = evaluate(&args[3], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("IPMT requires present value"))?;
            let fv = optional_number(args, 4, ctx, 0.0)?;
            let due = payment_timing(args, 5, ctx)?;
            // Excel: a period outside 1 to nper is #NUM!, whatever the rate
            if per < 1.0 || per > nper {
                return Err(EvalError::new(format!(
                    "#NUM!: IPMT period must be from 1 to nper ({nper}), got {per}"
                )));
            }
            Value::Number(annuity_ipmt(rate, per, nper, pv, fv, due))
        },

        "EFFECT" => {
            // Effective annual interest rate
            // EFFECT(nominal_rate, npery)
            require_args(name, args, 2)?;
            let nominal_rate = evaluate(&args[0], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("EFFECT requires nominal rate"))?;
            let npery = evaluate(&args[1], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("EFFECT requires periods per year"))?;
            // Excel truncates the periods per year to a whole number
            let npery = npery.trunc();

            if npery < 1.0 {
                return Err(EvalError::new("EFFECT: periods per year must be >= 1"));
            }
            if nominal_rate <= 0.0 {
                return Err(EvalError::new("EFFECT: nominal rate must be positive"));
            }

            let effect = (1.0 + nominal_rate / npery).powf(npery) - 1.0;
            Value::Number(effect)
        },

        "NOMINAL" => {
            // Nominal annual interest rate
            // NOMINAL(effect_rate, npery)
            require_args(name, args, 2)?;
            let effect_rate = evaluate(&args[0], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("NOMINAL requires effective rate"))?;
            let npery = evaluate(&args[1], ctx)?
                .as_cell_number()
                .ok_or_else(|| EvalError::new("NOMINAL requires periods per year"))?;
            // Excel truncates the periods per year to a whole number
            let npery = npery.trunc();

            if npery < 1.0 {
                return Err(EvalError::new("NOMINAL: periods per year must be >= 1"));
            }
            if effect_rate <= 0.0 {
                return Err(EvalError::new("NOMINAL: effective rate must be positive"));
            }

            let nominal = ((1.0 + effect_rate).powf(1.0 / npery) - 1.0) * npery;
            Value::Number(nominal)
        },

        "PRICEDISC" => {
            // PRICEDISC(settlement, maturity, discount, redemption, [basis])
            require_args_range(name, args, 4, 5)?;
            let settlement = date_arg(&args[0], ctx, "PRICEDISC settlement")?;
            let maturity = date_arg(&args[1], ctx, "PRICEDISC maturity")?;
            let discount = number_or(&args[2], ctx, 0.0)?;
            let redemption = number_or(&args[3], ctx, 0.0)?;
            let basis = basis_arg(args, 4, ctx, name)?;
            if settlement >= maturity {
                return Err(EvalError::new(
                    "#NUM!: PRICEDISC settlement must be before maturity",
                ));
            }
            if discount <= 0.0 || redemption <= 0.0 {
                return Err(EvalError::new(
                    "#NUM!: PRICEDISC discount and redemption must be positive",
                ));
            }
            // DSM / B, computed as YEARFRAC does (as LibreOffice does too)
            let fraction = day_count::year_fraction(settlement, maturity, basis);
            Value::Number((discount * redemption).mul_add(-fraction, redemption))
        },

        "YIELDDISC" => {
            // YIELDDISC(settlement, maturity, price, redemption, [basis])
            require_args_range(name, args, 4, 5)?;
            let settlement = date_arg(&args[0], ctx, "YIELDDISC settlement")?;
            let maturity = date_arg(&args[1], ctx, "YIELDDISC maturity")?;
            let price = number_or(&args[2], ctx, 0.0)?;
            let redemption = number_or(&args[3], ctx, 0.0)?;
            let basis = basis_arg(args, 4, ctx, name)?;
            if settlement >= maturity {
                return Err(EvalError::new(
                    "#NUM!: YIELDDISC settlement must be before maturity",
                ));
            }
            if price <= 0.0 || redemption <= 0.0 {
                return Err(EvalError::new(
                    "#NUM!: YIELDDISC price and redemption must be positive",
                ));
            }
            // DSM / B, computed as YEARFRAC does (as LibreOffice does too)
            let fraction = day_count::year_fraction(settlement, maturity, basis);
            Value::Number((redemption - price) / price / fraction)
        },

        "ACCRINT" => {
            // ACCRINT(issue, first_interest, settlement, rate, par, frequency, [basis])
            require_args_range(name, args, 6, 7)?;
            let issue = date_arg(&args[0], ctx, "ACCRINT issue")?;
            let first_interest = date_arg(&args[1], ctx, "ACCRINT first interest")?;
            let settlement = date_arg(&args[2], ctx, "ACCRINT settlement")?;
            let rate = number_or(&args[3], ctx, 0.0)?;
            let par = number_or(&args[4], ctx, 0.0)?;
            let frequency = number_or(&args[5], ctx, 0.0)?.trunc();
            let basis = basis_arg(args, 6, ctx, name)?;
            if issue >= settlement {
                return Err(EvalError::new(
                    "#NUM!: ACCRINT issue date must be before settlement",
                ));
            }
            if rate <= 0.0 || par <= 0.0 {
                return Err(EvalError::new(
                    "#NUM!: ACCRINT rate and par value must be positive",
                ));
            }
            let frequency = match frequency {
                1.0 => 1,
                2.0 => 2,
                4.0 => 4,
                _ => return Err(EvalError::new("#NUM!: ACCRINT frequency must be 1, 2 or 4")),
            };
            Value::Number(day_count::accrued_interest(
                issue,
                first_interest,
                settlement,
                rate,
                par,
                frequency,
                basis,
            )?)
        },

        _ => return Ok(None),
    };

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::super::tests::eval;
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_pmt() {
        let ctx = EvalContext::new();
        // PMT for $100,000 loan at 5% for 30 years
        let pmt = eval("PMT(0.05/12, 360, 100000)", &ctx).unwrap();
        assert!(matches!(pmt, Value::Number(n) if (n + 536.82).abs() < 0.01));
    }

    #[test]
    fn test_sln() {
        let ctx = EvalContext::new();
        // Straight-line depreciation: cost=10000, salvage=1000, life=5
        assert_eq!(
            eval("SLN(10000, 1000, 5)", &ctx).unwrap(),
            Value::Number(1800.0)
        );
    }

    #[test]
    fn test_irr() {
        let ctx = EvalContext::new();
        // IRR for cash flows: -100, 30, 35, 40, 45
        let irr = eval("IRR(-100, 30, 35, 40, 45)", &ctx).unwrap();
        assert!(matches!(irr, Value::Number(n) if (n - 0.178).abs() < 0.01));
    }

    fn eval_number(formula: &str, ctx: &EvalContext) -> f64 {
        match eval(formula, ctx).unwrap() {
            Value::Number(n) => n,
            other => panic!("{formula} returned {other:?}"),
        }
    }

    #[test]
    fn test_irr_matches_excel() {
        let ctx = EvalContext::new();
        // Excel's IRR help example gives 8.7% and -2.1% for these flows; the
        // exact digits come from an independent bisection
        let irr = eval_number("IRR(-70000, 12000, 15000, 18000, 21000, 26000)", &ctx);
        assert!((irr - 0.086_630_948_036_531_6).abs() < 1e-9, "{irr}");
        let irr = eval_number("IRR(-70000, 12000, 15000, 18000, 21000)", &ctx);
        assert!((irr + 0.021_244_848_273_411).abs() < 1e-9, "{irr}");
    }

    #[test]
    fn test_irr_without_sign_change_is_error() {
        let ctx = EvalContext::new();
        for formula in ["IRR(100, 200, 300)", "IRR(-100, -200)", "IRR(0, 0)"] {
            let err = eval(formula, &ctx).unwrap_err();
            assert!(
                err.message.contains("positive and one negative"),
                "{formula}: {}",
                err.message
            );
        }
    }

    #[test]
    fn test_irr_hard_start_still_converges() {
        let ctx = EvalContext::new();
        // Newton from 10% overshoots below -100% here; the bracket finds the root
        let irr = eval_number("IRR(-1000, 1, 1, 1, 1)", &ctx);
        let npv: f64 = [-1000.0, 1.0, 1.0, 1.0, 1.0]
            .iter()
            .enumerate()
            .map(|(i, cf)| cf / (1.0 + irr).powf(precision::count(i)))
            .sum();
        assert!(irr > -1.0 && npv.abs() < 1e-5, "irr {irr}, npv {npv}");
    }

    #[test]
    fn test_irr_guess_argument_is_not_a_cash_flow() {
        let mut ctx = EvalContext::new();
        let flows = [-100.0, 30.0, 35.0, 40.0, 45.0];
        let mut table = HashMap::new();
        table.insert(
            "cf".to_string(),
            flows.iter().copied().map(Value::Number).collect::<Vec<_>>(),
        );
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );
        let without = eval_number("IRR(t.cf)", &ctx);
        let with = eval_number("IRR(t.cf, 0.2)", &ctx);
        assert!((without - with).abs() < 1e-9, "{without} vs {with}");
        assert!((without - 0.170_936_863_395).abs() < 1e-9, "{without}");
    }

    #[test]
    fn test_xirr_without_sign_change_is_error() {
        let ctx = EvalContext::new();
        let err = eval("XIRR(100, 45000)", &ctx).unwrap_err();
        assert!(
            err.message.contains("positive and one negative"),
            "{}",
            err.message
        );
    }

    #[test]
    fn test_rate_matches_excel_and_reports_no_solution() {
        let ctx = EvalContext::new();
        // Excel's RATE help example gives 0.77% a month; exact digits from bisection
        let rate = eval_number("RATE(48, -200, 8000)", &ctx);
        assert!((rate - 0.007_701_472_488_23).abs() < 1e-9, "{rate}");
        // Payments at the start of each period (type 1)
        let due = eval_number("RATE(48, -200, 8000, 0, 1)", &ctx);
        assert!((due - 0.008_052_981_923_906).abs() < 1e-9, "{due}");
        // Receiving money every period with nothing paid: no rate exists
        assert!(eval("RATE(10, 100, 1000)", &ctx).is_err());
    }

    #[test]
    fn test_rate_rejects_column_for_type_and_guess() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "c".to_string(),
            vec![Value::Number(0.0), Value::Number(1.0)],
        );
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );
        for formula in [
            "RATE(48, -200, 8000, 0, t.c)",
            "RATE(48, -200, 8000, 0, 0, t.c)",
        ] {
            assert!(eval(formula, &ctx).is_err(), "{formula}");
        }
    }

    #[test]
    fn test_npv() {
        let ctx = EvalContext::new();
        // NPV at 10% for cash flows 100, 100, 100
        let npv = eval("NPV(0.10, 100, 100, 100)", &ctx).unwrap();
        assert!(matches!(npv, Value::Number(n) if (n - 248.69).abs() < 0.01));
    }

    // === EDGE CASES FOR 100% COVERAGE ===

    #[test]
    fn test_sln_zero_life() {
        let ctx = EvalContext::new();
        let result = eval("SLN(10000, 1000, 0)", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("zero"));
    }

    #[test]
    fn test_db_zero_life() {
        let ctx = EvalContext::new();
        // DB with life = 0 is #NUM!, as LibreOffice gives (it was 0)
        let error = eval("DB(10000, 1000, 0, 1)", &ctx).unwrap_err().to_string();
        assert!(error.contains("#NUM!"), "{error}");
    }

    #[test]
    fn test_db_zero_cost() {
        let ctx = EvalContext::new();
        // DB with cost = 0 is #NUM!, as LibreOffice gives (it was 0)
        let error = eval("DB(0, 0, 5, 1)", &ctx).unwrap_err().to_string();
        assert!(error.contains("#NUM!"), "{error}");
    }

    #[test]
    fn test_db_with_month() {
        let ctx = EvalContext::new();
        // DB with custom month parameter
        let result = eval("DB(10000, 1000, 5, 1, 6)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.0));
    }

    #[test]
    fn test_db_last_period() {
        let ctx = EvalContext::new();
        // DB last period (life + 1)
        let result = eval("DB(10000, 1000, 5, 6, 6)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n >= 0.0));
    }

    #[test]
    fn test_ddb_zero_life() {
        let ctx = EvalContext::new();
        let result = eval("DDB(10000, 1000, 0, 1)", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("zero"));
    }

    #[test]
    fn test_ddb_with_factor() {
        let ctx = EvalContext::new();
        // DDB with custom factor
        let result = eval("DDB(10000, 1000, 5, 1, 1.5)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.0));
    }

    #[test]
    fn test_ddb_below_salvage() {
        let ctx = EvalContext::new();
        // DDB where depreciation would go below salvage
        // High salvage relative to cost
        let result = eval("DDB(10000, 9000, 5, 5)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n >= 0.0));
    }

    #[test]
    fn test_mirr_positive_flows_only() {
        let ctx = EvalContext::new();
        // MIRR requires both positive and negative flows
        let result = eval("MIRR({100, 200, 300}, 0.10, 0.12)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_nper_zero_rate() {
        let ctx = EvalContext::new();
        // NPER with rate = 0
        let result = eval("NPER(0, -100, 1000)", &ctx).unwrap();
        assert_eq!(result, Value::Number(10.0)); // 1000 / 100 = 10
    }

    #[test]
    fn test_nper_with_fv() {
        let ctx = EvalContext::new();
        // NPER with future value
        let result = eval("NPER(0.05, -100, 1000, 500)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.0));
    }

    #[test]
    fn test_rate_function() {
        let ctx = EvalContext::new();
        // RATE: 12 periods, -100 payment, 1000 PV
        let result = eval("RATE(12, -100, 1000)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n.abs() < 0.1));
    }

    #[test]
    fn test_rate_with_fv() {
        let ctx = EvalContext::new();
        // RATE with future value
        let result = eval("RATE(12, -100, 1000, 100)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(_)));
    }

    #[test]
    fn test_rate_with_guess() {
        let ctx = EvalContext::new();
        // RATE with guess (6th param, skip 5th for pmt_type)
        let result = eval("RATE(12, -100, 1000, 0, 0, 0.05)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(_)));
    }

    #[test]
    fn test_fv_function() {
        let ctx = EvalContext::new();
        // FV: 5% rate, 12 periods, -100 payment
        let result = eval("FV(0.05, 12, -100)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.0));
    }

    #[test]
    fn test_fv_with_pv() {
        let ctx = EvalContext::new();
        // FV with present value
        let result = eval("FV(0.05, 12, -100, -1000)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(_)));
    }

    #[test]
    fn test_pv_function() {
        let ctx = EvalContext::new();
        // PV: 5% rate, 12 periods, -100 payment
        let result = eval("PV(0.05, 12, -100)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.0));
    }

    #[test]
    fn test_ppmt() {
        let ctx = EvalContext::new();
        // PPMT: Principal payment for period 1 of 60-month loan at 5% for $10,000
        let result = eval("PPMT(0.05/12, 1, 60, 10000)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n < 0.0));
    }

    #[test]
    fn test_ppmt_zero_rate() {
        let ctx = EvalContext::new();
        // PPMT with rate=0: simple division
        let result = eval("PPMT(0, 1, 12, 1200)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if (n + 100.0).abs() < 0.01));
    }

    #[test]
    fn test_ipmt() {
        let ctx = EvalContext::new();
        // IPMT: Interest payment for period 1 of 60-month loan at 5% for $10,000
        let result = eval("IPMT(0.05/12, 1, 60, 10000)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n < 0.0));
    }

    #[test]
    fn test_ipmt_zero_rate() {
        let ctx = EvalContext::new();
        // IPMT with rate=0: no interest
        let result = eval("IPMT(0, 1, 12, 1200)", &ctx).unwrap();
        assert_eq!(result, Value::Number(0.0));
    }

    #[test]
    fn test_effect() {
        let ctx = EvalContext::new();
        // EFFECT: 6% nominal rate compounded monthly
        let result = eval("EFFECT(0.06, 12)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.06 && n < 0.07));
    }

    #[test]
    fn test_effect_error_negative_rate() {
        let ctx = EvalContext::new();
        let result = eval("EFFECT(-0.05, 12)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_effect_error_invalid_periods() {
        let ctx = EvalContext::new();
        let result = eval("EFFECT(0.05, 0)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_nominal() {
        let ctx = EvalContext::new();
        // NOMINAL: 6.17% effective rate compounded monthly
        let result = eval("NOMINAL(0.0617, 12)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.05 && n < 0.07));
    }

    #[test]
    fn test_nominal_error_negative_rate() {
        let ctx = EvalContext::new();
        let result = eval("NOMINAL(-0.05, 12)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_nominal_error_invalid_periods() {
        let ctx = EvalContext::new();
        let result = eval("NOMINAL(0.05, 0)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_pricedisc() {
        let ctx = EvalContext::new();
        // PRICEDISC: $100 face value, 5% discount, 180 days
        // Settlement=0, Maturity=180, Discount=0.05, Redemption=100
        let result = eval("PRICEDISC(0, 180, 0.05, 100)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 95.0 && n < 100.0));
    }

    #[test]
    fn test_pricedisc_error_invalid_dates() {
        let ctx = EvalContext::new();
        // Settlement after maturity
        let result = eval("PRICEDISC(180, 0, 0.05, 100)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_yielddisc() {
        let ctx = EvalContext::new();
        // YIELDDISC: $97.50 price for $100 redemption, 180 days
        let result = eval("YIELDDISC(0, 180, 97.50, 100)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.0));
    }

    #[test]
    fn test_yielddisc_error_invalid_dates() {
        let ctx = EvalContext::new();
        let result = eval("YIELDDISC(180, 0, 97.50, 100)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_yielddisc_error_invalid_price() {
        let ctx = EvalContext::new();
        let result = eval("YIELDDISC(0, 180, 0, 100)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_accrint() {
        let ctx = EvalContext::new();
        // ACCRINT: $1000 par, 6% rate, 180 days, annual payment
        // Issue=0, FirstInterest=365, Settlement=180, Rate=0.06, Par=1000, Frequency=1
        let result = eval("ACCRINT(0, 365, 180, 0.06, 1000, 1)", &ctx).unwrap();
        assert!(matches!(result, Value::Number(n) if n > 0.0));
    }

    #[test]
    fn test_accrint_error_invalid_dates() {
        let ctx = EvalContext::new();
        // Issue after settlement
        let result = eval("ACCRINT(180, 365, 0, 0.06, 1000, 1)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_accrint_error_negative_rate() {
        let ctx = EvalContext::new();
        let result = eval("ACCRINT(0, 365, 180, -0.06, 1000, 1)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_accrint_error_invalid_par() {
        let ctx = EvalContext::new();
        let result = eval("ACCRINT(0, 365, 180, 0.06, 0, 1)", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_accrint_error_invalid_frequency() {
        let ctx = EvalContext::new();
        // Frequency must be 1, 2, or 4
        let result = eval("ACCRINT(0, 365, 180, 0.06, 1000, 3)", &ctx);
        assert!(result.is_err());
    }

    /// Payments at the start of each period (Excel `type` = 1). Reference values from
    /// numpy-financial 1.1.0, which follows Excel's definitions.
    fn number(formula: &str) -> f64 {
        match eval(formula, &EvalContext::new()).unwrap() {
            Value::Number(n) => n,
            other => panic!("{formula} gave {other:?}"),
        }
    }

    fn assert_close(formula: &str, expected: f64) {
        let got = number(formula);
        assert!(
            (got - expected).abs() <= 1e-9 * expected.abs().max(1.0),
            "{formula}: got {got}, expected {expected}"
        );
    }

    #[test]
    fn test_payment_timing_type_1() {
        assert_close("FV(0.04/12, 120, -200, -5000, 1)", 37_002.290_893_670_69);
        assert_close("PV(0.05/12, 60, -500, 1000, 1)", 25_826.545_076_488_12);
        assert_close("NPER(0.06/12, -400, 15000, 0, 1)", 41.401_543_679_333_26);
        assert_close("PMT(0.05/12, 60, 25000, 0, 1)", -469.823_244_249_236_2);
        assert_close("IPMT(0.01, 2, 24, 10000, 0, 1)", -95.339_260_175_914_39);
        assert_close("IPMT(0.01, 1, 24, 10000, 0, 1)", 0.0);
        assert_close("PPMT(0.01, 2, 24, 10000, 0, 1)", -370.734_722_232_646_7);
    }

    #[test]
    fn test_payment_timing_type_0_is_the_default() {
        for (with_type, without) in [
            ("FV(0.05, 12, -100, -1000, 0)", "FV(0.05, 12, -100, -1000)"),
            ("PV(0.05, 12, -100, 500, 0)", "PV(0.05, 12, -100, 500)"),
            ("NPER(0.05, -100, 500, 0, 0)", "NPER(0.05, -100, 500)"),
            ("IPMT(0.01, 3, 24, 10000, 0, 0)", "IPMT(0.01, 3, 24, 10000)"),
        ] {
            assert_close(with_type, number(without));
        }
    }

    #[test]
    fn test_nper_without_a_solution_is_num_error() {
        // A payment of 600 never covers 8% interest on 25,000
        let error = eval("NPER(0.08, -600, 25000)", &EvalContext::new()).unwrap_err();
        assert!(error.to_string().contains("#NUM!"), "{error}");
    }

    /// Security functions with real dates; values checked in Excel on 2026-11-06
    #[test]
    fn test_security_functions_match_excel() {
        assert_close("PRICEDISC(36341, 39903, 0.05, 100, 0)", 51.25);
        assert_close(
            "PRICEDISC(43541, 43708, 0.05, 100, 4)",
            97.736_111_111_111_11,
        );
        assert_close(
            "PRICEDISC(38777, 49064, 0.05, 100, 1)",
            -40.824_679_003_021_15,
        );
        assert_close(
            "YIELDDISC(36885, 36950, 95, 100, 1)",
            0.295_546_558_704_453_5,
        );
        assert_close(
            "ACCRINT(34938, 35115, 39325, 0.06, 1000, 1, 0)",
            720.666_666_666_666_6,
        );
        assert_close(
            "ACCRINT(44296, 44548, 48337, 0.06, 1000, 2, 1)",
            663.832_642_767_068_9,
        );
        // Dates may also be given as text
        assert_close("PRICEDISC(\"2024-01-15\", \"2024-07-15\", 0.05, 100)", 97.5);
    }

    #[test]
    fn test_security_functions_reject_bad_arguments() {
        let ctx = EvalContext::new();
        for formula in [
            "PRICEDISC(45488, 45306, 0.05, 100)",
            "PRICEDISC(45306, 45488, 0, 100)",
            "PRICEDISC(45306, 45488, 0.05, 100, 5)",
            "YIELDDISC(45488, 45306, 97, 100)",
            "YIELDDISC(45306, 45488, 0, 100)",
            "ACCRINT(45488, 45672, 45306, 0.06, 1000, 1)",
            "ACCRINT(45306, 45672, 45488, 0, 1000, 1)",
            "ACCRINT(45306, 45672, 45488, 0.06, 1000, 3)",
            "ACCRINT(\"not a date\", 45672, 45488, 0.06, 1000, 1)",
        ] {
            let error = eval(formula, &ctx).unwrap_err();
            assert!(
                error.to_string().contains("#NUM!") || error.to_string().contains("date"),
                "{formula}: {error}"
            );
        }
    }

    /// The five answers Excel gave differently from Forge when 40 formulas were
    /// checked in Excel on 2026-11-14
    /// (`validation/gnumeric/tests/edge/edge_excel_checks.yaml`), and what
    /// follows from each rule.
    mod checked_in_excel {
        use crate::core::array_calculator::evaluator::tests as h;

        #[test]
        fn db_cuts_a_fraction_of_a_month() {
            h::is_exactly("DB(10000, 1000, 5, 1, 6.5)", 1845.0); // Excel
            h::is_exactly("DB(10000, 1000, 5, 1, 6.9)", 1845.0); // LibreOffice
        }
    }

    /// Fixes for what the generated cases found (ADR-062) that the engines cannot
    /// check: answers Forge gives on its own, with the reason, and the exact
    /// values where every engine loses digits.
    mod generated_cases {
        use crate::core::array_calculator::evaluator::tests as h;

        #[test]
        fn annuities_keep_their_digits() {
            // Exact to 50 digits for the double inputs (2026-11-14)
            h::exactly(
                "NPER(1e-09, 3.944973623587, 0, 2.5, 1)",
                -0.633_717_798_414_327_9,
            );
            h::exactly("NPER(0.1, -1e-12, -2.5)", -275.361_213_342_520_2);
            h::exactly(
                "NPER(0.1, 0.285, 1E15, -0.559709076024, 1)",
                -348.670_778_361_094_7,
            );
            h::exactly(
                "CUMPRINC(0.5, 61, 487, 3, 36, 0)",
                -0.019_286_216_489_415_402,
            );
            h::exactly(
                "CUMPRINC(0.5, 60, 0.663097621158, 43, 52, 1)",
                -0.016_949_572_163_691_61,
            );
            h::exactly("CUMPRINC(0.03, 24, 900, 5, 9, 1)", -151.665_053_343_195_1);
            h::exactly("CUMPRINC(0.05, 30, 1000, 1, 30, 1)", -1000.0);
            h::exactly("RATE(276, -100, 1E15)", -0.095_099_553_259_213_3);
        }
    }

    /// A life of 3E+9 years made `life + 1` overflow an i32 in DB's loop, a
    /// crash; its rate rounds to 0.000, so Gnumeric gives 0. A period no i32
    /// holds is #NUM!.
    #[test]
    fn db_counts_its_periods_past_any_i32_life() {
        use crate::core::array_calculator::evaluator::tests as h;
        h::is_exactly("DB(1000, 100, 3E+9, 2)", 0.0);
        h::is_exactly("DB(1000, 100, 5, 2)", 232.839);
        h::fails_with("DB(1000, 100, 1E+10, 1E+10)", "#NUM!");
    }

    /// Negative cash flows are discounted at the finance rate, positive ones
    /// grown at the reinvestment rate; Gnumeric's MIRR
    #[test]
    fn mirr_takes_each_side_at_its_rate() {
        use crate::core::array_calculator::evaluator::tests as h;
        assert!(h::near(
            h::number("MIRR({-100, 50, 60}, 0.1, 0.12)"),
            0.077_032_961_426_900_75
        ));
        assert!(h::near(
            h::number("MIRR({-100, -20, 50, 80}, 0.1, 0.12)"),
            0.047_923_099_669_167_3
        ));
    }
}
