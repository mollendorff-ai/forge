//! Tests for how bare scalar names in formulas resolve

use super::ArrayCalculator;
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

fn model(scalars: &[(&str, Option<f64>, Option<&str>)]) -> ParsedModel {
    let mut model = ParsedModel::new();
    for (name, value, formula) in scalars {
        model.add_scalar(
            (*name).to_string(),
            Variable::new((*name).to_string(), *value, formula.map(str::to_string)),
        );
    }
    model
}

fn calculate(scalars: &[(&str, Option<f64>, Option<&str>)]) -> Result<ParsedModel, String> {
    ArrayCalculator::new(model(scalars))
        .calculate_all()
        .map_err(|e| e.to_string())
}

fn value(model: &ParsedModel, name: &str) -> f64 {
    model.scalars[name].value.unwrap()
}

#[test]
fn test_bare_name_uses_own_section_every_time() {
    // Same shape as examples/variance.yaml: two sections with the same names
    let scalars = [
        ("budget.revenue", Some(1000.0), None),
        ("budget.cogs", Some(400.0), None),
        ("budget.gross_profit", None, Some("=revenue - cogs")),
        ("actual.revenue", Some(950.0), None),
        ("actual.cogs", Some(420.0), None),
        ("actual.gross_profit", None, Some("=revenue - cogs")),
    ];
    // Hash map order changes between runs, so repeat
    for _ in 0..25 {
        let result = calculate(&scalars).unwrap();
        assert!((value(&result, "budget.gross_profit") - 600.0).abs() < 1e-9);
        assert!((value(&result, "actual.gross_profit") - 530.0).abs() < 1e-9);
    }
}

#[test]
fn test_ambiguous_bare_name_is_an_error() {
    let err = calculate(&[
        ("budget.revenue", Some(1000.0), None),
        ("actual.revenue", Some(950.0), None),
        ("summary.total", None, Some("=revenue * 2")),
    ])
    .unwrap_err();
    assert!(err.contains("ambiguous"), "{err}");
    assert!(err.contains("actual.revenue, budget.revenue"), "{err}");
}

#[test]
fn test_unique_bare_name_from_another_section_is_ordered() {
    for _ in 0..25 {
        let result = calculate(&[
            ("inputs.base", Some(10.0), None),
            ("calc.derived", None, Some("=base + 1")),
            ("summary.result", None, Some("=derived * 2")),
        ])
        .unwrap();
        assert!((value(&result, "summary.result") - 22.0).abs() < 1e-9);
    }
}

#[test]
fn test_own_section_beats_top_level_name() {
    let result = calculate(&[
        ("rate", Some(0.1), None),
        ("loan.rate", Some(0.2), None),
        ("loan.cost", None, Some("=rate * 100")),
        ("plain", None, Some("=rate * 100")),
    ])
    .unwrap();
    assert!((value(&result, "loan.cost") - 20.0).abs() < 1e-9);
    assert!((value(&result, "plain") - 10.0).abs() < 1e-9);
}

#[test]
fn test_ambiguous_bare_name_in_table_formula_is_an_error() {
    let mut model = model(&[("a.rate", Some(0.1), None), ("b.rate", Some(0.2), None)]);
    let mut table = Table::new("t".to_string());
    table.add_column(Column::new(
        "x".to_string(),
        ColumnValue::Number(vec![1.0, 2.0]),
    ));
    table.add_row_formula("y".to_string(), "=x * rate".to_string());
    model.add_table(table);
    let err = ArrayCalculator::new(model)
        .calculate_all()
        .unwrap_err()
        .to_string();
    assert!(err.contains("ambiguous"), "{err}");
}
