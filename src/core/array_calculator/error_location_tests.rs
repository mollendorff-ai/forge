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
