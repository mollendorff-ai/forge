//! One iteration of a simulation: the model calculated with the draws.

use crate::core::array_calculator::ArrayCalculator;
use std::collections::HashMap;

/// Calculate the model once with the sampled `inputs` and read the outputs.
///
/// An input named `@ns.path` is a distribution in the included file `ns`.
///
/// # Errors
///
/// Returns the calculation error, or an error naming an output that has no
/// calculated value. Callers must not turn either into a 0 sample.
pub(super) fn evaluate_iteration(
    model: &crate::types::ParsedModel,
    plan: &std::rc::Rc<crate::core::array_calculator::CalcPlan>,
    output_vars: &[(String, Option<(String, usize)>)],
    inputs: &HashMap<String, f64>,
    only_outputs: bool,
) -> Result<HashMap<String, f64>, String> {
    let mut iter_model = model.clone();
    // Replace MC.* distribution formulas with the sampled values: a scalar's,
    // or one row of an uncertain column's
    for (var_name, &value) in inputs {
        iter_model.set_input(var_name, value)?;
    }

    // Every iteration has the same formulas, so they are parsed and ordered once
    let calculator = ArrayCalculator::with_plan(iter_model, std::rc::Rc::clone(plan));
    let calculated = if only_outputs {
        // A search over many simulations: a formula no output uses may fail
        // at an input's end of the range without failing the answer
        let (columns, scalars): (Vec<_>, Vec<_>) =
            output_vars.iter().partition(|(_, row)| row.is_some());
        let scalars: Vec<String> = scalars
            .iter()
            .flat_map(|(name, _)| {
                [
                    name.clone(),
                    format!("outputs.{name}"),
                    format!("scalars.{name}"),
                ]
            })
            .collect();
        let columns: Vec<(String, String)> = columns
            .iter()
            .filter_map(|(_, row)| row.as_ref())
            .filter_map(|(column, _)| column.split_once('.'))
            .map(|(table, column)| (table.to_string(), column.to_string()))
            .collect();
        calculator.calculate_outputs(&scalars, &columns)
    } else {
        calculator.calculate_all()
    }
    .map_err(|e| e.to_string())?;

    output_vars
        .iter()
        .map(|(var_name, row)| {
            if let Some((column, row)) = row {
                let (table, col) = column.split_once('.').unwrap_or_default();
                return calculated
                    .tables
                    .get(table)
                    .and_then(|t| t.columns.get(col))
                    // An empty cell of a data column is 0, as a formula reads it
                    .and_then(|c| c.values.numbers().and_then(|v| v.get(*row).copied()))
                    .map(|value| (var_name.clone(), value))
                    .ok_or_else(|| format!("output '{var_name}' has no calculated number"));
            }
            calculated
                .scalars
                .get(var_name)
                .or_else(|| calculated.scalars.get(&format!("outputs.{var_name}")))
                .or_else(|| calculated.scalars.get(&format!("scalars.{var_name}")))
                .and_then(|s| s.value)
                .map(|value| (var_name.clone(), value))
                .ok_or_else(|| format!("output '{var_name}' has no calculated value"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::evaluate_iteration;
    use crate::core::array_calculator::CalcPlan;
    use crate::parser;
    use std::collections::HashMap;

    const MODEL: &str = "_forge_version: \"5.4.0\"\ns:\n  x: 2\n  out: {formula: \"=x * 3\"}\nt:\n  n: [1, 2]\n  label: [\"a\", \"b\"]\n";

    fn run(
        outputs: &[(&str, Option<(&str, usize)>)],
        inputs: &[(&str, f64)],
        only_outputs: bool,
    ) -> Result<HashMap<String, f64>, String> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.yaml");
        std::fs::write(&path, MODEL).unwrap();
        let model = parser::parse_model(&path).unwrap();
        let outputs: Vec<(String, Option<(String, usize)>)> = outputs
            .iter()
            .map(|(name, row)| (name.to_string(), row.map(|(c, r)| (c.to_string(), r))))
            .collect();
        let inputs = inputs.iter().map(|(n, v)| (n.to_string(), *v)).collect();
        evaluate_iteration(&model, &CalcPlan::new(), &outputs, &inputs, only_outputs)
    }

    #[test]
    fn the_outputs_are_read_with_the_draws_set() {
        let out = run(&[("s.out", None)], &[("s.x", 5.0)], false).unwrap();
        assert_eq!(out, HashMap::from([("s.out".to_string(), 15.0)]));
        // Only the outputs: the same, and a row of a column of data
        let out = run(&[("s.out", None)], &[("s.x", 5.0)], true).unwrap();
        assert_eq!(out, HashMap::from([("s.out".to_string(), 15.0)]));
        let row = run(&[("t.n (row 2)", Some(("t.n", 1)))], &[], true).unwrap();
        assert_eq!(row, HashMap::from([("t.n (row 2)".to_string(), 2.0)]));
    }

    #[test]
    fn what_has_no_value_or_no_input_is_an_error_not_a_zero() {
        assert_eq!(
            run(&[("s.out", None)], &[("s.nope", 5.0)], false).unwrap_err(),
            "Scenario override 's.nope' does not name a scalar in the model."
        );
        for only_outputs in [false, true] {
            assert_eq!(
                run(&[("s.zzz", None)], &[], only_outputs).unwrap_err(),
                "output 's.zzz' has no calculated value"
            );
        }
        // A column of text, and a column the table does not have
        for (name, column) in [("t.label (row 1)", "t.label"), ("t.zzz (row 1)", "t.zzz")] {
            assert_eq!(
                run(&[(name, Some((column, 0)))], &[], false).unwrap_err(),
                format!("output '{name}' has no calculated number")
            );
        }
    }
}
