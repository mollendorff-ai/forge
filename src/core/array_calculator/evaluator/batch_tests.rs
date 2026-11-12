//! Tests of the functions added in 10.1: schedules, conveniences, percentiles.
//! Their expected values were computed outside Forge (R, Python, by hand); the
//! Gnumeric suite (`statistics_finance_batch.yaml`) checks the same functions
//! against two spreadsheet engines.

use super::tests::eval;
use super::{EvalContext, Value};
use std::collections::HashMap;

fn context(columns: &[(&str, Vec<Value>)]) -> EvalContext {
    let mut ctx = EvalContext::new();
    let table: HashMap<String, Vec<Value>> = columns
        .iter()
        .map(|(name, values)| ((*name).to_string(), values.clone()))
        .collect();
    ctx.tables
        .insert("t".to_string(), table.into_iter().collect());
    ctx
}

fn numbers(values: &[f64]) -> Vec<Value> {
    values.iter().map(|v| Value::Number(*v)).collect()
}

fn number(formula: &str, ctx: &EvalContext) -> f64 {
    match eval(formula, ctx).unwrap() {
        Value::Number(n) => n,
        other => panic!("{formula}: {other:?}"),
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * b.abs().max(1.0)
}

fn error(formula: &str, ctx: &EvalContext) -> String {
    eval(formula, ctx).unwrap_err().to_string()
}

#[test]
fn rri_and_syd() {
    let ctx = EvalContext::new();
    // (2500 / 1000) ^ (1 / 10) - 1, and the sum of the years' digits for year 1 of 5
    assert!(close(
        number("RRI(10, 1000, 2500)", &ctx),
        0.095_958_226_385_217_23
    ));
    assert!(close(number("RRI(12, 5000, 5000)", &ctx), 0.0));
    assert!(close(number("SYD(10000, 1000, 5, 1)", &ctx), 3_000.0));
    assert!(close(number("SYD(10000, 1000, 5, 5)", &ctx), 600.0));
    assert!(error("RRI(0, 1000, 2500)", &ctx).contains("#NUM!"));
    assert!(error("RRI(5, 0, 2500)", &ctx).contains("#DIV/0!"));
    assert!(error("RRI(5, 100, -50)", &ctx).contains("#NUM!"));
    assert!(error("SYD(10000, 1000, 5, 6)", &ctx).contains("#NUM!"));
}

#[test]
fn cumulative_interest_and_principal_of_a_loan() {
    let ctx = EvalContext::new();
    // Microsoft's example: 9% a year, 30 years, 125,000; the second year's
    // interest is -11,135.23 and the principal -934.11 (a loop in R agrees)
    let interest = number("CUMIPMT(0.09/12, 360, 125000, 13, 24, 0)", &ctx);
    let principal = number("CUMPRINC(0.09/12, 360, 125000, 13, 24, 0)", &ctx);
    assert!((interest + 11_135.23).abs() < 0.005, "{interest}");
    assert!((principal + 934.107).abs() < 0.005, "{principal}");
    // Over the whole loan the principal paid is the loan, and interest plus
    // principal is the payments
    let all_principal = number("CUMPRINC(0.09/12, 360, 125000, 1, 360, 0)", &ctx);
    assert!(close(all_principal, -125_000.0));
    let payments = number("PMT(0.09/12, 360, 125000)", &ctx) * 360.0;
    let all_interest = number("CUMIPMT(0.09/12, 360, 125000, 1, 360, 0)", &ctx);
    assert!(close(all_interest + all_principal, payments));
    // A loan paid at the start of each period pays no interest in period 1
    assert!(close(
        number("CUMIPMT(0.05, 10, 20000, 1, 1, 1)", &ctx),
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
        assert!(error(bad, &ctx).contains("#NUM!"), "{bad}");
    }
}

#[test]
fn vdb_switches_to_the_straight_line_and_can_be_told_not_to() {
    let ctx = EvalContext::new();
    // Microsoft's examples with the answers they give: the first day, the
    // first month, the first year, and a factor of 1.5
    assert!((number("VDB(2400, 300, 3650, 0, 1)", &ctx) - 1.32).abs() < 0.005);
    assert!(close(number("VDB(2400, 300, 120, 0, 1)", &ctx), 40.0));
    assert!(close(number("VDB(2400, 300, 10, 0, 1)", &ctx), 480.0));
    assert!((number("VDB(2400, 300, 120, 6, 18, 1.5)", &ctx) - 311.81).abs() < 0.005);
    // Over the whole life the asset loses its cost less its salvage, switching or not
    assert!(close(number("VDB(2400, 300, 10, 0, 10)", &ctx), 2_100.0));
    // Without the switch the balance falls by a fixed share and stops at the salvage
    let no_switch = number("VDB(50000, 5000, 8, 0, 8, 2, TRUE)", &ctx);
    assert!(no_switch <= 45_000.0 + 1e-9);
    assert!(error("VDB(2400, 300, 10, 0, 11)", &ctx).contains("#NUM!"));
    assert!(error("VDB(2400, 300, 10, 5, 4)", &ctx).contains("#NUM!"));
    assert!(error("VDB(-1, 300, 10, 0, 1)", &ctx).contains("#NUM!"));
}

#[test]
fn mround_rounds_a_half_away_from_zero() {
    let ctx = EvalContext::new();
    assert!(close(number("MROUND(10, 3)", &ctx), 9.0));
    assert!(close(number("MROUND(-10, -3)", &ctx), -9.0));
    assert!(close(number("MROUND(5, 2)", &ctx), 6.0));
    assert!(close(number("MROUND(-5, -2)", &ctx), -6.0));
    // Microsoft's example: 1.3 to a multiple of 0.2 is 1.4
    assert!(close(number("MROUND(1.3, 0.2)", &ctx), 1.4));
    assert!(close(number("MROUND(7.5, 0)", &ctx), 0.0));
    assert!(error("MROUND(10, -3)", &ctx).contains("#NUM!"));
}

#[test]
fn week_numbers() {
    let ctx = EvalContext::new();
    // 2026-01-01 is a Thursday; Sunday-first weeks: week 1 ends Saturday the 3rd
    assert!(close(number("WEEKNUM(DATE(2026, 1, 3))", &ctx), 1.0));
    assert!(close(number("WEEKNUM(DATE(2026, 1, 4))", &ctx), 2.0));
    // Monday-first weeks (types 2 and 11): week 1 ends Sunday the 4th
    assert!(close(number("WEEKNUM(DATE(2026, 1, 4), 2)", &ctx), 1.0));
    assert!(close(number("WEEKNUM(DATE(2026, 1, 5), 11)", &ctx), 2.0));
    // ISO weeks: 2021-01-03 is in week 53 of 2020, 2024-12-30 in week 1 of 2025
    assert!(close(number("ISOWEEKNUM(DATE(2021, 1, 3))", &ctx), 53.0));
    assert!(close(number("ISOWEEKNUM(DATE(2024, 12, 30))", &ctx), 1.0));
    assert!(close(number("WEEKNUM(DATE(2021, 1, 3), 21)", &ctx), 53.0));
    assert!(error("WEEKNUM(DATE(2026, 1, 1), 5)", &ctx).contains("#NUM!"));
}

#[test]
fn geomean_percentile_and_quartile_exclusive() {
    let ctx = context(&[("v", numbers(&[3.5, 1.2, 9.9, 4.4, 7.7, 2.1, 8.8, 6.6]))]);
    // exp(mean(log(v))) and quantile(v, p, type = 6) in R
    assert!(close(number("GEOMEAN(0.5, 2, 8)", &ctx), 2.0));
    assert!(close(number("PERCENTILE.EXC(t.v, 0.3)", &ctx), 3.08));
    assert!(close(number("QUARTILE.EXC(t.v, 1)", &ctx), 2.45));
    assert!(close(number("PERCENTILE.INC(t.v, 0.3)", &ctx), 3.59));
    assert!(close(number("QUARTILE.INC(t.v, 2)", &ctx), 5.5));
    assert!(error("GEOMEAN(1, 0, 2)", &ctx).contains("#NUM!"));
    // With 8 values k has to lie between 1/9 and 8/9
    assert!(error("PERCENTILE.EXC(t.v, 0.05)", &ctx).contains("#NUM!"));
    assert!(error("PERCENTILE.EXC(t.v, 0.95)", &ctx).contains("#NUM!"));
    assert!(error("QUARTILE.EXC(t.v, 0)", &ctx).contains("#NUM!"));
    assert!(error("QUARTILE.EXC(t.v, 4)", &ctx).contains("#NUM!"));
}

#[test]
fn xmatch_exact_nearest_and_from_the_end() {
    let ctx = context(&[
        ("ten", numbers(&[10.0, 20.0, 30.0, 40.0, 50.0])),
        ("dup", numbers(&[5.0, 3.0, 5.0, 3.0, 5.0])),
    ]);
    assert!(close(number("XMATCH(30, t.ten)", &ctx), 3.0));
    // 35 is not there: the next smaller is 30 (position 3), the next larger 40 (position 4)
    assert!(close(number("XMATCH(35, t.ten, -1)", &ctx), 3.0));
    assert!(close(number("XMATCH(35, t.ten, 1)", &ctx), 4.0));
    assert!(error("XMATCH(35, t.ten)", &ctx).contains("not found"));
    // The first or the last of equal values
    assert!(close(number("XMATCH(5, t.dup)", &ctx), 1.0));
    assert!(close(number("XMATCH(5, t.dup, 0, -1)", &ctx), 5.0));
    // Of equal nearest values, the first met in the direction searched
    assert!(close(number("XMATCH(4, t.dup, -1)", &ctx), 2.0));
    assert!(close(number("XMATCH(4, t.dup, -1, -1)", &ctx), 4.0));
    // What is not supported says so
    assert!(error("XMATCH(30, t.ten, 2)", &ctx).contains("wildcards"));
    assert!(error("XMATCH(30, t.ten, 0, 2)", &ctx).contains("binary"));
}

#[test]
fn textjoin_joins_values_and_columns_and_can_skip_the_empty() {
    let ctx = context(&[(
        "names",
        vec![
            Value::Text("Ana".to_string()),
            Value::Text(String::new()),
            Value::Text("Bo".to_string()),
        ],
    )]);
    let text = |f: &str| match eval(f, &ctx).unwrap() {
        Value::Text(s) => s,
        other => panic!("{f}: {other:?}"),
    };
    assert_eq!(text("TEXTJOIN(\", \", TRUE, t.names)"), "Ana, Bo");
    assert_eq!(text("TEXTJOIN(\", \", FALSE, t.names)"), "Ana, , Bo");
    assert_eq!(text("TEXTJOIN(\"-\", TRUE, \"a\", 2, \"c\")"), "a-2-c");
    assert!(error("TEXTJOIN(\",\", TRUE)", &ctx).contains("at least 3"));
}
