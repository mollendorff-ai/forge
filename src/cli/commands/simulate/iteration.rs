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

    mod question_models {
        //! The two simulated models of `examples/questions/` that a person
        //! reads for themselves: every iteration worked out again here from
        //! Forge's own draws, and the chances against their exact values.
        //!
        //! The Python that gives those (independent of Forge):
        //! ```python
        //! from scipy.stats import norm
        //! # job_offer.yaml
        //! def options(exit, dilution):
        //!     price = max(0, exit - 30e6) * (1 - dilution) / 10e6
        //!     return 25000 * max(0, price - 2.00)
        //! options(300e6, 0.35)                      # 388750.0, the plan
        //! 0.12 * options(300e6, 0.35) + 0.03 * options(1500e6, 0.35)
        //!                                           # 116812.5, the mean: the value
        //!                                           # is linear in the dilution, whose
        //!                                           # mean is (0.20 + 0.35 + 0.50) / 3
        //! 0.12 + 0.03                               # 0.15, the chance of any value
        //! # mortgage_or_tfsa.yaml
        //! def fv(yearly, per_year):
        //!     m = (1 + yearly) ** (1 / per_year) - 1
        //!     return 300 * ((1 + m) ** 120 - 1) / m
        //! lower = fv((1 + 0.0479 / 2) ** 2 - 1, 12) # 45949.4363825837
        //! fv(0.06, 12) - lower                      # 2792.59634576786, the plan
        //! 1 - norm.cdf(((1 + 0.0479 / 2) ** 2 - 1 - 0.06) / 0.05)
        //!                                           # 0.5911592263, the chance the
        //!                                           # fund beats the mortgage
        //! ```

        use crate::cli::{calculate_plan_core, simulate_core, SimulateOptions};
        use std::path::Path;

        fn close(a: f64, b: f64, tol: f64) -> bool {
            (a - b).abs() <= tol
        }

        #[test]
        fn the_job_offers_options_are_worth_nothing_in_85_futures_of_100() {
            let file = Path::new("examples/questions/job_offer.yaml");
            let plan = calculate_plan_core(file, None).unwrap();
            assert!(close(
                plan.scalars["result.options_value"].unwrap(),
                388_750.0,
                1e-6
            ));
            assert!(close(
                plan.scalars["result.options_ahead"].unwrap(),
                288_750.0,
                1e-6
            ));

            let result = simulate_core(file, SimulateOptions::default()).unwrap();
            let value = &result.outputs["result.options_value"];
            let ahead = &result.outputs["result.options_ahead"];
            let exits = &result.input_samples["company.exit_value"];
            let dilutions = &result.input_samples["company.dilution"];
            assert_eq!(value.samples.len(), 10_000);
            for (i, (exit, dilution)) in exits.iter().zip(dilutions).enumerate() {
                let price = (exit - 30e6).max(0.0) * (1.0 - dilution) / 10e6;
                let options = 25_000.0 * (price - 2.0).max(0.0);
                assert!(close(options, value.samples[i], 1e-6), "iteration {i}");
                assert!(
                    close(options - 100_000.0, ahead.samples[i], 1e-6),
                    "iteration {i}"
                );
            }
            // Six exits in ten are nothing and a quarter are too small: 85%
            assert!(close(value.threshold_probabilities["> 0"], 0.15, 1e-12));
            assert!(close(ahead.threshold_probabilities["> 0"], 0.15, 1e-12));
            assert!(close(value.statistics.median, 0.0, 0.0));
            // The mean, 116,812.50 exactly, is under a third of the plan
            assert!(
                close(value.statistics.mean, 116_812.5, 500.0),
                "{}",
                value.statistics.mean
            );
        }

        #[test]
        fn the_tfsa_beats_the_mortgage_in_59_futures_of_100() {
            let file = Path::new("examples/questions/mortgage_or_tfsa.yaml");
            let plan = calculate_plan_core(file, None).unwrap();
            for (name, expected) in [
                ("mortgage.yearly_cost", 0.048_473_602_5),
                ("after.mortgage_lower_by", 45_949.436_382_583_7),
                ("after.tfsa_worth", 48_742.032_728_351_5),
                ("after.tfsa_ahead", 2_792.596_345_767_86),
            ] {
                assert!(close(plan.scalars[name].unwrap(), expected, 1e-6), "{name}");
            }

            let result = simulate_core(file, SimulateOptions::default()).unwrap();
            let ahead = &result.outputs["after.tfsa_ahead"];
            let returns = &result.input_samples["tfsa.yearly_return"];
            let future = |monthly: f64| 300.0 * ((1.0 + monthly).powi(120) - 1.0) / monthly;
            let lower = future((1.0_f64 + 0.0479 / 2.0).powf(1.0 / 6.0) - 1.0);
            assert_eq!(ahead.samples.len(), 10_000);
            for (i, yearly) in returns.iter().enumerate() {
                let worth = future((1.0 + yearly).powf(1.0 / 12.0) - 1.0);
                assert!(
                    close(worth - lower, ahead.samples[i], 1e-6),
                    "iteration {i}"
                );
            }
            let chance = ahead.threshold_probabilities["> 0"];
            assert!(close(chance, 0.591_159_226_3, 0.002), "{chance}");
            // A tenth of the futures end more than 10,000 behind the mortgage
            let mut sorted = ahead.samples.clone();
            sorted.sort_by(f64::total_cmp);
            assert!(sorted[999] < -10_000.0 && sorted[1_000] > -11_000.0);
        }
    }
}
