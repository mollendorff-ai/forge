//! A NaN in the data (`.nan` in YAML) must give an answer or an error, never a panic.

use super::ArrayCalculator;
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[test]
fn test_nan_in_a_column_never_panics() {
    let functions = [
        "MEDIAN",
        "MIN",
        "MAX",
        "SMALL",
        "LARGE",
        "PERCENTILE",
        "QUARTILE",
        "RANK",
        "SORT",
        "MODE",
        "STDEV",
        "VAR",
    ];
    for name in functions {
        let mut model = ParsedModel::new();
        let mut table = Table::new("t".to_string());
        table.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![3.0, f64::NAN, 2.0, 1.0]),
        ));
        model.add_table(table);
        let formula = match name {
            "SMALL" | "LARGE" | "PERCENTILE" | "QUARTILE" => format!("={name}(t.x, 1)"),
            "RANK" => "=RANK(2, t.x)".to_string(),
            "SORT" => "=SUM(SORT(t.x))".to_string(),
            _ => format!("={name}(t.x)"),
        };
        model.add_scalar(
            "out.v".to_string(),
            Variable::new("out.v".to_string(), None, Some(formula)),
        );
        // Ok or Err are both fine; a panic fails the test.
        let _ = ArrayCalculator::new(model).calculate_all();
    }
}
