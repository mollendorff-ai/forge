//! How `forge simulate` prints its results, and writes them to a file.

use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use colored::Colorize;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// Entries of a map in name order, so the printed results do not change between runs
fn sorted_by_name<V>(map: &std::collections::HashMap<String, V>) -> Vec<(&String, &V)> {
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    entries
}

/// Write the simulation results, as `forge simulate` prints them
pub(super) fn print_simulation_results(
    c: &mut Console<'_>,
    result: &crate::monte_carlo::SimulationResult,
) {
    outln!(c, "{}", "📊 Simulation Results:".bold().green());
    outln!(c, "   Iterations:     {}", result.iterations_completed);
    outln!(c, "   Execution time: {} ms", result.execution_time_ms);
    outln!(c);

    print_input_distributions(c, result);

    // Show output results
    if !result.outputs.is_empty() {
        outln!(c, "   {}", "Output Statistics:".bold());
        let rows_of = |name: &str| {
            result
                .config
                .outputs
                .iter()
                .find(|o| o.variable == name)
                .and_then(|o| o.row.clone())
        };
        let mut printed_columns: Vec<String> = Vec::new();
        for (var_name, output) in &result.outputs {
            if let Some((column, _)) = rows_of(var_name) {
                if !printed_columns.contains(&column) {
                    print_column_rows(c, result, &column);
                    printed_columns.push(column);
                }
                continue;
            }
            print_output(c, var_name, output);
        }
    }
}

/// The sampled inputs: each one's mean, plan value, min and max
fn print_input_distributions(c: &mut Console<'_>, result: &crate::monte_carlo::SimulationResult) {
    outln!(c, "   {}", "Input Distributions:".bold());
    for (var_name, samples) in sorted_by_name(&result.input_samples) {
        let mean = crate::monte_carlo::statistics::mean(samples);
        let min = samples.iter().copied().fold(f64::INFINITY, f64::min);
        let max = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let plan = result
            .plan_inputs
            .get(var_name)
            .map_or_else(String::new, |p| {
                format!(
                    " plan={} ({})",
                    super::super::display_number(p.value),
                    p.from
                )
            });
        outln!(
            c,
            "      {} mean={}{plan} min={} max={}",
            var_name.bright_blue(),
            super::super::display_number(mean),
            super::super::display_number(min),
            super::super::display_number(max)
        );
    }
    outln!(c);
}

/// One output's statistics, thresholds, expected shortfall and drivers
fn print_output(c: &mut Console<'_>, var_name: &str, output: &crate::monte_carlo::OutputResult) {
    let stats = &output.statistics;
    outln!(c, "      {}:", var_name.bright_blue().bold());
    outln!(
        c,
        "         Mean:      {}",
        super::super::display_number(stats.mean)
    );
    if let Some(at_plan) = output.at_plan {
        outln!(
            c,
            "         At plan:   {}  (each uncertain input at its plan value)",
            super::super::display_number(at_plan)
        );
        if let Some(note) = mean_versus_plan(stats.mean, at_plan, stats.std_dev) {
            outln!(c, "         {note}");
        }
    }
    outln!(
        c,
        "         Median:    {}",
        super::super::display_number(stats.median)
    );
    outln!(
        c,
        "         Std Dev:   {}",
        super::super::display_number(stats.std_dev)
    );
    outln!(
        c,
        "         Min:       {}",
        super::super::display_number(stats.min)
    );
    outln!(
        c,
        "         Max:       {}",
        super::super::display_number(stats.max)
    );

    // Percentiles
    outln!(c, "         Percentiles:");
    for (p, v) in &stats.percentiles {
        outln!(c, "            P{p}: {}", super::super::display_number(*v));
    }

    // Thresholds, in the order configured
    for (threshold, prob) in &output.threshold_probabilities {
        outln!(
            c,
            "         P({} {}) = {:.2}%",
            var_name,
            threshold,
            prob * 100.0
        );
    }

    for (p, mean) in &output.expected_shortfall {
        outln!(
            c,
            "         Mean of the worst {p}% (expected shortfall): {}",
            super::super::display_number(*mean)
        );
    }

    // Which sampled inputs move it most
    if !output.drivers.is_empty() {
        outln!(c, "         Drivers (rank correlation with the output):");
        for driver in &output.drivers {
            outln!(
                c,
                "            {:+.3}  {}",
                driver.rank_correlation,
                driver.input
            );
        }
    }
    outln!(c);
}

/// A note when an output's mean is far from its value at plan (more than a
/// tenth of a standard deviation), which a reader would otherwise take for a
/// contradiction between `forge calculate` and `forge simulate`.
fn mean_versus_plan(mean: f64, at_plan: f64, std_dev: f64) -> Option<String> {
    let gap = mean - at_plan;
    // Not past a tenth of a standard deviation, or not comparable (NaN)
    if gap.abs().partial_cmp(&(0.1 * std_dev)) != Some(std::cmp::Ordering::Greater) {
        return None;
    }
    Some(format!(
        "The mean is {} {} the value at plan: the uncertain inputs' means are not their plan \
         values (see Input Distributions), or the model is not linear in them.",
        super::super::display_number(gap.abs()),
        if gap < 0.0 { "below" } else { "above" }
    ))
}

/// A table column output, one line per row: mean, P10, P50, P90, and each
/// threshold's probability.
fn print_column_rows(
    c: &mut Console<'_>,
    result: &crate::monte_carlo::SimulationResult,
    column: &str,
) {
    let rows: Vec<&crate::monte_carlo::OutputResult> = result
        .config
        .outputs
        .iter()
        .filter(|o| o.row.as_ref().is_some_and(|(c, _)| c == column))
        .filter_map(|o| result.outputs.get(&o.variable))
        .collect();
    outln!(
        c,
        "      {} {}:",
        column.bright_blue().bold(),
        "by row".bold()
    );
    let mut header = format!(
        "         {:>4} {:>14} {:>14} {:>14} {:>14} {:>14}",
        "row", "mean", "at plan", "P10", "P50", "P90"
    );
    // Every row has the thresholds of its column; the first one names them
    for threshold in rows
        .iter()
        .take(1)
        .flat_map(|r| r.threshold_probabilities.keys())
    {
        let _ = write!(header, " {:>10}", format!("P({threshold})"));
    }
    outln!(c, "{header}");
    for (row, output) in rows.iter().enumerate() {
        let p = |q: u8| {
            output
                .statistics
                .percentiles
                .get(&q)
                .copied()
                .unwrap_or(f64::NAN)
        };
        let mut line = format!(
            "         {:>4} {:>14} {:>14} {:>14} {:>14} {:>14}",
            row + 1,
            super::super::display_number(output.statistics.mean),
            output
                .at_plan
                .map_or_else(|| "-".to_string(), super::super::display_number),
            super::super::display_number(p(10)),
            super::super::display_number(p(50)),
            super::super::display_number(p(90))
        );
        for prob in output.threshold_probabilities.values() {
            let _ = write!(line, " {:>9.2}%", prob * 100.0);
        }
        outln!(c, "{line}");
    }
    outln!(c);
}

/// Write simulation output to a file (YAML, JSON, or Excel)
pub(super) fn write_simulation_output(
    c: &mut Console<'_>,
    result: &crate::monte_carlo::SimulationResult,
    output_path: &Path,
) -> ForgeResult<()> {
    let ext = output_path.extension().and_then(|e| e.to_str());
    match ext {
        Some("xlsx") => {
            #[cfg(feature = "excel")]
            crate::monte_carlo::excel_export::export_results(result, output_path)
                .map_err(ForgeError::Validation)?;
            #[cfg(not(feature = "excel"))]
            return Err(ForgeError::Validation(
                "Writing Excel needs Forge's `excel` feature, which this build does not have"
                    .to_string(),
            ));
        },
        Some("json") => {
            fs::write(output_path, result.to_json()).map_err(ForgeError::Io)?;
        },
        _ => {
            let output_str = result.to_yaml();
            fs::write(output_path, output_str).map_err(ForgeError::Io)?;
        },
    }

    outln!(
        c,
        "{}",
        format!("💾 Results written to {}", output_path.display())
            .bold()
            .green()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T1-SIM-MEAN-VS-PLAN: the note appears past a tenth of a standard
    /// deviation, and says which way the mean lies.
    #[test]
    fn the_mean_versus_plan_note() {
        assert_eq!(mean_versus_plan(100.0, 100.5, 10.0), None);
        assert_eq!(mean_versus_plan(5.0, 5.0, 0.0), None);
        assert_eq!(mean_versus_plan(f64::NAN, 5.0, 1.0), None);
        let below = mean_versus_plan(-15_925.94, 786.32, 38_058.05).unwrap();
        assert!(
            below.starts_with("The mean is 16,712.26 below the value at plan"),
            "{below}"
        );
        let above = mean_versus_plan(12.0, 10.0, 1.0).unwrap();
        assert!(
            above.starts_with("The mean is 2.00 above the value at plan"),
            "{above}"
        );
    }

    const ROWS: &str = r#"_forge_version: "5.2.0"
t:
  n: [1, 2]
  shock: "=MC.Normal(0, 0.1)"
  level: "=10 + shock"
s:
  total: {formula: "=SUM(t.shock)"}
monte_carlo:
  iterations: 200
  seed: 5
  outputs:
    - variable: s.total
    - variable: t.level
      thresholds: ["> 10"]
"#;

    fn simulated(model: &str) -> crate::monte_carlo::SimulationResult {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.yaml");
        fs::write(&path, model).unwrap();
        crate::cli::simulate_core(&path, crate::cli::SimulateOptions::default()).unwrap()
    }

    /// A column of rows is printed once, by row, with each threshold's
    /// probability; a lone output is printed on its own.
    #[test]
    fn a_column_is_printed_by_row_with_its_thresholds() {
        let result = simulated(ROWS);
        let ((), printed) = crate::cli::console::capture(|c| print_simulation_results(c, &result));
        let out = printed.out;
        assert_eq!(out.matches("by row:").count(), 1, "{out}");
        assert!(out.contains("t.level by row:"), "{out}");
        assert!(out.contains("P(> 10)"), "{out}");
        assert!(out.contains("Iterations:     200"), "{out}");
        assert!(out.contains("Output Statistics:"), "{out}");
        assert!(out.contains("s.total"), "{out}");
        // Two rows, numbered from 1
        let rows: Vec<&str> = out
            .lines()
            .filter(|l| l.trim_start().starts_with(['1', '2']) && l.contains('%'))
            .collect();
        assert_eq!(rows.len(), 2, "{out}");
        assert!(rows[0].trim_start().starts_with("1 "), "{out}");
    }

    #[test]
    fn results_are_written_as_yaml_json_and_excel_and_each_failure_is_an_error() {
        let result = simulated(ROWS);
        let dir = tempfile::tempdir().unwrap();
        for name in ["r.yaml", "r.json", "r.xlsx", "r.other"] {
            let path = dir.path().join(name);
            let (done, printed) =
                crate::cli::console::capture(|c| write_simulation_output(c, &result, &path));
            assert!(done.is_ok(), "{name}");
            assert!(
                printed
                    .out
                    .contains(&format!("Results written to {}", path.display())),
                "{}",
                printed.out
            );
            assert!(fs::metadata(&path).unwrap().len() > 0, "{name}");
        }
        let json = fs::read_to_string(dir.path().join("r.json")).unwrap();
        assert!(json.contains("\"iterations\": 200"), "{json}");
        let yaml = fs::read_to_string(dir.path().join("r.yaml")).unwrap();
        assert!(yaml.contains("iterations: 200"), "{yaml}");
        // A folder that does not exist cannot be written to
        let missing = dir.path().join("none").join("r.json");
        let (done, _) =
            crate::cli::console::capture(|c| write_simulation_output(c, &result, &missing));
        assert!(done.unwrap_err().to_string().starts_with("IO error"));
        // The same for the YAML, and for a workbook that cannot be made
        for name in ["none/r.yaml", "none/r.xlsx"] {
            let path = dir.path().join(name);
            let (done, printed) =
                crate::cli::console::capture(|c| write_simulation_output(c, &result, &path));
            assert!(done.is_err(), "{name}");
            assert_eq!(printed.out, "", "{name}");
        }
    }

    /// An output with no value at plan, no drivers and a threshold-less row
    /// still prints, with a dash where the plan would be; a run with no
    /// outputs prints the inputs only.
    #[test]
    fn what_a_result_lacks_is_left_out_of_what_is_printed() {
        let mut result = simulated(ROWS);
        for output in result.outputs.values_mut() {
            output.at_plan = None;
            output.drivers.clear();
        }
        let ((), printed) = crate::cli::console::capture(|c| print_simulation_results(c, &result));
        let out = printed.out;
        assert!(!out.contains("At plan:"), "{out}");
        assert!(!out.contains("Drivers"), "{out}");
        let by_row: Vec<&str> = out
            .lines()
            .skip_while(|l| !l.contains("t.level by row:"))
            .filter(|l| l.trim_start().starts_with(['1', '2']))
            .collect();
        assert_eq!(by_row.len(), 2, "{out}");
        assert!(
            by_row
                .iter()
                .all(|l| l.split_whitespace().nth(2) == Some("-")),
            "{out}"
        );

        result.outputs.clear();
        let ((), printed) = crate::cli::console::capture(|c| print_simulation_results(c, &result));
        assert!(
            !printed.out.contains("Output Statistics:"),
            "{}",
            printed.out
        );
        assert!(
            printed.out.contains("Input Distributions:"),
            "{}",
            printed.out
        );
    }
}
