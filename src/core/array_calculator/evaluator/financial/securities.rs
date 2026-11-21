//! PRICEDISC, YIELDDISC and ACCRINT: the functions of securities, which take
//! dates and a day-count basis (split from the rest of the financial
//! functions to keep each file under 1,500 lines).

use super::optional_number;
use crate::core::array_calculator::evaluator::day_count::{self, Basis};
use crate::core::array_calculator::evaluator::{
    evaluate, number_or, parse_date_value, require_args_range, EvalContext, EvalError, Expr, Value,
};

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

/// Evaluate `PRICEDISC`.
pub(super) fn eval_pricedisc(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
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
    Ok(Value::Number(
        (discount * redemption).mul_add(-fraction, redemption),
    ))
}

/// Evaluate `YIELDDISC`.
pub(super) fn eval_yielddisc(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
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
    Ok(Value::Number((redemption - price) / price / fraction))
}

/// Evaluate `ACCRINT`.
pub(super) fn eval_accrint(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
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
    Ok(Value::Number(day_count::accrued_interest(
        issue,
        first_interest,
        settlement,
        rate,
        par,
        frequency,
        basis,
    )?))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::{each_argument, eval};
    use super::*;

    fn assert_close(formula: &str, expected: f64) {
        let got = super::super::super::tests::number(formula);
        assert!(
            (got - expected).abs() <= 1e-9 * expected.abs().max(1.0),
            "{formula}: got {got}, expected {expected}"
        );
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

    /// An error in any argument is passed on, and text where a date is
    /// needed is refused with the argument's name
    #[test]
    fn each_argument_is_checked() {
        let dates = ["\"2024-01-01\"", "\"2024-07-01\"", "\"2024-04-01\""];
        each_argument(
            "PRICEDISC",
            &[
                (dates[0], Some("PRICEDISC settlement")),
                (dates[1], Some("PRICEDISC maturity")),
                ("0.05", None),
                ("100", None),
                ("0", None),
            ],
        );
        each_argument(
            "YIELDDISC",
            &[
                (dates[0], Some("YIELDDISC settlement")),
                (dates[1], Some("YIELDDISC maturity")),
                ("95", None),
                ("100", None),
                ("0", None),
            ],
        );
        each_argument(
            "ACCRINT",
            &[
                (dates[0], Some("ACCRINT issue")),
                (dates[1], Some("ACCRINT first interest")),
                (dates[2], Some("ACCRINT settlement")),
                ("0.05", None),
                ("1000", None),
                ("2", None),
                ("0", None),
            ],
        );
    }

    /// ACCRINT with four coupons a year: a quarter's interest by 30/360
    #[test]
    fn accrint_quarterly() {
        use super::super::super::tests::{near, number};
        let quarter =
            number(r#"ACCRINT("2024-01-01", "2024-07-01", "2024-04-01", 0.05, 1000, 4, 0)"#);
        assert!(near(quarter, 12.5), "{quarter}");
    }

    /// A coupon past the last date there is: the schedule cannot be built
    #[test]
    fn accrint_past_the_last_date() {
        use super::super::super::tests::fails_with;
        let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
        let last = (chrono::NaiveDate::MAX - base).num_days();
        fails_with(
            &format!(
                "ACCRINT({}, {}, {last}, 0.05, 1000, 1)",
                last - 400,
                last - 200
            ),
            "ACCRINT: cannot build the coupon schedule",
        );
    }
}
