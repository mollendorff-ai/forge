//! The five answers Excel gave differently from Forge when 40 formulas were
//! checked in Excel on 2026-11-14
//! (`validation/gnumeric/tests/edge/edge_excel_checks.yaml`), and what
//! follows from each rule.

#![allow(clippy::float_cmp)] // small whole numbers

use super::tests::eval;
use super::{EvalContext, Value};

fn number(formula: &str) -> f64 {
    match eval(formula, &EvalContext::new()) {
        Ok(Value::Number(n)) => n,
        other => panic!("{formula}: {other:?}"),
    }
}

fn text(formula: &str) -> String {
    match eval(formula, &EvalContext::new()) {
        Ok(Value::Text(t)) => t,
        other => panic!("{formula}: {other:?}"),
    }
}

fn fails(formula: &str) {
    assert!(eval(formula, &EvalContext::new()).is_err(), "{formula}");
}

#[test]
fn large_rounds_k_up_and_small_down() {
    // Excel: LARGE(.., 1.5) is 5 and SMALL(.., 1.5) is 1
    assert_eq!(number("LARGE({5,1,3,3,8}, 1.5)"), 5.0);
    assert_eq!(number("SMALL({5,1,3,3,8}, 1.5)"), 1.0);
    // Both engines: 1.2 is rounded up too, and 0.5 is the largest
    assert_eq!(number("LARGE({5,1,3,3,8}, 1.2)"), 5.0);
    assert_eq!(number("LARGE({5,1,3,3,8}, 0.5)"), 8.0);
    fails("LARGE({5,1,3,3,8}, 5.5)");
    fails("LARGE({5,1,3,3,8}, 0)");
}

#[test]
fn match_counts_a_type_by_its_sign() {
    assert_eq!(number("MATCH(3, {1,3,5}, 2)"), 2.0); // Excel
    assert_eq!(number("MATCH(3, {5,3,1}, -2)"), 2.0); // Gnumeric
    assert_eq!(number("MATCH(3, {1,3,5}, 0.5)"), 2.0); // cut to 0: exact
}

#[test]
fn match_halves_the_range_as_excel_does() {
    // Excel and both engines: 6, where a search from the first finds 1
    assert_eq!(number("MATCH(0, {1,3,5,7,9,11}, -1)"), 6.0);
    // The last of repeated values, in all three
    assert_eq!(number("MATCH(3, {1,3,3,5}, 1)"), 3.0);
    assert_eq!(number("MATCH(2, {1,2,2,2,3}, 1)"), 4.0);
    // Sorted values give what they gave
    assert_eq!(number("MATCH(6, {1,3,5,7,9,11}, 1)"), 3.0);
    assert_eq!(number("MATCH(6, {11,9,7,5,3,1}, -1)"), 3.0);
    assert_eq!(number("MATCH(\"b\", {\"a\",\"B\",\"c\"}, 1)"), 2.0);
    fails("MATCH(0, {1,3,5}, 1)");
    fails("MATCH(12, {11,9,7}, -1)");
    // A text among numbers is passed over (as Gnumeric does)
    assert_eq!(number("MATCH(4, {1,\"x\",3,5}, 1)"), 3.0);
    // A date is its day number
    assert_eq!(
        number("MATCH(DATE(2024,3,1), {\"2024-01-01\",\"2024-02-01\",\"2024-04-01\"}, 1)"),
        2.0
    );
}

#[test]
fn db_cuts_a_fraction_of_a_month() {
    assert_eq!(number("DB(10000, 1000, 5, 1, 6.5)"), 1845.0); // Excel
    assert_eq!(number("DB(10000, 1000, 5, 1, 6.9)"), 1845.0); // LibreOffice
}

#[test]
fn text_writes_no_minus_on_a_number_that_rounds_to_zero() {
    assert_eq!(text("TEXT(-0.004, \"0.00\")"), "0.00"); // Excel
    assert_eq!(text("TEXT(-0.006, \"0.00\")"), "-0.01");
    assert_eq!(text("TEXT(-0.4, \"0\")"), "0");
}
