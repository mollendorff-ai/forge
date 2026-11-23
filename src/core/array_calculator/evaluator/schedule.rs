//! Growth, loan and depreciation schedules: RRI, CUMIPMT, CUMPRINC, SYD and VDB
//!
//! CUMIPMT, CUMPRINC and VDB add up a run of periods. Each sum is a closed form,
//! so a run of 10^17 periods costs what a run of one does, and no sum is a
//! difference of large numbers: a balance is e^(j ln(1 - rate)), and what it
//! loses between two periods is that times -expm1.

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

/// (x - 1 + e^-x) / x for x > 0: how far the mean of e^-t over 0..x falls
/// short of 1. Near 0 the two sides of the difference agree in most of their
/// digits, so there it is its Taylor series, x/2! - x^2/3! + x^3/4! - ...,
/// whose 18 terms at x < 0.5 leave less than 1E-25 out.
fn shortfall(x: f64) -> f64 {
    if x < 0.5 {
        let mut sum = 0.0;
        let mut term = x / 2.0;
        for k in 1..=18_u8 {
            sum += term;
            term *= -x / f64::from(k + 2);
        }
        sum
    } else {
        (x + (-x).exp_m1()) / x
    }
}

/// The level payment, paid at the end of each period, of a loan of `pv`
/// over `nper` periods at `rate` (Excel's `PMT`), written so that a large
/// `nper` neither overflows nor divides infinity by infinity
fn level_payment(rate: f64, nper: f64, pv: f64) -> f64 {
    pv * rate / (-nper * rate.ln_1p()).exp_m1()
}

/// How much of the payments `start..=end` of a loan over `nper` periods at
/// `rate` (payments at the end of each period) is interest and how much is
/// principal, each as a multiple of one payment.
///
/// After j periods the balance is pv * (1 - (1 + r)^(j - n)) / (1 - (1 + r)^-n),
/// so the principal of period k is the payment times (1 + r)^(k - 1 - n), and
/// its interest the payment times 1 - (1 + r)^(k - 1 - n). With h = 1 / (1 + r),
/// c = n - end + 1 periods after the last and m = end - start + 1 of them, the
/// principal is h^c (1 - h^m) / (1 - h), and the interest m (1 - h^c) plus h^c
/// times the sum of 1 - h^i for i below m, which is m (s(mu) - s(u)) / (1 - s(u))
/// with u = ln(1 + r) and s the shortfall: no term is a difference of large
/// numbers.
fn shares(rate: f64, nper: f64, start: f64, end: f64) -> (f64, f64) {
    let u = rate.ln_1p();
    let m = end - start + 1.0;
    let after = nper - end + 1.0;
    let kept = (-after * u).exp();
    let principal = kept * (-m * u).exp_m1() / (-u).exp_m1();
    let short = shortfall(u);
    let below = m * (shortfall(m * u) - short) / (1.0 - short);
    let interest = (-m).mul_add((-after * u).exp_m1(), kept * below);
    (interest, principal)
}

/// `CUMIPMT` and `CUMPRINC`: the interest or the principal of the payments
/// from period `start` to `end`. With payments at the start of each period
/// (`kind` 1) every amount is one period earlier, so divided by 1 + rate,
/// and the first payment is all principal.
fn cumulative(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
    interest: bool,
) -> Result<f64, EvalError> {
    let (rate, nper, pv, start, end, kind) = cumulative_args(name, args, ctx)?;
    Ok(cumulative_sum(rate, nper, pv, start, end, kind, interest))
}

/// The sum `cumulative` gives, the arguments checked
fn cumulative_sum(
    rate: f64,
    nper: f64,
    pv: f64,
    start: f64,
    end: f64,
    kind: f64,
    interest: bool,
) -> f64 {
    let payment = level_payment(rate, nper, pv);
    let part = |from: f64| {
        let (interest_share, principal_share) = shares(rate, nper, from, end);
        payment
            * if interest {
                interest_share
            } else {
                principal_share
            }
    };
    if kind == 0.0 {
        return part(start);
    }
    let first = if interest || start > 1.0 {
        0.0
    } else {
        payment
    };
    let rest = if end < 2.0 { 0.0 } else { part(start.max(2.0)) };
    (first + rest) / (1.0 + rate)
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

/// An asset written down by a declining balance: each period it loses
/// `rate` of what it is worth, down to its salvage value
struct Declining {
    cost: f64,
    salvage: f64,
    rate: f64,
    /// ln(1 - rate), or `None` when the rate is 1 and all goes at once
    ln_kept: Option<f64>,
}

impl Declining {
    fn new(cost: f64, salvage: f64, life: f64, factor: f64) -> Self {
        let rate = (factor / life).min(1.0);
        let ln_kept = (rate < 1.0).then(|| (-rate).ln_1p());
        Self {
            cost,
            salvage,
            rate,
            ln_kept,
        }
    }

    /// The worth after `periods` periods, as if it could fall below the salvage
    fn worth(&self, periods: f64) -> f64 {
        if periods == 0.0 {
            return self.cost;
        }
        self.ln_kept
            .map_or(0.0, |ln_kept| self.cost * (periods * ln_kept).exp())
    }

    /// The depreciation from period `from` to period `to` (whole periods,
    /// `from` <= `to`), which stops at the salvage, as `DDB` gives each period
    fn lost(&self, from: f64, to: f64) -> f64 {
        let before = self.worth(from);
        if to <= from || before <= self.salvage {
            0.0
        } else if self.worth(to) >= self.salvage {
            let share = self
                .ln_kept
                .map_or(1.0, |ln_kept| -((to - from) * ln_kept).exp_m1());
            before * share
        } else {
            before - self.salvage
        }
    }
}

/// Depreciation over the first `periods` periods of an asset of this `cost`,
/// by the declining balance until the straight line over what is left is
/// larger, and then by the straight line. `remaining_life` is the life the
/// straight line counts from the start. The last period counts only as far
/// as `periods` reaches into it.
///
/// Period p switches when what is left, w(p - 1) - salvage, spread over the
/// k = `remaining_life` - p + 1 periods to go, is more than the period's fall,
/// w(p - 1) * rate: when w(p - 1) * (1 - rate * k) > salvage (until the worth
/// reaches the salvage, after which nothing falls). That side rises with p to
/// a peak at `remaining_life` + 1 - 1/rate - 1/ln(1 - rate) and falls after
/// it, so the first period that switches is found by halving the periods up
/// to the peak.
fn switching(
    cost: f64,
    salvage: f64,
    life: f64,
    remaining_life: f64,
    periods: f64,
    factor: f64,
) -> f64 {
    let whole = periods.ceil();
    if whole < 1.0 {
        return 0.0;
    }
    let part = periods + 1.0 - whole;
    let asset = Declining::new(cost, salvage, life, factor);
    let rate = asset.rate;
    let room = |p: f64| asset.worth(p - 1.0) * rate.mul_add(-(remaining_life - p + 1.0), 1.0);
    let within = |p: f64| p.clamp(1.0, whole);
    // 1/rate + 1/ln(1 - rate) is -1/2 - rate/12 - rate^2/24 - ..., whose
    // two terms agree in most of their digits when the rate is small
    let peak = match asset.ln_kept {
        None => remaining_life,
        Some(_) if rate == 0.0 => 1.0,
        Some(_) if rate < 1e-3 => remaining_life + 1.5 + rate / 12.0 + rate * rate / 24.0,
        Some(ln_kept) => remaining_life + 1.0 - (1.0 / rate + 1.0 / ln_kept),
    };
    let near = within(peak.round());
    let top = [within(near - 1.0), near, within(near + 1.0)]
        .into_iter()
        .fold(near, |best, p| if room(p) > room(best) { p } else { best });
    let mut switch = None;
    if room(top) > salvage {
        // The first period that switches: `low` does not, `high` does
        let (mut low, mut high) = (1.0, top);
        if room(low) > salvage {
            high = low;
        }
        // Beyond 2^53 halving can stop moving, so the steps are counted
        for _ in 0..1100 {
            if high - low <= 1.0 {
                break;
            }
            let middle = (f64::midpoint(low, high)).floor();
            if room(middle) > salvage {
                high = middle;
            } else {
                low = middle;
            }
        }
        if asset.worth(high) >= salvage {
            switch = Some(high);
        }
    }
    if let Some(first) = switch {
        let level = (asset.worth(first - 1.0) - salvage) / (remaining_life - first + 1.0);
        return level.mul_add(whole - first + part, asset.lost(0.0, first - 1.0));
    }
    let spent = asset.lost(0.0, whole - 1.0);
    let fall = asset.lost(whole - 1.0, whole);
    // Within the last period of the life only part of a period is left to
    // spread over, which can make the straight line the larger even there
    let to_go = remaining_life - whole + 1.0;
    let left = cost - salvage - spent;
    let last = if to_go < 1.0 && left / to_go > fall {
        left / to_go
    } else {
        fall
    };
    last.mul_add(part, spent)
}

/// `VDB`: depreciation
/// between two points of an asset's life
fn vdb(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<f64, EvalError> {
    require_args_range(name, args, 5, 7)?;
    let cost = number(name, args, 0, ctx)?;
    let salvage = number(name, args, 1, ctx)?;
    let life = number(name, args, 2, ctx)?;
    let start = number(name, args, 3, ctx)?;
    let end = number(name, args, 4, ctx)?;
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
    Ok(depreciation(
        cost, salvage, life, start, end, factor, no_switch,
    ))
}

/// VDB's depreciation of an asset from `start` to `end`, the arguments checked
fn depreciation(
    cost: f64,
    salvage: f64,
    life: f64,
    start: f64,
    end: f64,
    factor: f64,
    no_switch: bool,
) -> f64 {
    if no_switch {
        // The declining balance alone; the first and the last period count
        // only the part inside start..end
        let first = start.floor();
        let last = end.ceil();
        if last < first + 1.0 {
            return 0.0;
        }
        let asset = Declining::new(cost, salvage, life, factor);
        let mut total = asset.lost(first, first + 1.0) * (end.min(first + 1.0) - start);
        if last > first + 1.0 {
            total += asset
                .lost(last - 1.0, last)
                .mul_add(end + 1.0 - last, asset.lost(first + 1.0, last - 1.0));
        }
        return total;
    }
    let (mut start, mut end, mut remaining_life) = (start, end, life);
    if start.fract() != 0.0 && factor > 1.0 && start >= life / 2.0 {
        let part = start - life / 2.0;
        start = life / 2.0;
        end -= part;
        remaining_life += 1.0;
    }
    let spent = switching(cost, salvage, life, remaining_life, start, factor);
    switching(
        cost - spent,
        salvage,
        life,
        life - start,
        end - start,
        factor,
    )
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

        "CUMIPMT" => Value::Number(cumulative(name, args, ctx, true)?),
        "CUMPRINC" => Value::Number(cumulative(name, args, ctx, false)?),
        "VDB" => Value::Number(vdb(name, args, ctx)?),

        _ => return Ok(None),
    };
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    /// payment (`LibreOffice`: -123.337690443292 for both). PMT and CUMPRINC
    /// compute it in different ways, which agree to the last bit but one.
    #[test]
    fn principal_of_the_first_payment_at_the_start() {
        use crate::core::array_calculator::evaluator::tests as h;
        let principal = h::number("CUMPRINC(0.05, 10, 1000, 1, 1, 1)");
        let payment = h::number("PMT(0.05, 10, 1000, 0, 1)");
        assert!((principal - payment).abs() <= f64::EPSILON * payment.abs());
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

    /// `got` is `want` to within `relative` of it
    fn close(got: f64, want: f64, relative: f64) -> bool {
        (got - want).abs() <= relative * want.abs()
    }

    /// Runs of 10^17 periods are summed at once. Each of these ran for
    /// hours, counting one period at a time; Gnumeric 1.12.56 gives #NUM!,
    /// 0 (for a whole life with no salvage, which loses the whole cost) or
    /// #VALUE!, and `LibreOffice` 24.8 does not finish. The exact values:
    /// far from the end of a loan the balance is the loan, so the interest is
    /// rate * pv; a rate of 2E-17 over 5E+16 periods keeps e^-1 of the cost.
    #[test]
    fn a_run_of_10_to_the_17_periods_is_summed_at_once() {
        use crate::core::array_calculator::evaluator::tests as h;
        h::is_exactly("CUMIPMT(0.01, 1E+20, 1000, 1E+17, 1E+17, 0)", -10.0);
        h::is_exactly("CUMIPMT(0.01, 1E+20, 1000, 1E+17 - 64, 1E+17, 0)", -650.0);
        let principal = h::number("CUMPRINC(0.01, 1E+20, 1000, 1E+17, 1E+17, 0)");
        assert_eq!(principal.abs().to_bits(), 0.0_f64.to_bits());
        h::is_exactly("VDB(1000, 0, 1E+17, 0, 1E+17)", 1000.0);
        let kept_e = h::number("VDB(1000, 100, 1E+17, 0, 5E+16, 2, TRUE)");
        assert!(close(kept_e, 632.120_558_828_557_7, 1e-14), "{kept_e}");
        let first = h::number("VDB(2400, 300, 1E+17, 0, 1)");
        assert!(close(first, 4.8e-14, 1e-14), "{first}");
    }

    /// Late in a long loan the balance is a small difference of large
    /// numbers, which the sum of IPMT lost entirely: the last interest
    /// payment of 1,000 at 5% was out by a factor of millions. Gnumeric gives
    /// 0 and `LibreOffice` 858,993,459.2; the exact values (60 digits, mpmath)
    /// are below. A tiny rate is exact too, as `LibreOffice` gives it.
    #[test]
    fn late_periods_of_a_long_loan_and_tiny_rates_are_exact() {
        use crate::core::array_calculator::evaluator::tests as h;
        for (formula, exact) in [
            (
                "CUMIPMT(0.05, 1000, 100000, 1000, 1000, 0)",
                -238.095_238_095_238_1,
            ),
            (
                "CUMPRINC(0.05, 1000, 100000, 1000, 1000, 0)",
                -4_761.904_761_904_762,
            ),
            (
                "CUMIPMT(0.05, 1000, 100000, 990, 1000, 1)",
                -12_826.598_960_613_1,
            ),
            (
                "CUMIPMT(1E-9, 12, 100000, 2, 2, 1)",
                -9.166_666_662_083_333e-5,
            ),
            (
                "CUMIPMT(0.09/12, 360, 125000, 13, 24, 0)",
                -11_135.232_130_750_84,
            ),
        ] {
            let got = h::number(formula);
            assert!(close(got, exact, 1e-14), "{formula}: {got}, not {exact}");
        }
    }

    /// The closed forms of CUMIPMT and CUMPRINC are the sums of each
    /// period's interest and principal, payment * (1 - h^(n - k + 1)) and
    /// payment * h^(n - k + 1) with h = 1 / (1 + rate), added one by one
    #[test]
    fn cumulative_sums_are_the_periods_added_one_by_one() {
        let mut checked = 0;
        for rate in [1e-12, 1e-6, 1e-3, 0.0075, 0.05, 0.3, 2.0] {
            for nper in [1_u32, 2, 3, 12, 360] {
                let n = f64::from(nper);
                let payment = level_payment(rate, n, 1000.0);
                let after = |k: u32| -f64::from(nper - k + 1) * rate.ln_1p();
                for (start, end) in [(1, 1), (1, nper), (nper, nper), (nper.div_ceil(2), nper)] {
                    for due in [false, true] {
                        let kind = f64::from(u8::from(due));
                        let (mut interest, mut principal) = (0.0, 0.0);
                        for k in start..=end {
                            if due && k == 1 {
                                principal += payment / (1.0 + rate);
                                continue;
                            }
                            let shift = kind.mul_add(rate, 1.0);
                            interest += payment * -after(k).exp_m1() / shift;
                            principal += payment * after(k).exp() / shift;
                        }
                        let (from, to) = (f64::from(start), f64::from(end));
                        let sum = |part| cumulative_sum(rate, n, 1000.0, from, to, kind, part);
                        for (got, want) in [(sum(true), interest), (sum(false), principal)] {
                            assert!(
                                (got - want).abs() <= 1e-12 * want.abs().max(1e-300),
                                "{rate} {nper} {start}..{end} {kind}: {got}, not {want}"
                            );
                            checked += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(checked, 7 * 5 * 4 * 2 * 2);
    }

    /// VDB as it was computed before the closed form, one period at a time
    /// (`LibreOffice`'s `ScVDB`), for lives short enough to walk
    fn walked(
        cost: f64,
        salvage: f64,
        life: f64,
        start: f64,
        end: f64,
        factor: f64,
        no_switch: bool,
    ) -> f64 {
        use num_traits::ToPrimitive;
        let steps = |to: f64| 1..=to.to_u32().expect("a short life");
        if no_switch {
            let first = start.floor();
            let last = end.ceil();
            let mut total = 0.0;
            for step in steps(last) {
                let period = f64::from(step);
                if period < first + 1.0 {
                    continue;
                }
                let mut term = declining(cost, salvage, life, period, factor);
                if same(period, first + 1.0) {
                    term *= end.min(first + 1.0) - start;
                } else if same(period, last) {
                    term *= end + 1.0 - last;
                }
                total += term;
            }
            return total;
        }
        let switching = |cost: f64, remaining_life: f64, periods: f64| {
            let whole = periods.ceil();
            let mut total = 0.0;
            let mut left = cost - salvage;
            let mut straight_line = None;
            for step in steps(whole) {
                let period = f64::from(step);
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
                    term *= periods + 1.0 - whole;
                }
                total += term;
            }
            total
        };
        let (mut start, mut end, mut remaining_life) = (start, end, life);
        if start.fract() != 0.0 && factor > 1.0 && start >= life / 2.0 {
            let part = start - life / 2.0;
            start = life / 2.0;
            end -= part;
            remaining_life += 1.0;
        }
        let spent = switching(cost, remaining_life, start);
        switching(cost - spent, life - start, end - start)
    }

    /// The closed form of VDB gives what walking the periods gives, on
    /// 20,000 assets: lives of 1 to 10,000 periods and part periods, every
    /// kind of factor (none, below 1, the life and more), salvages from
    /// nothing to the cost, with and without the switch. Where walking
    /// leaves a millionth of the cost or less, both are as good as nothing.
    #[test]
    fn vdb_in_closed_form_is_vdb_walked_period_by_period() {
        use rand::rngs::StdRng;
        use rand::{RngExt, SeedableRng};
        let mut rng = StdRng::seed_from_u64(20_261_123);
        let lives = [
            1.0, 2.0, 3.0, 5.0, 10.0, 12.0, 30.0, 120.0, 365.0, 2500.0, 10_000.0, 1.5, 7.5, 10.25,
        ];
        let (mut compared, mut nothing) = (0, 0);
        for _ in 0..20_000 {
            let life = lives[rng.random_range(0..lives.len())];
            let base = [1.0, 1000.0, 2400.0, 50_000.0][rng.random_range(0..4)];
            let cost = rng.random::<f64>().mul_add(100.0, base);
            let salvage = cost * [0.0, 0.1, 0.3, 0.5, 0.9, 1.0][rng.random_range(0..6)];
            let factor = [
                0.0,
                0.5,
                1.0,
                1.5,
                2.0,
                3.0,
                life,
                life * 2.0,
                rng.random::<f64>() * 4.0,
            ][rng.random_range(0..9)];
            let mut a = rng.random::<f64>() * life;
            let mut b = rng.random::<f64>() * life;
            if rng.random_bool(0.4) {
                a = a.floor();
            }
            if rng.random_bool(0.4) {
                b = b.ceil().min(life);
            }
            let (start, end) = (a.min(b), a.max(b));
            let no_switch = rng.random_bool(0.3);
            let want = walked(cost, salvage, life, start, end, factor, no_switch);
            let got = depreciation(cost, salvage, life, start, end, factor, no_switch);
            let case =
                format!("VDB({cost}, {salvage}, {life}, {start}, {end}, {factor}, {no_switch})");
            if want.abs() > cost * 1e-6 {
                assert!(close(got, want, 1e-10), "{case}: {got}, not {want}");
                compared += 1;
            } else {
                assert!(got.abs() <= cost * 1e-6, "{case}: {got}, not {want}");
                nothing += 1;
            }
        }
        assert_eq!(compared + nothing, 20_000);
        assert!(compared > 10_000, "{compared}");
    }
}
