//! Tests that table columns and scalars are calculated in one dependency order,
//! and that a formula scalar's stored value is never read

use super::ArrayCalculator;
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

fn scalar(model: &mut ParsedModel, name: &str, value: Option<f64>, formula: Option<&str>) {
    model.add_scalar(
        name.to_string(),
        Variable::new(name.to_string(), value, formula.map(str::to_string)),
    );
}

fn table(name: &str, data: &[(&str, Vec<f64>)], formulas: &[(&str, &str)]) -> Table {
    let mut table = Table::new(name.to_string());
    for (column, values) in data {
        table.add_column(Column::new(
            (*column).to_string(),
            ColumnValue::Number(values.clone()),
        ));
    }
    for (column, formula) in formulas {
        table.add_row_formula((*column).to_string(), (*formula).to_string());
    }
    table
}

fn numbers(model: &ParsedModel, table: &str, column: &str) -> Vec<f64> {
    match &model.tables[table].columns[column].values {
        ColumnValue::Number(v) => v.clone(),
        other => panic!("{table}.{column} is not numeric: {other:?}"),
    }
}

fn value(model: &ParsedModel, name: &str) -> f64 {
    model.scalars[name].value.unwrap()
}

#[test]
fn test_column_uses_recalculated_scalar_not_stored_value() {
    // The stored 999 is stale (from the file, an import or an earlier run)
    let mut model = ParsedModel::new();
    scalar(&mut model, "assumptions.base", Some(10.0), None);
    scalar(
        &mut model,
        "assumptions.double",
        Some(999.0),
        Some("=assumptions.base * 2"),
    );
    model.add_table(table(
        "t",
        &[("x", vec![1.0, 2.0, 3.0])],
        &[("y", "=x * assumptions.double")],
    ));

    let result = ArrayCalculator::new(model).calculate_all().unwrap();
    assert_eq!(numbers(&result, "t", "y"), vec![20.0, 40.0, 60.0]);
    assert!((value(&result, "assumptions.double") - 20.0).abs() < f64::EPSILON);
}

#[test]
fn test_column_uses_calculated_scalar_without_stored_value() {
    let mut model = ParsedModel::new();
    scalar(&mut model, "assumptions.base", Some(10.0), None);
    scalar(
        &mut model,
        "assumptions.double",
        None,
        Some("=assumptions.base * 2"),
    );
    model.add_table(table(
        "t",
        &[("x", vec![1.0, 2.0])],
        &[("y", "=x * double")],
    ));

    let result = ArrayCalculator::new(model).calculate_all().unwrap();
    assert_eq!(numbers(&result, "t", "y"), vec![20.0, 40.0]);
}

#[test]
fn test_share_of_total_is_not_a_cycle() {
    // share reads total; total sums an input column of the same table
    let mut model = ParsedModel::new();
    model.add_table(table(
        "t",
        &[("revenue", vec![100.0, 300.0])],
        &[("share", "=revenue / summary.total")],
    ));
    scalar(&mut model, "summary.total", None, Some("=SUM(t.revenue)"));
    scalar(&mut model, "summary.top", None, Some("=MAX(t.share)"));

    let result = ArrayCalculator::new(model).calculate_all().unwrap();
    assert_eq!(numbers(&result, "t", "share"), vec![0.25, 0.75]);
    assert!((value(&result, "summary.total") - 400.0).abs() < f64::EPSILON);
    assert!((value(&result, "summary.top") - 0.75).abs() < f64::EPSILON);
}

#[test]
fn test_column_in_one_table_uses_calculated_column_of_another() {
    let mut model = ParsedModel::new();
    model.add_table(table("a", &[("x", vec![1.0, 2.0])], &[("y", "=x * 10")]));
    model.add_table(table("b", &[("z", vec![3.0, 4.0])], &[("w", "=z + a.y")]));

    let result = ArrayCalculator::new(model).calculate_all().unwrap();
    assert_eq!(numbers(&result, "b", "w"), vec![13.0, 24.0]);
}

#[test]
fn test_cycle_through_a_column_and_a_scalar_is_named() {
    let mut model = ParsedModel::new();
    model.add_table(table("t", &[("x", vec![1.0, 2.0])], &[("y", "=x * s.k")]));
    scalar(&mut model, "s.k", None, Some("=SUM(t.y)"));

    let err = ArrayCalculator::new(model)
        .calculate_all()
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("formulas t.y, s.k depend on each other"),
        "{err}"
    );
}

#[test]
fn test_stored_value_of_a_failing_formula_is_not_used() {
    // Before, a stale stored value could stand in for a formula that was never
    // recalculated; now the formula must calculate or the model fails
    let mut model = ParsedModel::new();
    scalar(&mut model, "s.bad", Some(5.0), Some("=1 / 0"));
    scalar(&mut model, "s.uses_bad", None, Some("=s.bad + 1"));

    let err = ArrayCalculator::new(model)
        .calculate_all()
        .unwrap_err()
        .to_string();
    assert!(err.contains("s.bad: Division by zero"), "{err}");
}

#[test]
fn test_order_is_the_same_on_every_run() {
    // Several independent failures: the same one is reported every time
    let build = || {
        let mut model = ParsedModel::new();
        for name in ["s.a", "s.b", "s.c", "s.d"] {
            scalar(&mut model, name, None, Some("=1 / 0"));
        }
        model
    };
    let first = ArrayCalculator::new(build())
        .calculate_all()
        .unwrap_err()
        .to_string();
    for _ in 0..20 {
        let again = ArrayCalculator::new(build())
            .calculate_all()
            .unwrap_err()
            .to_string();
        assert_eq!(first, again);
    }
}
