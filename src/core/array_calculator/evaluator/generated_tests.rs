//! Fixes for what the generated cases found (ADR-062) that the engines cannot
//! check: answers Forge gives on its own, with the reason, and the exact
//! values where every engine loses digits.

#![allow(clippy::float_cmp)] // exact values, worked with 50-digit decimals

use super::tests::eval;
use super::{EvalContext, Value};

fn number(formula: &str) -> f64 {
    match eval(formula, &EvalContext::new()) {
        Ok(Value::Number(n)) => n,
        other => panic!("{formula}: {other:?}"),
    }
}

fn fails(formula: &str, code: &str) {
    match eval(formula, &EvalContext::new()) {
        Err(e) => assert!(e.to_string().contains(code), "{formula}: {e}"),
        Ok(v) => panic!("{formula} gave {v:?}, not {code}"),
    }
}

fn close(formula: &str, exact: f64) {
    let got = number(formula);
    assert!(
        (got - exact).abs() <= 1e-14 * exact.abs().max(1.0),
        "{formula}: {got}, exact {exact}"
    );
}

#[test]
fn a_negative_time_is_no_time() {
    fails("HOUR(-0.5)", "#NUM!");
    fails("MINUTE(-1E-12)", "#NUM!");
    fails("SECOND(-2.5)", "#NUM!");
    assert_eq!(number("HOUR(0.999999999)"), 0.0); // 24 before
}

#[test]
fn na_is_an_error_and_an_empty_cell_is_not() {
    assert_eq!(number("TYPE(NA())"), 16.0);
    assert_eq!(
        eval("ISBLANK(NA())", &EvalContext::new()).unwrap(),
        Value::Boolean(false)
    );
    fails("NA() + 1", "#N/A");
    assert_eq!(number("IFNA(NA(), 5)"), 5.0);
    fails("IFNA(1/0, 5)", "");
}

#[test]
fn mod_is_the_exact_remainder() {
    // n - d * INT(n / d) in floating point gave -5.8E-16 and 0.0068 (above d)
    let r = number("MOD(7.3, 0.1)");
    assert!(r > 0.0 && r < 0.1, "{r}");
    let r = number("MOD(1E15, 1E-12)");
    assert!((0.0..1e-12).contains(&r), "{r}");
    assert_eq!(number("MOD(-7, 3)"), 2.0);
    assert_eq!(number("MOD(7, -3)"), -2.0);
}

#[test]
fn annuities_keep_their_digits() {
    // Exact to 50 digits for the double inputs (2026-11-14)
    close(
        "NPER(1e-09, 3.944973623587, 0, 2.5, 1)",
        -0.633_717_798_414_327_9,
    );
    close("NPER(0.1, -1e-12, -2.5)", -275.361_213_342_520_2);
    close(
        "NPER(0.1, 0.285, 1E15, -0.559709076024, 1)",
        -348.670_778_361_094_7,
    );
    close(
        "CUMPRINC(0.5, 61, 487, 3, 36, 0)",
        -0.019_286_216_489_415_402,
    );
    close(
        "CUMPRINC(0.5, 60, 0.663097621158, 43, 52, 1)",
        -0.016_949_572_163_691_61,
    );
    close("CUMPRINC(0.03, 24, 900, 5, 9, 1)", -151.665_053_343_195_1);
    close("CUMPRINC(0.05, 30, 1000, 1, 30, 1)", -1000.0);
    close("RATE(276, -100, 1E15)", -0.095_099_553_259_213_3);
}

#[test]
fn rank_counts_the_values_above_exactly() {
    let mut ctx = EvalContext::new();
    let v: Vec<Value> = [629.65, 1.005, 1e-12, -0.0, 0.0, -0.93]
        .iter()
        .map(|n| Value::Number(*n))
        .collect();
    ctx.tables.insert(
        "t".to_string(),
        std::iter::once(("v".to_string(), v.into())).collect(),
    );
    assert_eq!(eval("RANK.EQ(0, t.v)", &ctx).unwrap(), Value::Number(4.0));
    assert_eq!(
        eval("RANK.EQ(0, t.v, 1)", &ctx).unwrap(),
        Value::Number(2.0)
    );
}

#[test]
fn a_number_as_text_rounds_its_shortest_form() {
    match eval("CONCAT(-985667.1985084685)", &EvalContext::new()).unwrap() {
        Value::Text(t) => assert_eq!(t, "-985667.198508469"),
        other => panic!("{other:?}"),
    }
    match eval("CONCAT(9.99999999999999999)", &EvalContext::new()).unwrap() {
        Value::Text(t) => assert_eq!(t, "10"),
        other => panic!("{other:?}"),
    }
}
