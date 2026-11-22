//! XIRR and XNPV: the functions of cash flows on dates (split from the rest
//! of the financial functions to keep each file under 1,500 lines).

use super::{require_sign_change, solve_rate};
use crate::core::array_calculator::evaluator::{
    evaluate, require_args, require_args_range, EvalContext, EvalError, Expr, Value,
};

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

/// Evaluate `XIRR`.
pub(super) fn eval_xirr(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
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
    let rate = solve_rate("XIRR", guess, scale, &|rate| {
        values
            .iter()
            .zip(&year_fracs)
            .map(|(cf, t)| cf / (1.0 + rate).powf(*t))
            .sum()
    })?;
    Ok(Value::Number(rate))
}

/// Evaluate `XNPV`.
pub(super) fn eval_xnpv(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
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
    // Excel gives an error for a date before the first (measured on iOS
    // 2026-11-19); for the same flows XIRR gives its rate, so it has no check.
    if dates.iter().any(|d| *d < dates[0]) {
        return Err(EvalError::new(
            "#NUM!: XNPV: a date comes before the first date",
        ));
    }
    let base_date = dates[0];
    let mut npv = 0.0;
    for (i, cf) in values.iter().enumerate() {
        let t = (dates[i] - base_date) / 365.0;
        npv += cf / (1.0 + rate).powf(t);
    }
    Ok(Value::Number(npv))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use super::*;

    use super::super::super::tests::number_in as eval_number;

    // Excel, measured on iOS 2026-11-19: XNPV with a date before the first is an
    // error, while XIRR with the same flows gives its rate (0.23587), unlike what
    // Microsoft's documentation says. 44197 is 2021-01-01, 43831 is 2020-01-01.
    #[test]
    fn test_xnpv_with_a_date_before_the_first_is_num_error() {
        let ctx = EvalContext::new();
        let err = eval("XNPV(0.1, {-100,-50,200}, {44197,43831,44562})", &ctx).unwrap_err();
        assert!(err.message.starts_with("#NUM!"), "{}", err.message);
        assert!(err.message.contains("XNPV"), "{}", err.message);
    }

    #[test]
    fn test_xnpv_with_dates_in_order_or_equal_to_the_first_calculates() {
        let ctx = EvalContext::new();
        let in_order = eval_number("XNPV(0.1, {-50,-100,200}, {43831,44197,44562})", &ctx);
        assert!((in_order - 24.360_745_394_926).abs() < 1e-9, "{in_order}");
        let same_day = eval_number("XNPV(0.1, {-50,-100,200}, {43831,43831,44562})", &ctx);
        assert!(same_day.is_finite(), "{same_day}");
    }

    #[test]
    fn test_xirr_with_a_date_before_the_first_calculates_as_excel_does() {
        let ctx = EvalContext::new();
        let rate = eval_number("XIRR({-100,-50,200}, {44197,43831,44562})", &ctx);
        assert!((rate - 0.235_869_752_954_134).abs() < 1e-9, "{rate}");
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

    /// An error in any argument is passed on, and text where a number is
    /// needed is refused
    #[test]
    fn each_argument_is_checked() {
        use super::super::super::tests::each_argument;
        let (flows, dates) = ("{-100, 200}", "{44197, 44562}");
        each_argument(
            "XIRR",
            &[
                (flows, None),
                (dates, Some("XIRR requires dates")),
                ("0.1", Some("XIRR guess must be a number")),
            ],
        );
        each_argument(
            "XNPV",
            &[
                ("0.1", Some("XNPV requires rate")),
                (flows, None),
                (dates, Some("XNPV requires dates")),
            ],
        );
    }

    /// A flow or a date that is not a number is left out with its pair when
    /// the two lists are as long as each other, and on its own when they are
    /// not; what is left must pair up. 44197 is 2021-01-01 and 44562 is
    /// 2022-01-01, a year of 365 days, so 100 that becomes 200 is 100%.
    #[test]
    fn flows_and_dates_that_are_not_numbers() {
        use super::super::super::tests::{fails_with, is_exactly};
        is_exactly("XIRR({-100, \"a\", 200}, {44197, 44300, 44562})", 1.0);
        is_exactly("XIRR({-100, 50, 200}, {44197, \"x\", 44562})", 1.0);
        is_exactly("XIRR({-100, \"a\", 200}, {44197, 44562})", 1.0);
        fails_with(
            "XIRR({-100, 200}, {44197, 44300, 44562})",
            "XIRR: values and dates must have same length",
        );
        fails_with(
            "XNPV(0.1, {-100, 200}, {44197, 44300, 44562})",
            "XNPV: values and dates must have same length",
        );
        fails_with(
            "XIRR({-100, 50, -100}, {44197, 44300, 44562})",
            "XIRR did not converge",
        );
    }
}
