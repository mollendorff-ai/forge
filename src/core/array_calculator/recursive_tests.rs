//! Tests of `PREV` and of columns calculated together row by row (ADR-053).
//! Expected numbers are worked out by hand in each test.

use super::ArrayCalculator;
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

fn model(data: &[(&str, Vec<f64>)], formulas: &[(&str, &str)]) -> ParsedModel {
    let mut table = Table::new("t".to_string());
    for (column, values) in data {
        table.add_column(Column::new(
            (*column).to_string(),
            ColumnValue::Number(values.clone()),
        ));
    }
    for (column, formula) in formulas {
        table.add_row_formula((*column).to_string(), (*formula).to_string());
    }
    let mut model = ParsedModel::new();
    model.add_table(table);
    model
}

fn numbers(model: &ParsedModel, column: &str) -> Vec<f64> {
    match &model.tables["t"].columns[column].values {
        ColumnValue::Number(v) => v.clone(),
        other => panic!("t.{column} is not numeric: {other:?}"),
    }
}

fn calculate(model: ParsedModel) -> Result<ParsedModel, String> {
    ArrayCalculator::new(model)
        .calculate_all()
        .map_err(|e| e.to_string())
}

#[test]
fn a_column_reads_its_own_row_above() {
    let m = calculate(model(
        &[("x", vec![10.0, 20.0, 30.0, 40.0])],
        &[
            ("running", "=PREV(running, 0) + x"),
            ("growth", "=PREV(growth, 100) * 1.1"),
        ],
    ))
    .unwrap();
    assert_eq!(numbers(&m, "running"), [10.0, 30.0, 60.0, 100.0]);
    // 100 * 1.1 in the first row, then 1.1 times the row above
    let growth = numbers(&m, "growth");
    for (got, want) in growth.iter().zip([110.0, 121.0, 133.1, 146.41]) {
        assert!((got - want).abs() < 1e-9, "{growth:?}");
    }
}

/// opening reads closing in the row above; closing reads opening in its own
/// row. A loan of 1000 at 1% a period, paying 300:
/// 1000 -> +10 -300 = 710 -> +7.10 -300 = 417.10 -> +4.171 -300 = 121.271
#[test]
fn columns_that_read_each_other_are_calculated_row_by_row() {
    let mut m = model(
        &[("n", vec![1.0, 2.0, 3.0])],
        &[
            ("opening", "=PREV(closing, loan.principal)"),
            ("interest", "=opening * 0.01"),
            ("closing", "=opening + interest - 300"),
        ],
    );
    m.add_scalar(
        "loan.principal".to_string(),
        Variable::new("loan.principal".to_string(), Some(1000.0), None),
    );
    m.add_scalar(
        "loan.left".to_string(),
        Variable::new(
            "loan.left".to_string(),
            None,
            Some("=INDEX(t.closing, 3)".to_string()),
        ),
    );
    let m = calculate(m).unwrap();
    assert_eq!(numbers(&m, "opening"), [1000.0, 710.0, 417.1]);
    let closing = numbers(&m, "closing");
    for (got, want) in closing.iter().zip([710.0, 417.1, 121.271]) {
        assert!((got - want).abs() < 1e-9, "{closing:?}");
    }
    // A scalar that reads the group is calculated after it
    assert!((m.scalars["loan.left"].value.unwrap() - 121.271).abs() < 1e-9);
}

#[test]
fn prev_reads_any_expression_and_any_number_of_rows_back() {
    let m = calculate(model(
        &[("x", vec![1.0, 2.0, 3.0, 4.0])],
        &[
            ("two_back", "=PREV(x, -1, 2)"),
            ("sum_above", "=PREV(x + x * 10, 0)"),
            ("change", "=x - PREV(x, x)"),
            ("row", "=PREV(ROW(), 0)"),
            // The first-row value is read in the formula's own row
            ("first", "=PREV(x, x * 100)"),
            ("nested", "=PREV(PREV(x, 7), 9)"),
        ],
    ))
    .unwrap();
    assert_eq!(numbers(&m, "two_back"), [-1.0, -1.0, 1.0, 2.0]);
    assert_eq!(numbers(&m, "sum_above"), [0.0, 11.0, 22.0, 33.0]);
    assert_eq!(numbers(&m, "change"), [0.0, 1.0, 1.0, 1.0]);
    assert_eq!(numbers(&m, "row"), [0.0, 1.0, 2.0, 3.0]);
    assert_eq!(numbers(&m, "first"), [100.0, 1.0, 2.0, 3.0]);
    assert_eq!(numbers(&m, "nested"), [9.0, 7.0, 1.0, 2.0]);
}

/// A column that another reads only through PREV, without a cycle, is still
/// calculated first as a whole, and a text column works row by row too.
#[test]
fn a_lagged_read_without_a_cycle_and_a_text_column() {
    let m = calculate(model(
        &[("x", vec![1.0, 2.0, 3.0])],
        &[
            ("double", "=x * 2"),
            ("above", "=PREV(double, 0)"),
            (
                "label",
                "=IF(PREV(label, \"\") = \"\", \"first\", \"later\")",
            ),
        ],
    ))
    .unwrap();
    assert_eq!(numbers(&m, "above"), [0.0, 2.0, 4.0]);
    match &m.tables["t"].columns["label"].values {
        ColumnValue::Text(v) => assert_eq!(v, &["first", "later", "later"]),
        other => panic!("{other:?}"),
    }
}

#[test]
fn what_prev_refuses() {
    for (formulas, why) in [
        // A column that reads itself in its own row is still circular
        (
            vec![("a", "=a + 1")],
            "refers to itself (to read the row above, write PREV",
        ),
        (
            vec![("a", "=b + 1"), ("b", "=a * 2")],
            "depend on each other",
        ),
        // A cycle within the row stays one, whatever PREV does elsewhere
        (
            vec![("a", "=PREV(b, 0) + b"), ("b", "=a * 2")],
            "depend on each other",
        ),
        (
            vec![("a", "=PREV(x)")],
            "PREV takes a value, what to use in the first row",
        ),
        (vec![("a", "=PREV(x, 0, 0)")], "whole number, 1 or more"),
        (vec![("a", "=PREV(x, 0, 1.5)")], "whole number, 1 or more"),
        // Reading a later row of a column being calculated
        (
            vec![("a", "=PREV(b, 0) + 1"), ("b", "=a + INDEX(a, 3)")],
            "known only up to the row being calculated",
        ),
    ] {
        let err = calculate(model(&[("x", vec![1.0, 2.0, 3.0])], &formulas)).unwrap_err();
        assert!(err.contains(why), "{formulas:?}: {err}");
    }

    // PREV has no row above in a scalar
    let mut m = model(&[("x", vec![1.0, 2.0])], &[]);
    m.add_scalar(
        "s".to_string(),
        Variable::new("s".to_string(), None, Some("=PREV(1, 2)".to_string())),
    );
    assert!(calculate(m)
        .unwrap_err()
        .contains("only in a table's row formula"));

    // A cycle through a scalar needs the whole column before its first row
    let mut m = model(
        &[("x", vec![1.0, 2.0])],
        &[("a", "=PREV(b, 0) + 1"), ("b", "=total * 2")],
    );
    m.add_scalar(
        "total".to_string(),
        Variable::new("total".to_string(), None, Some("=SUM(t.a)".to_string())),
    );
    let err = calculate(m).unwrap_err();
    assert!(err.contains("only among columns of one table"), "{err}");
}

/// PREV of a column of another table reads that table's row above (the other
/// table is calculated first), and a column may name itself in full.
#[test]
fn prev_across_tables_and_by_full_name() {
    let mut m = model(
        &[("v", vec![1.0, 2.0, 3.0, 4.0])],
        &[("cum", "=PREV(t.cum, 0) + v")],
    );
    let mut other = Table::new("u".to_string());
    other.add_column(Column::new(
        "w".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
    ));
    other.add_row_formula("from_t".to_string(), "=PREV(t.cum, -1) + w".to_string());
    m.add_table(other);
    let m = calculate(m).unwrap();
    assert_eq!(numbers(&m, "cum"), [1.0, 3.0, 6.0, 10.0]);
    match &m.tables["u"].columns["from_t"].values {
        ColumnValue::Number(v) => assert_eq!(v, &[9.0, 21.0, 33.0, 46.0]),
        other => panic!("{other:?}"),
    }
}

/// The error of a row names the column and the row, and the other columns of
/// the group are not left half calculated.
#[test]
fn an_error_in_a_row_names_the_column_and_the_row() {
    let err = calculate(model(
        &[("x", vec![1.0, 0.0, 3.0])],
        &[("a", "=PREV(b, 1) / x"), ("b", "=a + 1")],
    ))
    .unwrap_err();
    assert!(err.contains("t.a row 2:"), "{err}");
}
