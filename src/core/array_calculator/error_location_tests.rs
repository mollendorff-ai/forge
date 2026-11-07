//! Tests that calculation errors name the column or scalar that failed

use super::ArrayCalculator;
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

fn with_scalars(scalars: &[(&str, Option<f64>, Option<&str>)]) -> ParsedModel {
    let mut model = ParsedModel::new();
    for (name, value, formula) in scalars {
        model.add_scalar(
            (*name).to_string(),
            Variable::new((*name).to_string(), *value, formula.map(str::to_string)),
        );
    }
    model
}

fn error(model: ParsedModel) -> String {
    ArrayCalculator::new(model)
        .calculate_all()
        .unwrap_err()
        .to_string()
}

#[test]
fn test_scalar_error_names_the_scalar_and_formula() {
    let err = error(with_scalars(&[
        ("assumptions.a", Some(10.0), None),
        ("calc.x", None, Some("=assumptions.a / 0")),
    ]));
    assert!(
        err.contains("calc.x: Division by zero (formula: =assumptions.a / 0)"),
        "{err}"
    );
    assert!(!err.contains("Eval error"), "{err}");
    assert!(!err.contains("Eval: "), "{err}");
}

#[test]
fn test_unknown_function_names_the_scalar() {
    let err = error(with_scalars(&[("calc.x", None, Some("=FOOBAR(1)"))]));
    assert!(err.contains("calc.x: Unknown function: FOOBAR"), "{err}");
}

#[test]
fn test_syntax_error_names_the_scalar() {
    let err = error(with_scalars(&[("calc.x", None, Some("=1 +* 2"))]));
    assert!(err.contains("calc.x: syntax error:"), "{err}");
}

#[test]
fn test_row_error_names_column_and_one_based_row() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("projections".to_string());
    table.add_column(Column::new(
        "year".to_string(),
        ColumnValue::Number(vec![1.0, 2.0, 3.0]),
    ));
    table.add_column(Column::new(
        "cost".to_string(),
        ColumnValue::Number(vec![10.0, 0.0, 5.0]),
    ));
    table.add_row_formula("ratio".to_string(), "=year / cost".to_string());
    model.add_table(table);

    let err = error(model);
    assert!(
        err.contains("projections.ratio row 2: Division by zero (formula: =year / cost)"),
        "{err}"
    );
}

#[test]
fn test_scalar_cycle_lists_its_members() {
    let err = error(with_scalars(&[
        ("calc.a", None, Some("=calc.b + 1")),
        ("calc.b", None, Some("=calc.a + 1")),
        ("calc.c", Some(1.0), None),
    ]));
    assert!(
        err.contains("scalars calc.a, calc.b depend on each other"),
        "{err}"
    );
    assert!(!err.contains("calc.c"), "{err}");
}

#[test]
fn test_self_reference_is_named() {
    let err = error(with_scalars(&[("calc.a", None, Some("=calc.a + 1"))]));
    assert!(err.contains("calc.a refers to itself"), "{err}");
}

#[test]
fn test_column_cycle_lists_qualified_columns() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("t".to_string());
    table.add_column(Column::new(
        "base".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]),
    ));
    table.add_row_formula("p".to_string(), "=q + base".to_string());
    table.add_row_formula("q".to_string(), "=p + 1".to_string());
    model.add_table(table);

    let err = error(model);
    assert!(
        err.contains("columns t.p, t.q depend on each other"),
        "{err}"
    );
}

#[test]
fn test_one_failure_keeps_its_own_message() {
    let err = error(with_scalars(&[
        ("calc.x", None, Some("=1 / 0")),
        ("calc.fine", None, Some("=2 * 3")),
    ]));
    assert_eq!(
        err,
        "Formula evaluation error: calc.x: Division by zero (formula: =1 / 0)"
    );
}

#[test]
fn test_every_failure_is_reported_in_name_order() {
    let err = error(with_scalars(&[
        ("calc.y", None, Some("=FOOBAR(1)")),
        ("calc.x", None, Some("=1 / 0")),
        ("calc.fine", None, Some("=2 * 3")),
    ]));
    assert!(err.contains("2 formulas failed:"), "{err}");
    let x = err.find("calc.x: Division by zero").expect(&err);
    let y = err.find("calc.y: Unknown function: FOOBAR").expect(&err);
    assert!(x < y, "{err}");
    assert!(!err.contains("calc.fine"), "{err}");
}

#[test]
fn test_cells_that_use_a_failed_cell_are_named_not_calculated() {
    let mut model = with_scalars(&[
        ("calc.x", None, Some("=1 / 0")),
        ("calc.uses_x", Some(42.0), Some("=calc.x + 1")),
        ("calc.uses_uses_x", None, Some("=calc.uses_x * 2")),
    ]);
    let mut table = Table::new("t".to_string());
    table.add_column(Column::new(
        "n".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]),
    ));
    table.add_row_formula("m".to_string(), "=n * calc.x".to_string());
    table.add_row_formula("ok".to_string(), "=n * 2".to_string());
    model.add_table(table);

    let err = error(model);
    assert!(err.contains("1 formula failed:"), "{err}");
    assert!(err.contains("calc.uses_x (uses calc.x)"), "{err}");
    assert!(err.contains("calc.uses_uses_x (uses calc.uses_x)"), "{err}");
    assert!(err.contains("t.m (uses calc.x)"), "{err}");
    // The stored 42 is never reported or used as a result
    assert!(!err.contains("42"), "{err}");
    assert!(!err.contains("t.ok"), "{err}");
}
