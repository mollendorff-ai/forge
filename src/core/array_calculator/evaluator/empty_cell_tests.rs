//! An empty cell (ADR-059): the table of the ADR, measured in Gnumeric 1.12
//! and `LibreOffice` 26.2 on 2026-11-12, which agree on every value here.
//! `x` is 10, empty, 30, empty, 0; `blank` is one empty cell; `b` is a column
//! of empty cells.

use super::tests::eval;
use super::{EvalContext, Value};
use std::collections::HashMap;

fn context() -> EvalContext {
    let mut ctx = EvalContext::new();
    let n = Value::Number;
    let mut t: HashMap<String, Vec<Value>> = HashMap::new();
    t.insert(
        "x".into(),
        vec![n(10.0), Value::Null, n(30.0), Value::Null, n(0.0)],
    );
    t.insert("b".into(), vec![Value::Null; 5]);
    ctx.tables.insert("t".into(), t.into_iter().collect());
    ctx.scalars.insert("blank".into(), Value::Null);
    ctx
}

#[test]
fn an_empty_cell_is_what_excel_reads_it_as() {
    let ctx = context();
    let b = Value::Boolean;
    for (formula, expected) in [
        ("blank + 1", Value::Number(1.0)),
        ("blank * 5", Value::Number(0.0)),
        ("-blank", Value::Number(0.0)),
        ("blank = 0", b(true)),
        ("blank = \"\"", b(true)),
        ("blank = FALSE", b(true)),
        ("blank <> 0", b(false)),
        ("blank <> \"\"", b(false)),
        ("blank = 1", b(false)),
        ("blank < 1", b(true)),
        ("ISBLANK(blank)", b(true)),
        ("ISNUMBER(blank)", b(false)),
        ("IF(blank, 1, 2)", Value::Number(2.0)),
        ("\"[\" & blank & \"]\"", Value::Text("[]".into())),
        ("LEN(blank)", Value::Number(0.0)),
        ("ROUND(blank, 0)", Value::Number(0.0)),
        ("ABS(blank)", Value::Number(0.0)),
    ] {
        assert_eq!(eval(formula, &ctx).unwrap(), expected, "{formula}");
    }
}

#[test]
fn aggregates_skip_an_empty_cell_and_count_a_zero() {
    let ctx = context();
    for (formula, expected) in [
        ("SUM(t.x)", 40.0),
        ("COUNT(t.x)", 3.0),
        ("COUNTA(t.x)", 3.0),
        ("COUNTBLANK(t.x)", 2.0),
        ("AVERAGE(t.x)", 40.0 / 3.0),
        ("MEDIAN(t.x)", 10.0),
        ("MIN(t.x)", 0.0),
        ("MAX(t.x)", 30.0),
        ("PRODUCT(t.x)", 0.0),
        ("LARGE(t.x, 3)", 0.0),
        ("SMALL(t.x, 1)", 0.0),
        ("RANK(30, t.x)", 1.0),
        ("PERCENTILE(t.x, 0.5)", 10.0),
        ("SUMPRODUCT(t.x, t.x)", 1000.0),
        ("COUNTIF(t.x, \"\")", 2.0),
        ("COUNTIF(t.x, \"=\")", 2.0),
        ("COUNTIF(t.x, \"<>\")", 3.0),
        ("COUNTIF(t.x, 0)", 1.0),
        ("COUNTIFS(t.x, \"\")", 2.0),
        ("SUMIF(t.x, \">=0\")", 40.0),
        ("AVERAGEIF(t.x, \">=0\")", 40.0 / 3.0),
        // 10 / 1.1 + 30 / 1.1^2 + 0 / 1.1^3: the empty cells are skipped
        ("NPV(0.1, t.x)", 10.0 / 1.1 + 30.0 / 1.21),
    ] {
        match eval(formula, &ctx).unwrap() {
            Value::Number(n) => assert!((n - expected).abs() < 1e-12, "{formula}: {n}"),
            other => panic!("{formula}: {other:?}"),
        }
    }
    // STDEV of 10, 30, 0, the empty cells left out: R gives sd(c(10, 30, 0))
    match eval("STDEV(t.x)", &ctx).unwrap() {
        Value::Number(n) => assert!((n - 15.275_252_316_519_467).abs() < 1e-12, "{n}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_column_of_only_empty_cells() {
    let ctx = context();
    for formula in ["SUM(t.b)", "COUNT(t.b)", "MAX(t.b)", "MIN(t.b)"] {
        // 0, and not -0 or an error
        let value = eval(formula, &ctx).unwrap();
        assert!(
            matches!(value, Value::Number(n) if n == 0.0 && n.is_sign_positive()),
            "{formula}: {value:?}"
        );
    }
    assert!(eval("AVERAGE(t.b)", &ctx).is_err());
    assert_eq!(eval("COUNTBLANK(t.b)", &ctx).unwrap(), Value::Number(5.0));
}
