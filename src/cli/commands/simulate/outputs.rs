//! The outputs of a simulation: which ones a run is asked for, and a table
//! column as one output per row.

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
            // A game's result is a scalar once calculated (ADR-068)
            if model.names_game_result(var) {
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
