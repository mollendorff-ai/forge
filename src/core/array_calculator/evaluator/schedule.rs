//! Growth, loan and depreciation schedules: RRI, CUMIPMT, CUMPRINC, SYD and VDB

// Periods, types and lives are whole numbers held in f64, so comparing and counting
// them is exact.
#![allow(clippy::while_float)]

use super::financial::{annuity_ipmt, annuity_pmt};
use super::{evaluate, number_or, require_args, require_args_range};
use super::{EvalContext, EvalError, Expr, Value};
use crate::core::precision::same;

/// The argument at `index` as a number
fn number(name: &str, args: &[Expr], index: usize, ctx: &EvalContext) -> Result<f64, EvalError> {
    evaluate(&args[index], ctx)?
        .as_cell_number()
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
    let (rate, nper, pv, start, end, kind) = cumulative_args(name, args, ctx)?;
    let mut total = 0.0;
    let mut per = start;
    while per <= end {
        total += part(rate, per, nper, pv, kind);
        per += 1.0;
    }
    Ok(total)
}

/// CUMPRINC in closed form. The principal of period k is the first full
/// period's times (1 + r)^(k - 1), and that first one is -pv * r / ((1 + r)^n - 1),
/// so the sum is a geometric series with no difference of large numbers (the
/// sum of PMT - IPMT lost digits at a rate of 50%: 7E-6 relative). With
/// payments at the start, period 1 is the whole payment and the series starts
/// at period 2.
fn cumulative_principal(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<f64, EvalError> {
    let (rate, nper, pv, start, end, kind) = cumulative_args(name, args, ctx)?;
    let ln = rate.ln_1p();
    let grow = |k: f64| (k * ln).exp();
    let grown = |k: f64| (k * ln).exp_m1();
    let per_unit = -pv / grown(nper);
    // Sum of (1 + r)^j for j from a to b, times r: (1 + r)^a * ((1 + r)^(b - a + 1) - 1)
    let series = |a: f64, b: f64| {
        if b < a {
            0.0
        } else {
            grow(a) * grown(b - a + 1.0)
        }
    };
    Ok(if kind == 0.0 {
        per_unit * series(start - 1.0, end - 1.0)
    } else {
        let first = if start <= 1.0 {
            annuity_pmt(rate, nper, pv, 0.0, 1.0)
        } else {
            0.0
        };
        per_unit.mul_add(series(start.max(2.0) - 2.0, end - 2.0), first)
    })
}

/// The checked arguments of CUMIPMT and CUMPRINC
fn cumulative_args(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<(f64, f64, f64, f64, f64, f64), EvalError> {
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
    if !(same(kind, 0.0) || same(kind, 1.0)) {
        return Err(num_error(name, "the type must be 0 or 1"));
    }
    Ok((rate, nper.trunc(), pv, start, end, kind))
}

/// Depreciation of one whole period by the declining balance, as `DDB` gives it
pub(super) fn declining(cost: f64, salvage: f64, life: f64, period: f64, factor: f64) -> f64 {
    let mut rate = factor / life;
    let before = if rate >= 1.0 {
        rate = 1.0;
        if same(period, 1.0) {
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
        if same(period, whole) {
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
    // A salvage above the cost is #NUM! too, as both engines give
    if cost < 0.0
        || salvage < 0.0
        || salvage > cost
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
            if same(period, first + 1.0) {
                term *= end.min(first + 1.0) - start;
            } else if same(period, last) {
                term *= end + 1.0 - last;
            }
            total += term;
            period += 1.0;
        }
        return Ok(total);
    }
    let mut remaining_life = life;
    if start.fract() != 0.0 && factor > 1.0 && start >= life / 2.0 {
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
            // A negative ratio has a root only when 1/nper is a whole number,
            // as in POWER (RRI(1, 100, -50) is -1.5, as both engines give)
            let grown = (fv / pv).powf(1.0 / nper);
            if grown.is_nan() {
                return Err(num_error(
                    name,
                    "the present and future values must have the same sign",
                ));
            }
            Value::Number(grown - 1.0)
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

        "CUMPRINC" => Value::Number(cumulative_principal(name, args, ctx)?),
        "VDB" => Value::Number(vdb(name, args, ctx)?),

        _ => return Ok(None),
    };
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    /// Tests of the functions added in 10.1: schedules, conveniences, percentiles.
    /// Their expected values were computed outside Forge (R, Python, by hand); the
    /// Gnumeric suite (`statistics_finance_batch.yaml`) checks the same functions
    /// against two spreadsheet engines.
    mod batch_10_1 {
        use crate::core::array_calculator::evaluator::tests as h;
        use crate::core::array_calculator::evaluator::EvalContext;

        #[test]
        fn rri_and_syd() {
            let ctx = EvalContext::new();
            // (2500 / 1000) ^ (1 / 10) - 1, and the sum of the years' digits for year 1 of 5
            assert!(h::near(
                h::number_in("RRI(10, 1000, 2500)", &ctx),
                0.095_958_226_385_217_23
            ));
            assert!(h::near(h::number_in("RRI(12, 5000, 5000)", &ctx), 0.0));
            assert!(h::near(
                h::number_in("SYD(10000, 1000, 5, 1)", &ctx),
                3_000.0
            ));
            assert!(h::near(h::number_in("SYD(10000, 1000, 5, 5)", &ctx), 600.0));
            assert!(h::error_in("RRI(0, 1000, 2500)", &ctx).contains("#NUM!"));
            assert!(h::error_in("RRI(5, 0, 2500)", &ctx).contains("#DIV/0!"));
            assert!(h::error_in("RRI(5, 100, -50)", &ctx).contains("#NUM!"));
            assert!(h::error_in("SYD(10000, 1000, 5, 6)", &ctx).contains("#NUM!"));
        }

        #[test]
        fn cumulative_interest_and_principal_of_a_loan() {
            let ctx = EvalContext::new();
            // Microsoft's example: 9% a year, 30 years, 125,000; the second year's
            // interest is -11,135.23 and the principal -934.11 (a loop in R agrees)
            let interest = h::number_in("CUMIPMT(0.09/12, 360, 125000, 13, 24, 0)", &ctx);
            let principal = h::number_in("CUMPRINC(0.09/12, 360, 125000, 13, 24, 0)", &ctx);
            assert!((interest + 11_135.23).abs() < 0.005, "{interest}");
            assert!((principal + 934.107).abs() < 0.005, "{principal}");
            // Over the whole loan the principal paid is the loan, and interest plus
            // principal is the payments
            let all_principal = h::number_in("CUMPRINC(0.09/12, 360, 125000, 1, 360, 0)", &ctx);
            assert!(h::near(all_principal, -125_000.0));
            let payments = h::number_in("PMT(0.09/12, 360, 125000)", &ctx) * 360.0;
            let all_interest = h::number_in("CUMIPMT(0.09/12, 360, 125000, 1, 360, 0)", &ctx);
            assert!(h::near(all_interest + all_principal, payments));
            // A loan paid at the start of each period pays no interest in period 1
            assert!(h::near(
                h::number_in("CUMIPMT(0.05, 10, 20000, 1, 1, 1)", &ctx),
                0.0
            ));
            for bad in [
                "CUMIPMT(0, 360, 125000, 1, 12, 0)",
                "CUMIPMT(0.05, 360, 0, 1, 12, 0)",
                "CUMIPMT(0.05, 360, 125000, 0, 12, 0)",
                "CUMIPMT(0.05, 360, 125000, 12, 1, 0)",
                "CUMPRINC(0.05, 360, 125000, 1, 361, 0)",
                "CUMPRINC(0.05, 360, 125000, 1, 12, 2)",
            ] {
                assert!(h::error_in(bad, &ctx).contains("#NUM!"), "{bad}");
            }
        }

        #[test]
        fn vdb_switches_to_the_straight_line_and_can_be_told_not_to() {
            let ctx = EvalContext::new();
            // Microsoft's examples with the answers they give: the first day, the
            // first month, the first year, and a factor of 1.5
            assert!((h::number_in("VDB(2400, 300, 3650, 0, 1)", &ctx) - 1.32).abs() < 0.005);
            assert!(h::near(
                h::number_in("VDB(2400, 300, 120, 0, 1)", &ctx),
                40.0
            ));
            assert!(h::near(
                h::number_in("VDB(2400, 300, 10, 0, 1)", &ctx),
                480.0
            ));
            assert!((h::number_in("VDB(2400, 300, 120, 6, 18, 1.5)", &ctx) - 311.81).abs() < 0.005);
            // Over the whole life the asset loses its cost less its salvage, switching or not
            assert!(h::near(
                h::number_in("VDB(2400, 300, 10, 0, 10)", &ctx),
                2_100.0
            ));
            // Without the switch the balance falls by a fixed share and stops at the salvage
            let no_switch = h::number_in("VDB(50000, 5000, 8, 0, 8, 2, TRUE)", &ctx);
            assert!(no_switch <= 45_000.0 + 1e-9);
            assert!(h::error_in("VDB(2400, 300, 10, 0, 11)", &ctx).contains("#NUM!"));
            assert!(h::error_in("VDB(2400, 300, 10, 5, 4)", &ctx).contains("#NUM!"));
            assert!(h::error_in("VDB(-1, 300, 10, 0, 1)", &ctx).contains("#NUM!"));
        }
    }

    /// An error in any argument is passed on, and text where a number is
    /// needed is refused with the argument's place
    #[test]
    fn each_argument_is_checked() {
        use crate::core::array_calculator::evaluator::tests::each_argument;
        for (name, values) in [
            ("CUMIPMT", vec!["0.05", "10", "1000", "1", "3", "0"]),
            ("CUMPRINC", vec!["0.05", "10", "1000", "1", "3", "0"]),
            ("VDB", vec!["2400", "300", "10", "1", "3"]),
            ("RRI", vec!["10", "100", "200"]),
            ("SYD", vec!["2400", "300", "10", "2"]),
        ] {
            let messages: Vec<String> = (1..=values.len())
                .map(|i| format!("{name} needs a number as argument {i}"))
                .collect();
            let args: Vec<(&str, Option<&str>)> = values
                .iter()
                .zip(&messages)
                .map(|(v, m)| (*v, Some(m.as_str())))
                .collect();
            each_argument(name, &args);
        }
        // VDB's factor and no-switch flag, which take what they are given
        each_argument(
            "VDB",
            &[
                ("2400", None),
                ("300", None),
                ("10", None),
                ("1", None),
                ("3", None),
                ("1.5", None),
                ("1", None),
            ],
        );
    }

    /// A factor of at least the life takes everything down to the salvage in
    /// the first period and nothing after (`LibreOffice` Calc 24.2: 900 and 0),
    /// and a number for VDB's no-switch flag is TRUE unless it is 0
    #[test]
    fn declining_balance_edges() {
        use crate::core::array_calculator::evaluator::tests as h;
        h::is_exactly("DDB(1000, 100, 2, 1, 3)", 900.0);
        h::is_exactly("DDB(1000, 100, 2, 2, 3)", 0.0);
        assert_eq!(
            h::number("VDB(2400, 300, 10, 1, 3, 2, 1)").to_bits(),
            h::number("VDB(2400, 300, 10, 1, 3, 2, TRUE)").to_bits()
        );
    }

    /// The principal of the first payment, made at the start, is the whole
    /// payment (`LibreOffice`: -123.337690443292 for both)
    #[test]
    fn principal_of_the_first_payment_at_the_start() {
        use crate::core::array_calculator::evaluator::tests as h;
        assert_eq!(
            h::number("CUMPRINC(0.05, 10, 1000, 1, 1, 1)").to_bits(),
            h::number("PMT(0.05, 10, 1000, 0, 1)").to_bits()
        );
        assert!(h::near(
            h::number("CUMPRINC(0.05, 10, 1000, 1, 1, 1)"),
            -123.337_690_443_292
        ));
    }

    /// A start that is no whole period, in the second half of the life with
    /// a factor above 1, moves back to half the life, and the end with it, as
    /// `LibreOffice`'s `ScVDB` does. Worked by hand through its steps: the
    /// first 5 periods of 2,400 over 10 (a straight line over 11) lose 480,
    /// 384, 307.2, 245.76 and 196.608; then 3.5 periods of the 786.432 left
    /// (a straight line over 5) lose 157.2864, 125.82912, 100.663296 and half
    /// of 80.5306368.
    #[test]
    fn vdb_from_a_part_period_in_the_second_half_of_the_life() {
        use crate::core::array_calculator::evaluator::tests::{near, number};
        let got = number("VDB(2400, 300, 10, 6.5, 10, 2)");
        assert!(near(got, 424.044_134_4), "{got}");
    }
}
