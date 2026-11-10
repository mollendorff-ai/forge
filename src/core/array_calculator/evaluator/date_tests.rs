//! A date is a date (ADR-054): the text `2026-01-31` in a model, and its day
//! number wherever a formula reads a number. `+` adds and `&` joins, as in
//! Excel. Each result here is what Gnumeric and `LibreOffice` give for the
//! exported formula over a real date (forge-e2e-gnumeric,
//! `edge_real_dates.yaml`).

use super::tests::eval;
use super::{EvalContext, Value};

fn number(formula: &str) -> f64 {
    match eval(formula, &EvalContext::new()).unwrap() {
        Value::Number(n) => n,
        other => panic!("{formula}: {other:?}"),
    }
}

fn text(formula: &str) -> String {
    match eval(formula, &EvalContext::new()).unwrap() {
        Value::Text(t) => t,
        other => panic!("{formula}: {other:?}"),
    }
}

fn truth(formula: &str) -> bool {
    match eval(formula, &EvalContext::new()).unwrap() {
        Value::Boolean(b) => b,
        other => panic!("{formula}: {other:?}"),
    }
}

/// `+` joined text when either side was text: `"5" + 1` was `"51"` (Excel:
/// 6) and a date plus a day was `"2026-01-311"` (Excel: the next day).
#[test]
fn plus_adds_as_in_excel() {
    assert!((number("\"5\" + 1") - 6.0).abs() < 1e-12);
    assert!((number("\"2026-01-31\" + 1") - 46054.0).abs() < 1e-12);
    assert!((number("TRUE + 1") - 2.0).abs() < 1e-12);
    let error = eval("\"a\" + \"b\"", &EvalContext::new())
        .unwrap_err()
        .to_string();
    assert!(error.contains("#VALUE!"), "{error}");
    assert!(error.contains("use & or CONCAT"), "{error}");
}

#[test]
fn ampersand_joins_text() {
    assert_eq!(text("\"a\" & \"b\""), "ab");
    assert_eq!(text("\"n\" & 1"), "n1");
    assert_eq!(text("1 & 2"), "12");
    assert_eq!(text("\"due \" & \"2026-01-31\""), "due 2026-01-31");
    // It binds less than + and more than a comparison, as in Excel
    assert_eq!(text("\"x\" & 1 + 2"), "x3");
    assert!(truth("\"a\" & \"b\" = \"ab\""));
}

/// `d = DATE(2026, 1, 31)` was FALSE for the date 2026-01-31.
#[test]
fn a_date_equals_its_day_number() {
    assert!(truth("\"2026-01-31\" = DATE(2026, 1, 31)"));
    assert!(truth("DATE(2026, 1, 31) = \"2026-01-31\""));
    assert!(!truth("\"2026-01-31\" <> 46053"));
    assert!(!truth("\"2026-01-31\" = 46054"));
    // Other text is never equal to a number, as in Excel
    assert!(!truth("\"5\" = 5"));
    assert!(truth("\"2026-01-31\" = \"2026-01-31\""));
}

/// `MAX(d, DATE(2026, 6, 1))` ignored the date `d` of a row, and
/// `COUNT(d, e, n)` counted only the number.
#[test]
fn a_single_date_counts_as_a_number_in_aggregates() {
    assert!((number("MAX(\"2026-12-01\", DATE(2026, 6, 1))") - 46357.0).abs() < 1e-12);
    assert!((number("MIN(\"2026-01-31\", \"2026-02-01\")") - 46053.0).abs() < 1e-12);
    assert!((number("SUM(\"2026-01-31\", 1)") - 46054.0).abs() < 1e-12);
    assert!((number("AVERAGE(\"2026-01-31\", \"2026-02-02\")") - 46054.0).abs() < 1e-12);
    assert!((number("COUNT(\"2026-01-31\", 1, \"x\")") - 2.0).abs() < 1e-12);
    // Text that is not a date is still not a number here
    assert!((number("SUM(\"x\", 1)") - 1.0).abs() < 1e-12);
}

/// `TEXT(d, "yyyy")` was the day number as text ("46053").
#[test]
fn text_formats_a_date() {
    assert_eq!(text("TEXT(\"2026-01-31\", \"yyyy\")"), "2026");
    assert_eq!(
        text("TEXT(DATE(2026, 1, 31), \"mmm d, yyyy\")"),
        "Jan 31, 2026"
    );
    assert_eq!(text("TEXT(1234.5, \"#,##0.00\")"), "1,234.50");
    let error = eval("TEXT(1, \"h:mm\")", &EvalContext::new())
        .unwrap_err()
        .to_string();
    assert!(error.contains("is not one Forge reads"), "{error}");
}
