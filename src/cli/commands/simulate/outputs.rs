//! The outputs of a simulation: which ones a run is asked for, a table
//! column as one output per row, and the results a game or the network gives
//! once the model is calculated.

use crate::error::{ForgeError, ForgeResult};

/// The outputs as simulated: a table column (`months.profit`) becomes one
/// output per row (`months.profit (row 1)`, ...), and each output learns
/// from the model whether its highest results are the worst (`better: lower`).
///
/// # Errors
///
/// Returns an error naming an output that is neither a scalar nor a column.
pub(super) fn expand_outputs(
    outputs: &[crate::monte_carlo::config::OutputConfig],
    model: &crate::types::ParsedModel,
) -> ForgeResult<Vec<crate::monte_carlo::config::OutputConfig>> {
    let mut expanded = Vec::new();
    for output in outputs {
        let var = &output.variable;
        let column = var.split_once('.').and_then(|(table, col)| {
            let t = model.tables.get(table)?;
            let meta = t
                .columns
                .get(col)
                .map(|c| c.metadata.clone())
                .or_else(|| t.row_formulas.contains_key(col).then(Default::default))?;
            Some((t.row_count(), meta))
        });
        if let Some((rows, metadata)) = column {
            for row in 0..rows {
                let mut one = output.clone();
                one.variable = format!("{var} (row {})", row + 1);
                one.row = Some((var.clone(), row));
                one.higher_is_worse = metadata.better.as_deref() == Some("lower");
                expanded.push(one);
            }
            continue;
        }
        let scalar = model
            .scalars
            .get(var)
            .or_else(|| model.scalars.get(&format!("outputs.{var}")))
            .or_else(|| model.scalars.get(&format!("scalars.{var}")));
        let Some(scalar) = scalar else {
            // A game's or the network's result is a scalar once calculated
            if model.names_calculated_result(var) {
                expanded.push(output.clone());
                continue;
            }
            return Err(ForgeError::Validation(format!(
                "monte_carlo output '{var}' is neither a scalar nor a table column of the model"
            )));
        };
        let mut one = output.clone();
        one.higher_is_worse = scalar.metadata.better.as_deref() == Some("lower");
        expanded.push(one);
    }
    Ok(expanded)
}

/// The outputs of `outputs` that `names` choose, in the order configured: a
/// name is an output's variable, or its last part when only one output ends
/// with it (`profit` for `year.profit`). No names chooses all of them.
///
/// # Errors
///
/// Returns an error for a name that is not an output, or that ends more than
/// one, listing the outputs.
pub(super) fn select_outputs(
    outputs: &[crate::monte_carlo::config::OutputConfig],
    names: &[String],
) -> ForgeResult<Vec<crate::monte_carlo::config::OutputConfig>> {
    if names.is_empty() {
        return Ok(outputs.to_vec());
    }
    let listed = || {
        outputs
            .iter()
            .map(|o| o.variable.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut chosen = vec![false; outputs.len()];
    for name in names {
        let index = if let Some(i) = outputs.iter().position(|o| &o.variable == name) {
            i
        } else {
            let suffix = format!(".{name}");
            let ends: Vec<usize> = (0..outputs.len())
                .filter(|&i| outputs[i].variable.ends_with(&suffix))
                .collect();
            match ends.as_slice() {
                [i] => *i,
                [] => {
                    return Err(ForgeError::Validation(format!(
                        "--output-var {name}: not an output of monte_carlo (outputs: {})",
                        listed()
                    )))
                },
                _ => {
                    let names: Vec<&str> =
                        ends.iter().map(|&i| outputs[i].variable.as_str()).collect();
                    return Err(ForgeError::Validation(format!(
                        "--output-var {name}: more than one output ends with it ({}); write \
                         the whole name",
                        names.join(", ")
                    )));
                },
            }
        };
        chosen[index] = true;
    }
    Ok(outputs
        .iter()
        .zip(chosen)
        .filter(|(_, keep)| *keep)
        .map(|(o, _)| o.clone())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::super::{simulate_core, SimulateOptions};

    fn write(dir: &std::path::Path, name: &str, text: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    /// A network's posterior is an output, and a prior that reads an MC.*
    /// input moves with it (ADR-074, step 6): in each run, by Bayes' rule,
    /// the partner is a defector after one defection with
    /// p (1 - e) / (p (1 - e) + (1 - p) e), for that run's share p.
    #[test]
    fn a_prior_that_reads_an_uncertain_input_moves_run_by_run() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "m.yaml",
            r#"_forge_version: "5.6.0"
i:
  p:
    formula: "=MC.Uniform(0.01, 0.3)"
  e: 0.05
bayesian_network:
  nodes:
    partner: {states: [cooperator, defector], prior: [=1 - i.p, =i.p]}
    move: {states: [cooperate, defect], parents: [partner], cpt: {cooperator: [=1 - i.e, =i.e], defector: [=i.e, =1 - i.e]}}
  queries:
    one: {node: partner, evidence: {move: defect}}
monte_carlo:
  iterations: 100
  seed: 11
  outputs:
    - variable: bayesian_network.one.defector
"#,
        );
        let result = simulate_core(&path, SimulateOptions::default()).unwrap();
        let posterior = &result.outputs["bayesian_network.one.defector"].samples;
        let share = &result.input_samples["i.p"];
        assert_eq!(posterior.len(), 100);
        let mut seen = std::collections::BTreeSet::new();
        for (got, p) in posterior.iter().zip(share) {
            let by_hand = p * 0.95 / (1.0 - p).mul_add(0.05, p * 0.95);
            assert!((got - by_hand).abs() < 1e-12, "{got} at {p}");
            seen.insert(got.to_bits());
        }
        let moved = seen.len();
        assert!(moved > 90, "the posterior moves: {moved}");
    }

    /// A column of data and a column of row formulas are each one output
    /// per row: 2 and 3 times a run's draw of u
    #[test]
    fn a_column_is_an_output_for_each_row() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "m.yaml",
            "_forge_version: \"5.1.0\"\ni:\n  u:\n    formula: \"=MC.Uniform(0, 1)\"\nt:\n  x: [2, 3]\n  y: \"=x * i.u\"\nmonte_carlo:\n  iterations: 100\n  seed: 3\n  outputs:\n    - variable: t.x\n    - variable: t.y\n",
        );
        let result = simulate_core(&path, SimulateOptions::default()).unwrap();
        let mut names: Vec<&String> = result.outputs.keys().collect();
        names.sort();
        assert_eq!(
            names,
            ["t.x (row 1)", "t.x (row 2)", "t.y (row 1)", "t.y (row 2)"]
        );
        let draws = &result.input_samples["i.u"];
        for (y, u) in result.outputs["t.y (row 2)"].samples.iter().zip(draws) {
            assert!((y - 3.0 * u).abs() < 1e-12, "{y} at {u}");
        }
    }
}
