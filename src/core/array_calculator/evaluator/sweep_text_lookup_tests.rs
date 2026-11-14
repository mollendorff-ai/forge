//! Wrong answers found by sweeping the statistical, lookup, math and text
//! functions against Gnumeric and `LibreOffice` (`validation/sweep`), each pinned
//! to what both engines give, or to Excel's documentation where they differ.

#![allow(clippy::float_cmp)] // exact values checked against both engines

use super::tests::eval;
use super::{EvalContext, Value};
use std::collections::HashMap;

fn context() -> EvalContext {
    let mut ctx = EvalContext::new();
    let text = |items: &[&str]| -> Vec<Value> {
        items
            .iter()
            .map(|s| Value::Text((*s).to_string()))
            .collect()
    };
    let numbers =
        |items: &[f64]| -> Vec<Value> { items.iter().map(|n| Value::Number(*n)).collect() };
    let table: HashMap<String, Vec<Value>> = [
        (
            "nm".to_string(),
            text(&["apple", "Banana", "cherry", "apple pie", "date", "b*"]),
        ),
        (
            "pr".to_string(),
            numbers(&[10.0, 20.0, 30.0, 40.0, 50.0, 60.0]),
        ),
        ("lu".to_string(), numbers(&[4.0, 2.0, 9.0, 2.0, 7.0, 4.0])),
        ("s".to_string(), text(&["a", "", "b", "c"])),
    ]
    .into_iter()
    .collect();
    ctx.tables
        .insert("t".to_string(), table.into_iter().collect());
    ctx
}

fn number(formula: &str) -> f64 {
    match eval(formula, &context()) {
        Ok(Value::Number(n)) => n,
        other => panic!("{formula}: {other:?}"),
    }
}

fn text(formula: &str) -> String {
    match eval(formula, &context()) {
        Ok(Value::Text(t)) => t,
        other => panic!("{formula}: {other:?}"),
    }
}

fn fails(formula: &str, code: &str) {
    match eval(formula, &context()) {
        Err(e) => assert!(e.to_string().contains(code), "{formula}: {e}"),
        Ok(v) => panic!("{formula} gave {v:?}, not {code}"),
    }
}

#[test]
fn wildcards_in_criteria_match_and_search() {
    assert_eq!(number("COUNTIF(t.nm, \"b*\")"), 2.0);
    assert_eq!(number("COUNTIF(t.nm, \"*e\")"), 3.0);
    assert_eq!(number("COUNTIF(t.nm, \"?pple\")"), 1.0);
    assert_eq!(number("COUNTIF(t.nm, \"b~*\")"), 1.0);
    assert_eq!(number("COUNTIF(t.nm, \"<>b*\")"), 4.0);
    assert_eq!(number("SUMIF(t.nm, \"apple*\", t.pr)"), 50.0);
    assert_eq!(number("COUNTIF(t.pr, \"*\")"), 0.0); // numbers never match a pattern
    assert_eq!(number("MATCH(\"c?erry\", t.nm, 0)"), 3.0);
    assert_eq!(number("MATCH(\"b~*\", t.nm, 0)"), 6.0);
    assert_eq!(number("XMATCH(\"ban*\", t.nm, 2)"), 2.0);
    assert_eq!(number("XMATCH(\"b*\", t.nm, 0)"), 6.0); // no wildcards in mode 0
    assert_eq!(number("SEARCH(\"?\", \"hello\", 4)"), 4.0);
    assert_eq!(number("SEARCH(\"~*\", \"a*b?c\")"), 2.0);
    assert_eq!(number("SEARCH(\"b?c\", \"a*b?c\")"), 3.0);
}

#[test]
fn xlookup_searches_from_the_end_when_asked() {
    assert_eq!(number("XLOOKUP(2, t.lu, t.pr, -1, 0, -1)"), 40.0);
    assert_eq!(number("XLOOKUP(2, t.lu, t.pr, -1, 0, 1)"), 20.0);
    assert_eq!(number("XLOOKUP(3, t.lu, t.pr, -1, 1, -1)"), 60.0);
}

#[test]
fn find_and_search_count_characters() {
    assert_eq!(number("FIND(\"γ\", \"αβγ\")"), 3.0);
    assert_eq!(number("FIND(\"β\", \"αβγ\", 2)"), 2.0); // panicked before
    assert_eq!(number("SEARCH(\"E\", \"straße\")"), 6.0);
    fails("FIND(\"l\", \"hello\", 0)", "#VALUE!");
    fails("SEARCH(\"l\", \"hello\", 6)", "#VALUE!");
}

#[test]
fn text_arguments_out_of_range_are_value_errors() {
    fails("LEFT(\"hello\", -1)", "#VALUE!");
    fails("RIGHT(\"hello\", -1)", "#VALUE!");
    fails("MID(\"hello\", 0, 2)", "#VALUE!");
    fails("MID(\"hello\", 1, -1)", "#VALUE!");
    fails("REPT(\"ab\", -1)", "#VALUE!");
    fails("REPLACE(\"hello\", 0, 1, \"Z\")", "#VALUE!");
    fails("SUBSTITUTE(\"hello\", \"l\", \"L\", 0)", "#VALUE!");
    assert_eq!(text("REPLACE(\"hello\", 20, 2, \"Q\")"), "helloQ"); // panicked before
    assert_eq!(text("MID(\"hello\", 2.9, 2.5)"), "el");
}

#[test]
fn concat_joins_a_column_and_numbers_keep_fifteen_digits() {
    assert_eq!(text("CONCAT(t.s)"), "abc");
    assert_eq!(text("CONCAT(0.1 + 0.2)"), "0.3");
    assert_eq!(text("CONCAT(1 / 3)"), "0.333333333333333");
    assert_eq!(text("CONCAT(1E+20)"), "1E+20");
    assert_eq!(number("LEN(2 / 3)"), 17.0);
}

#[test]
fn value_reads_what_excel_reads() {
    assert_eq!(number("VALUE(\"50%\")"), 0.5);
    assert_eq!(number("VALUE(\"$1,000\")"), 1000.0);
    assert_eq!(number("VALUE(\"(5)\")"), -5.0);
    assert_eq!(number("VALUE(\"5-\")"), -5.0);
    assert_eq!(number("VALUE(\"$-5\")"), -5.0);
    assert_eq!(number("VALUE(\"2024-01-15\")"), 45306.0);
    assert!((number("VALUE(\"12:30\")") - 12.5 / 24.0).abs() < 1e-15);
    for bad in ["inf", "nan", "1,2,3", "", "TRUE", "1.2.3"] {
        fails(&format!("VALUE(\"{bad}\")"), "#VALUE!");
    }
}

#[test]
fn rounding_works_on_fifteen_digits_as_excel_does() {
    assert_eq!(number("ROUND(1.005, 2)"), 1.01);
    assert_eq!(number("ROUND(0.285, 2)"), 0.29);
    assert_eq!(number("ROUND(-2.5, 0)"), -3.0);
    assert_eq!(number("ROUNDDOWN(4.35, 2)"), 4.35);
    assert_eq!(number("TRUNC(4.35, 2)"), 4.35);
    assert_eq!(number("ROUNDUP(0.1 + 0.2, 1)"), 0.3);
    assert_eq!(number("ROUND(1234.5678, -2)"), 1200.0);
    // 0.3 / 0.1 is 2.9999999999999996 in binary: 3 multiples, not 2
    assert!((number("FLOOR(0.3, 0.1)") - 0.3).abs() < 1e-15);
    fails("FLOOR(7.3, -1)", "#NUM!");
    fails("CEILING(7.3, -2)", "#NUM!");
    fails("FLOOR(7.3, 0)", "#DIV/0!");
    assert_eq!(number("CEILING(7.3, 0)"), 0.0);
    assert_eq!(number("FLOOR(-7.3, -2)"), -6.0);
}
