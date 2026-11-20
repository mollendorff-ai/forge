//! Errors that say where they happened: the column and one-based row, or
//! the scalar, with its formula, and the members of a cycle.

use super::evaluator;
use super::graph::Graph;
use super::order::Cell;
use crate::error::ForgeError;

/// Name the column or scalar whose formula failed, and show the formula, so an
/// error points at the place to fix (`calc.x: Division by zero`). Only
/// evaluation errors are rewritten; other errors already say where they are.
pub(super) fn locate_error(err: ForgeError, location: &str, formula: &str) -> ForgeError {
    match err {
        ForgeError::Eval(msg) => {
            // Row errors read "row 3: ..."; keep them on the location's line
            let sep = if msg.starts_with("row ") { " " } else { ": " };
            ForgeError::Eval(format!("{location}{sep}{msg} (formula: {formula})"))
        },
        other => other,
    }
}

/// The error for a calculation in which cells failed. One failure keeps its
/// own message; several are listed, followed by the cells that were not
/// calculated because they use a failed one.
pub(super) fn failure_report(failures: &[String], skipped: &[(Cell, Cell)]) -> ForgeError {
    use std::fmt::Write as _;

    if failures.len() == 1 && skipped.is_empty() {
        return ForgeError::Eval(failures[0].clone());
    }
    let mut msg = if failures.len() == 1 {
        "1 formula failed:".to_string()
    } else {
        format!("{} formulas failed:", failures.len())
    };
    for failure in failures {
        msg.push_str("\n  - ");
        msg.push_str(failure);
    }
    if !skipped.is_empty() {
        msg.push_str("\n  Not calculated, because they use a failed formula:");
        for (cell, dep) in skipped {
            let _ = write!(msg, "\n  - {} (uses {})", cell.label(), dep.label());
        }
    }
    ForgeError::Eval(msg)
}

/// Say which cells form a dependency cycle, for example
/// "scalars calc.a, calc.b depend on each other".
pub(super) fn describe_cycle(graph: &Graph<Cell>) -> String {
    let mut cells: Vec<&Cell> = graph
        .strongly_connected()
        .into_iter()
        .filter(|component| component.len() > 1 || graph.contains_edge(component[0], component[0]))
        .flatten()
        .map(|node| &graph[node])
        .collect();
    cells.sort();
    cells.dedup();
    let kind = if cells.iter().all(|c| matches!(c, Cell::Scalar(_))) {
        "scalars"
    } else if cells.iter().all(|c| matches!(c, Cell::Column { .. })) {
        "columns"
    } else {
        "formulas"
    };
    let names: Vec<String> = cells.iter().map(|c| c.label()).collect();
    match names.as_slice() {
        [] => "formulas depend on each other".to_string(),
        [one] if matches!(cells[0], Cell::Column { .. }) => format!(
            "{one} refers to itself (to read the row above, write PREV(column, first_row_value))"
        ),
        [one] => format!("{one} refers to itself"),
        _ => format!("{kind} {} depend on each other", names.join(", ")),
    }
}

/// The error for a row formula whose result in `row` is not one number:
/// several values (an array constant or a whole column, unaggregated) or
/// none.
pub(super) fn not_a_number(row: usize, result: &evaluator::Value) -> ForgeError {
    match result {
        evaluator::Value::Array(items) => ForgeError::Eval(format!(
            "row {row}: the result has {} values, and a row formula gives one value a \
             row. Aggregate it with a function such as SUM or INDEX",
            items.len()
        )),
        _ => ForgeError::Eval(format!("row {row}: result is not a number")),
    }
}

#[cfg(test)]
mod tests {
    //! Tests that calculation errors name the column or scalar that failed

    use crate::core::array_calculator::ArrayCalculator;
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
}
