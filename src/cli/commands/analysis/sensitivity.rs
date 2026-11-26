//! `forge sensitivity`: an output for each value of one input, or a table
//! for two.

use crate::cli::console::{out, outln};
use crate::cli::Console;
use crate::core::precision;
use crate::core::ArrayCalculator;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use colored::Colorize;
use num_traits::ToPrimitive;
use std::path::Path;

use crate::cli::commands::display_number;
use crate::cli::commands::results::SensitivityData;

/// Parse a range string "start,end,step" into a vector of values
///
/// # Errors
///
/// Returns an error if the range format is invalid, values cannot be parsed,
/// step is non-positive, or start exceeds end.
pub fn parse_range(range: &str) -> ForgeResult<Vec<f64>> {
    let parts: Vec<&str> = range.split(',').collect();
    if parts.len() != 3 {
        return Err(ForgeError::Validation(format!(
            "Invalid range format '{range}'. Expected: start,end,step (e.g., 0.01,0.15,0.02)"
        )));
    }

    let start: f64 = parts[0]
        .trim()
        .parse()
        .map_err(|_| ForgeError::Validation(format!("Invalid start value: '{}'", parts[0])))?;
    let end: f64 = parts[1]
        .trim()
        .parse()
        .map_err(|_| ForgeError::Validation(format!("Invalid end value: '{}'", parts[1])))?;
    let step: f64 = parts[2]
        .trim()
        .parse()
        .map_err(|_| ForgeError::Validation(format!("Invalid step value: '{}'", parts[2])))?;

    if step <= 0.0 {
        return Err(ForgeError::Validation("Step must be positive".to_string()));
    }
    if start > end {
        return Err(ForgeError::Validation(
            "Start must be less than or equal to end".to_string(),
        ));
    }

    // Use integer step count to avoid while-float comparison
    let num_steps = ((end - start) / step + 1.001)
        .floor()
        .to_usize()
        .ok_or_else(|| {
            ForgeError::Validation(format!("The range {range} has too many steps to calculate"))
        })?;
    let values: Vec<f64> = (0..num_steps)
        .map(|i| step.mul_add(precision::count(i), start))
        .collect();

    Ok(values)
}

/// Calculate model with a specific variable override and return the output value
///
/// # Errors
///
/// Returns an error if calculation fails or the output variable is not found.
pub fn calculate_with_override(
    base_model: &crate::types::ParsedModel,
    var_name: &str,
    var_value: f64,
    output_name: &str,
) -> ForgeResult<f64> {
    base_model
        .refuse_date_input(var_name, "this command")
        .map_err(ForgeError::Validation)?;
    let mut model = base_model.clone();
    // The model's scalar, or an included file's (`@costs.flour`)
    model
        .set_input(var_name, var_value)
        .map_err(ForgeError::Validation)?;

    // Calculate for the output alone: a formula it does not use may fail at
    // this value of the input without failing the answer
    ArrayCalculator::new(model).calculate_scalar(output_name)
}

/// The model and the full names of the inputs and the output, each found as
/// [`scalar_name`](crate::cli::commands::names::scalar_name) finds it, with a
/// date input refused (ADR-060).
struct Resolved {
    model: crate::types::ParsedModel,
    vary: String,
    vary2: Option<String>,
    output: String,
}

fn resolve(file: &Path, vary: &str, vary2: Option<&str>, output: &str) -> ForgeResult<Resolved> {
    use crate::cli::commands::names::scalar_name;
    let model = parser::parse_model(file)?;
    crate::cli::commands::reject_distributions_in(&model, "sensitivity")?;
    let vary = scalar_name(&model, vary, false)?;
    let vary2 = vary2.map(|v| scalar_name(&model, v, false)).transpose()?;
    let output = scalar_name(&model, output, true)?;
    for name in std::iter::once(&vary).chain(&vary2) {
        model
            .refuse_date_input(name, "sensitivity")
            .map_err(ForgeError::Validation)?;
    }
    Ok(Resolved {
        model,
        vary,
        vary2,
        output,
    })
}

/// The output with both inputs set, or `None` when it has no value there: a
/// formula the output does not use may fail without failing the cell.
fn two_input_cell(
    base_model: &crate::types::ParsedModel,
    (vary, val1): (&str, f64),
    (vary2, val2): (&str, f64),
    output: &str,
) -> Option<f64> {
    let mut model = base_model.clone();
    model
        .set_input(vary, val1)
        .and_then(|()| model.set_input(vary2, val2))
        .ok()
        .and_then(|()| ArrayCalculator::new(model).calculate_scalar(output).ok())
}

/// Run sensitivity analysis and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, variables are not found,
/// ranges are invalid, or calculation fails.
pub fn sensitivity_core(
    file: &Path,
    vary: &str,
    range: &str,
    vary2: Option<&str>,
    range2: Option<&str>,
    output: &str,
) -> ForgeResult<crate::cli::commands::results::SensitivityResult> {
    use crate::cli::commands::results::{SensitivityData, SensitivityEntry, SensitivityResult};
    let Resolved {
        model,
        vary,
        vary2,
        output,
    } = resolve(file, vary, vary2, output)?;
    let values1 = parse_range(range)?;

    let data = if let (Some(v2), Some(r2)) = (vary2, range2) {
        let values2 = parse_range(r2)?;
        let matrix = values1
            .iter()
            .map(|val1| {
                values2
                    .iter()
                    .map(|val2| two_input_cell(&model, (&vary, *val1), (&v2, *val2), &output))
                    .collect()
            })
            .collect();
        SensitivityData::TwoVar {
            vary2: v2,
            row_values: values1,
            col_values: values2,
            matrix,
        }
    } else {
        let entries = values1
            .iter()
            .map(
                |val| match calculate_with_override(&model, &vary, *val, &output) {
                    Ok(result) => SensitivityEntry {
                        input: *val,
                        output: Some(result),
                        error: None,
                    },
                    Err(e) => SensitivityEntry {
                        input: *val,
                        output: None,
                        error: Some(e.to_string()),
                    },
                },
            )
            .collect();
        SensitivityData::OneVar { entries }
    };

    Ok(SensitivityResult { vary, output, data })
}

/// Execute the sensitivity command
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, variables are not found,
/// ranges are invalid, or calculation fails.
pub fn sensitivity(
    c: &mut Console<'_>,
    file: &Path,
    (vary, range): (&str, &str),
    (vary2, range2): (Option<&str>, Option<&str>),
    output: &str,
    verbose: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "🔥 Forge - Sensitivity Analysis".bold().green());
    outln!(c, "   File: {}", file.display());
    outln!(c, "   Vary: {} ({})", vary.bright_yellow(), range);
    if let Some(v2) = vary2 {
        outln!(
            c,
            "   Vary2: {} ({})",
            v2.bright_yellow(),
            range2.unwrap_or("?")
        );
    }
    outln!(c, "   Output: {}\n", output.bright_blue());

    let result = sensitivity_core(file, vary, range, vary2, range2, output)?;
    let (vary, output) = (result.vary.as_str(), result.output.as_str());
    match &result.data {
        SensitivityData::OneVar { entries } => {
            if verbose {
                let inputs: Vec<f64> = entries.iter().map(|e| e.input).collect();
                print_range(c, 1, &inputs);
            }
            print_one_var_sensitivity(c, vary, output, entries);
        },
        SensitivityData::TwoVar {
            vary2,
            row_values,
            col_values,
            matrix,
        } => {
            if verbose {
                print_range(c, 1, row_values);
                print_range(c, 2, col_values);
            }
            print_two_var_sensitivity(c, (vary, vary2), output, (row_values, col_values), matrix);
        },
    }

    outln!(c, "\n{}", "✅ Sensitivity analysis complete".bold().green());
    Ok(())
}

/// The table of a one-variable sensitivity: each input value and the output
/// it gives, in columns wide enough for the names and the numbers. A value
/// with no output shows `ERR`, as in the two-input matrix, and its reason is
/// under the table, where its lines do not stretch the column.
fn print_one_var_sensitivity(
    c: &mut Console<'_>,
    vary: &str,
    output: &str,
    entries: &[crate::cli::commands::results::SensitivityEntry],
) {
    outln!(
        c,
        "\n{} {} → {}",
        "📊 Sensitivity Table:".bold().cyan(),
        vary.yellow(),
        output.bright_blue()
    );
    // Wide enough for the names and the numbers, so the header lines up
    let rows: Vec<(String, Result<f64, String>)> = entries
        .iter()
        .map(|entry| {
            let result = entry
                .output
                .ok_or_else(|| entry.error.clone().unwrap_or_default());
            (format!("{:.4}", entry.input), result)
        })
        .collect();
    let shown = |result: &Result<f64, String>| {
        result
            .as_ref()
            .map_or_else(|_| "ERR".to_string(), |v| display_number(*v))
    };
    let input_width = rows
        .iter()
        .map(|(input, _)| input.len())
        .chain([vary.chars().count()])
        .max()
        .unwrap_or(12);
    let output_width = rows
        .iter()
        .map(|(_, result)| shown(result).chars().count())
        .chain([output.chars().count()])
        .max()
        .unwrap_or(15);
    let rule = "─".repeat(input_width + 1 + output_width);
    outln!(c, "{rule}");
    outln!(
        c,
        "{:>input_width$} {:>output_width$}",
        vary.bold(),
        output.bold()
    );
    outln!(c, "{rule}");
    for (input, result) in &rows {
        let text = shown(result);
        outln!(
            c,
            "{:>input_width$} {:>output_width$}",
            input.bright_yellow(),
            if result.is_ok() {
                text.green()
            } else {
                text.red()
            }
        );
    }
    outln!(c, "{rule}");
    for (input, result) in &rows {
        if let Err(e) = result {
            outln!(c, "{} {vary} = {input}: {e}", "ERR".red());
        }
    }
}

/// `forge sensitivity` in the format asked for: the table, or JSON for
/// programs that read it, with the numbers at their 15 significant digits
/// and not as the table shows them.
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, variables are not found,
/// ranges are invalid, or calculation fails.
pub fn sensitivity_in_format(
    c: &mut Console<'_>,
    file: &Path,
    (vary, range): (&str, &str),
    (vary2, range2): (Option<&str>, Option<&str>),
    output: &str,
    (format, verbose): (&str, bool),
) -> ForgeResult<()> {
    if format != "json" {
        return sensitivity(c, file, (vary, range), (vary2, range2), output, verbose);
    }
    let result = sensitivity_core(file, vary, range, vary2, range2, output)?;
    // Names, numbers and lists of them: there is nothing JSON cannot write
    let json =
        serde_json::to_string_pretty(&crate::core::precision::Rounded(&result)).unwrap_or_default();
    outln!(c, "{json}");
    Ok(())
}

/// How many values a range has, and its first and last.
fn print_range(c: &mut Console<'_>, which: u8, values: &[f64]) {
    outln!(
        c,
        "   Range {which}: {} values from {} to {}",
        values.len(),
        values.first().unwrap_or(&0.0),
        values.last().unwrap_or(&0.0)
    );
}

/// The matrix of a two-input sensitivity, each cell the output or `ERR`.
fn print_two_var_sensitivity(
    c: &mut Console<'_>,
    (vary, v2): (&str, &str),
    output: &str,
    (values1, values2): (&[f64], &[f64]),
    matrix: &[Vec<Option<f64>>],
) {
    // Calculate matrix
    outln!(
        c,
        "\n{} {} → {}",
        "📊 Sensitivity Matrix:".bold().cyan(),
        format!("({vary}, {v2})").yellow(),
        output.bright_blue()
    );

    // Every cell first, so the columns can be as wide as their numbers
    let rows: Vec<(String, Vec<String>)> = values1
        .iter()
        .zip(matrix)
        .map(|(val1, cells)| {
            let cells = cells
                .iter()
                .map(|cell| cell.map_or_else(|| "ERR".to_string(), display_number))
                .collect();
            (format!("{val1:.4}"), cells)
        })
        .collect();
    let heads: Vec<String> = values2.iter().map(|val2| format!("{val2:.4}")).collect();
    let first_width = rows
        .iter()
        .map(|(input, _)| input.len())
        .chain([vary.chars().count()])
        .max()
        .unwrap_or(12);
    let width = rows
        .iter()
        .flat_map(|(_, cells)| cells.iter())
        .chain(heads.iter())
        .map(|text| text.chars().count())
        .max()
        .unwrap_or(0)
        .max(12)
        + 1;

    // Header row
    out!(c, "{:>first_width$}", vary.bright_yellow());
    for head in &heads {
        out!(c, "{:>width$}", head.dimmed());
    }
    outln!(c);
    outln!(c, "{}", "─".repeat(first_width + heads.len() * width));

    // Data rows
    for (input, cells) in &rows {
        out!(c, "{:>first_width$}", input.bright_yellow());
        for text in cells {
            match text.as_str() {
                "ERR" => out!(c, "{:>width$}", text.red()),
                _ => out!(c, "{:>width$}", text.green()),
            }
        }
        outln!(c);
    }
}

#[cfg(test)]
mod tests {
    /// A model with an `MC.*` input is refused with what to run instead
    /// (roadmap E-MC-MESSAGE)
    mod distributions {
        #[test]
        fn sensitivity_names_the_distribution_and_what_to_run() {
            crate::cli::commands::assert_distributions_refused(
                &[
                    "sensitivity",
                    "FILE",
                    "--vary",
                    "assumptions.units",
                    "--range",
                    "50,150,50",
                    "--output",
                    "outputs.profit",
                ],
                "sensitivity",
            );
        }
    }

    use super::parse_range;
    use crate::cli::commands::results::SensitivityData;

    /// The matrix of a two-input result (empty for one input)
    fn two_var_matrix(data: SensitivityData) -> Vec<Vec<Option<f64>>> {
        let json = serde_json::to_value(data).unwrap();
        serde_json::from_value(json["TwoVar"]["matrix"].clone()).unwrap_or_default()
    }

    #[test]
    fn a_range_with_more_steps_than_can_be_counted_is_refused() {
        assert_eq!(parse_range("0,1,0.5").unwrap(), [0.0, 0.5, 1.0]);
        for range in ["0,1e300,1e-300", "0,inf,1"] {
            let err = parse_range(range).unwrap_err().to_string();
            assert!(
                err.contains(&format!("The range {range} has too many steps")),
                "{err}"
            );
        }
    }

    mod date_inputs {
        //! `forge sensitivity` refuses a date input however it is asked for (ADR-060):
        //! an error and a failing exit code, not a table of errors.

        use crate::cli::forge;

        const MODEL: &str = r#"_forge_version: "5.4.0"
inputs:
  last_actual: 2026-10-31
out:
  next:
    formula: "=EOMONTH(inputs.last_actual, 1)"
"#;

        #[test]
        fn sensitivity_refuses_a_date_input_and_says_so_in_its_exit_code() {
            let dir = tempfile::tempdir().expect("temp dir");
            let path = dir.path().join("model.yaml");
            std::fs::write(&path, MODEL).expect("model written");
            let ran = forge(&[
                "sensitivity",
                path.to_str().expect("path"),
                "--vary",
                "inputs.last_actual",
                "--range",
                "1,3,1",
                "--output",
                "out.next",
            ]);
            assert!(!ran.success(), "a date input must fail the command");
            let text = ran.both();
            assert!(text.contains("varying a date is not supported"), "{text}");
            assert!(!text.contains("Sensitivity analysis complete"), "{text}");
        }
    }

    mod printed {
        //! `forge sensitivity`'s table lines up under its header, and a program can
        //! read the full numbers as JSON instead of parsing the table (the end-to-end
        //! pass of 2026-11-12).

        use crate::cli::forge;

        #[test]
        fn the_sensitivity_table_lines_up_under_its_header() {
            let text = forge(&[
                "sensitivity",
                "examples/periods/dcf.yaml",
                "--vary",
                "assumptions.wacc",
                "-r",
                "0.08,0.11,0.01",
                "--output-var",
                "valuation.equity_value",
            ])
            .printed();
            let lines: Vec<&str> = text.lines().collect();
            let header = lines
                .iter()
                .position(|l| {
                    l.contains("assumptions.wacc")
                        && l.contains("valuation.equity_value")
                        && !l.contains('→')
                })
                .unwrap();
            let header_line = lines[header];
            // Every row ends where the header ends, and its input where the header's input ends
            let first_end =
                header_line.find("assumptions.wacc").unwrap() + "assumptions.wacc".len();
            for row in &lines[header + 2..header + 6] {
                assert_eq!(row.chars().count(), header_line.chars().count(), "{row}");
                assert_eq!(&row[first_end..=first_end], " ", "{row}");
            }
            assert!(text.contains("7,693,647.27"), "{text}");
        }

        #[test]
        fn sensitivity_can_print_json_with_the_full_numbers() {
            let text = forge(&[
                "sensitivity",
                "examples/periods/dcf.yaml",
                "--vary",
                "assumptions.wacc",
                "-r",
                "0.08,0.09,0.01",
                "--output-var",
                "valuation.equity_value",
                "--format",
                "json",
            ])
            .printed();
            let json: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(json["vary"], "assumptions.wacc");
            let entries = json["data"]["OneVar"]["entries"].as_array().unwrap();
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0]["input"], 0.08);
            assert_eq!(entries[0]["output"], 7_693_647.272_727_27);
        }
    }

    mod microgrid_sizes {
        //! examples/microgrid: the plan's 3.4 megawatts of solar and 10 megawatt-hours
        //! of battery are the best sizes; a size either side is worth less. The `run` of
        //! the Python in prediction/scenarios.rs (`microgrid_example`):
        //! ```python
        //! run(batt=8), run(batt=10), run(batt=12)  # 11934790.42001938 13161304.863400452 12708614.061956115
        //! run(mwp=3.0), run(mwp=3.8)               # 12813461.91775453 12650614.202066816
        //! ```

        use crate::cli::sensitivity_core;
        use std::path::Path;

        fn sweep(vary: &str, range: &str) -> Vec<f64> {
            let result = sensitivity_core(
                Path::new("examples/microgrid/microgrid.yaml"),
                vary,
                range,
                None,
                None,
                "verdict.project_npv",
            )
            .unwrap();
            let json = serde_json::to_value(result.data).unwrap();
            let entries = json["OneVar"]["entries"].as_array().unwrap();
            entries
                .iter()
                .map(|e| e["output"].as_f64().unwrap())
                .collect()
        }

        #[test]
        fn the_plan_is_the_best_size_of_battery_and_of_solar() {
            let battery = sweep("system.battery_mwh", "8,12,2");
            let want = [
                11_934_790.420_019_38,
                13_161_304.863_400_452,
                12_708_614.061_956_115,
            ];
            assert_eq!(battery.len(), want.len());
            for (got, want) in battery.iter().zip(want) {
                assert!((got - want).abs() <= 1e-6, "{got}, not {want}");
            }
            let solar = sweep("system.solar_mwp", "3.0,3.8,0.4");
            let want = [
                12_813_461.917_754_53,
                13_161_304.863_400_452,
                12_650_614.202_066_816,
            ];
            assert_eq!(solar.len(), want.len());
            for (got, want) in solar.iter().zip(want) {
                assert!((got - want).abs() <= 1e-6, "{got}, not {want}");
            }
        }
    }

    mod two_inputs {
        //! The matrix of two inputs: a formula the output does not use may fail
        //! in a cell without blanking it. `s.per_point` divides by zero at
        //! x = 1; `s.out` = x + y does not use it, so its row at x = 1 is
        //! 1 + 0 and 1 + 1. Both cells were empty (`ERR` on the screen). An
        //! output that uses the failing formula still has no value there.

        use super::super::sensitivity_core;
        use super::two_var_matrix;
        use crate::cli::forge;

        const MODEL: &str = "_forge_version: \"5.4.0\"\ns:\n  x: 2\n  y: 0\n  per_point: {formula: \"=1 / (x - 1)\"}\n  out: {formula: \"=x + y\"}\n  uses_it: {formula: \"=per_point + y\"}\n";

        fn file() -> (tempfile::TempDir, std::path::PathBuf) {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("m.yaml");
            std::fs::write(&path, MODEL).unwrap();
            (dir, path)
        }

        fn matrix(output: &str) -> Vec<Vec<Option<f64>>> {
            let (_dir, path) = file();
            let result =
                sensitivity_core(&path, "s.x", "1,2,1", Some("s.y"), Some("0,1,1"), output)
                    .unwrap();
            two_var_matrix(result.data)
        }

        #[test]
        fn a_failure_the_output_does_not_use_leaves_its_cells() {
            assert_eq!(
                matrix("s.out"),
                [[Some(1.0), Some(2.0)], [Some(2.0), Some(3.0)]]
            );
            assert_eq!(matrix("s.uses_it"), [[None, None], [Some(1.0), Some(2.0)]]);
        }

        #[test]
        fn the_printed_matrix_shows_err_only_where_the_output_fails() {
            let (_dir, path) = file();
            let run = |output: &str| {
                forge(&[
                    "sensitivity",
                    path.to_str().unwrap(),
                    "--vary",
                    "s.x",
                    "-r",
                    "1,2,1",
                    "--vary2",
                    "s.y",
                    "--range2",
                    "0,1,1",
                    "--output-var",
                    output,
                ])
                .printed()
            };
            let out = run("s.out");
            assert!(!out.contains("ERR"), "{out}");
            let uses_it = run("s.uses_it");
            assert_eq!(uses_it.matches("ERR").count(), 2, "{uses_it}");
        }
    }

    mod acquisition_example {
        //! The banker's table of examples/acquisition: the value created at each
        //! premium and each run rate of cost synergies. Two corners, as the `run`
        //! of the Python in prediction/scenarios.rs (`acquisition_example`) gives
        //! them: run(premium=0.45, cost=30) -77.39138476481048 and
        //! run(premium=0.2, cost=60) 583.5065697844925; the plan, 224.50793341829012,
        //! is in the middle.

        use super::super::sensitivity_core;
        use super::two_var_matrix;
        use std::path::Path;

        #[test]
        fn the_table_runs_from_a_loss_to_more_than_twice_the_plan() {
            let result = sensitivity_core(
                Path::new("examples/acquisition/deal.yaml"),
                "offer.premium",
                "0.20,0.50,0.05",
                Some("synergies.cost_run_rate"),
                Some("20,60,10"),
                "valuation.value_created",
            )
            .unwrap();
            let matrix = two_var_matrix(result.data);
            assert_eq!((matrix.len(), matrix[0].len()), (7, 5));
            let cell = |row: usize, col: usize| matrix[row][col].unwrap();
            for (got, want) in [
                (cell(2, 2), 224.507_933_418_290_12),
                (cell(5, 1), -77.391_384_764_810_48),
                (cell(0, 4), 583.506_569_784_492_5),
            ] {
                assert!((got - want).abs() <= 1e-9, "{got}, not {want}");
            }
        }
    }

    mod included_inputs {
        //! Roadmap T1-VARY-INCLUDED: a sensitivity table over an input of a file
        //! the model includes, in one direction and in two, is what the plain
        //! model gives with each value.

        use super::super::sensitivity_core;
        use super::two_var_matrix;
        use crate::core::ArrayCalculator;
        use std::path::Path;

        const MODEL: &str = "examples/acquisition/deal.yaml";
        const CASH: &str = "@target.company.free_cash_flow_next_year";

        fn value_created(inputs: &[(&str, f64)]) -> f64 {
            let mut model = crate::parser::parse_model(Path::new(MODEL)).unwrap();
            for (name, value) in inputs {
                model.override_scalar(name, Some(*value), None).unwrap();
            }
            let result = ArrayCalculator::new(model).calculate_all().unwrap();
            result.scalars["valuation.value_created"].value.unwrap()
        }

        #[test]
        fn one_input_of_an_included_file() {
            let result = sensitivity_core(
                Path::new(MODEL),
                CASH,
                "50,70,10",
                None,
                None,
                "value_created",
            )
            .unwrap();
            assert_eq!(result.vary, CASH);
            let json = serde_json::to_value(result.data).unwrap();
            let entries = json["OneVar"]["entries"].as_array().unwrap();
            assert_eq!(entries.len(), 3);
            for entry in entries {
                let input = entry["input"].as_f64().unwrap();
                let want = value_created(&[(CASH, input)]);
                let got = entry["output"].as_f64().unwrap();
                assert!((got - want).abs() <= 1e-9, "{input}");
            }
        }

        #[test]
        fn two_inputs_one_of_them_included() {
            let result = sensitivity_core(
                Path::new(MODEL),
                "offer.premium",
                "0.2,0.4,0.2",
                Some(CASH),
                Some("50,70,20"),
                "valuation.value_created",
            )
            .unwrap();
            let matrix = two_var_matrix(result.data);
            for (row, premium) in [0.2, 0.4].into_iter().enumerate() {
                for (col, cash) in [50.0, 70.0].into_iter().enumerate() {
                    let want = value_created(&[("offer.premium", premium), (CASH, cash)]);
                    let got = matrix[row][col].unwrap();
                    assert!((got - want).abs() <= 1e-9, "{premium} {cash}: {got}");
                }
            }
        }

        /// The printed matrix took the included input as unknown ("Variable
        /// '`@target.company.free_cash_flow_next_year`' not found"), though
        /// the JSON and the one-input table took it.
        #[test]
        fn the_printed_matrix_takes_an_included_input_too() {
            let text = crate::cli::forge(&[
                "sensitivity",
                MODEL,
                "--vary",
                "offer.premium",
                "-r",
                "0.2,0.4,0.2",
                "--vary2",
                CASH,
                "--range2",
                "50,70,20",
                "--output-var",
                "valuation.value_created",
            ])
            .printed();
            let rows: Vec<&str> = text.lines().skip_while(|l| !l.starts_with('─')).collect();
            assert_eq!(
                rows[1..3],
                [
                    "       0.2000        92.86       516.27",
                    "       0.4000      -151.94       271.47"
                ]
            );
            for (premium, cash, shown) in [(0.2, 50.0, "92.86"), (0.4, 70.0, "271.47")] {
                let want = value_created(&[("offer.premium", premium), (CASH, cash)]);
                assert_eq!(crate::cli::commands::display_number(want), shown);
            }
        }
    }

    #[test]
    fn a_range_that_is_not_start_end_step_is_refused() {
        for (range, want) in [
            (
                "1,2",
                "Invalid range format '1,2'. Expected: start,end,step (e.g., 0.01,0.15,0.02)",
            ),
            ("a,2,1", "Invalid start value: 'a'"),
            ("1,b,1", "Invalid end value: 'b'"),
            ("1,2,c", "Invalid step value: 'c'"),
            ("1,2,0", "Step must be positive"),
            ("1,2,-1", "Step must be positive"),
            ("3,2,1", "Start must be less than or equal to end"),
        ] {
            let err = parse_range(range).unwrap_err().to_string();
            assert_eq!(err, format!("Validation error: {want}"), "{range}");
        }
        assert_eq!(parse_range(" 1 , 2 , 1 ").unwrap(), [1.0, 2.0]);
    }

    #[test]
    fn an_override_of_a_name_the_model_does_not_have_is_refused() {
        let model =
            crate::parser::parse_model(std::path::Path::new("examples/acquisition/deal.yaml"))
                .unwrap();
        let err = super::calculate_with_override(&model, "nope", 1.0, "valuation.value_created")
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Validation error: Scenario override 'nope' does not name a scalar in the model."
        );
        let dated = crate::parser::parse_model_text(
            "m.yaml",
            "_forge_version: \"5.4.0\"\ns:\n  d: 2026-10-31\n  out: {formula: \"=1\"}\n",
            &std::collections::HashMap::new(),
        )
        .unwrap();
        let err = super::calculate_with_override(&dated, "s.d", 1.0, "s.out").unwrap_err();
        assert_eq!(
            err.to_string(),
            "Validation error: 's.d' is a date (2026-10-31), and this command varies numbers: \
             varying a date is not supported (ADR-060)"
        );
    }

    mod refused_and_failed {
        //! What each way in refuses before it calculates (the file, a name, a
        //! date, a range), and a value whose output fails. `s.per_point`
        //! divides by zero at x = 1, and `b.out` uses it; `a.out` does not.

        use super::super::sensitivity_core;
        use crate::cli::forge;

        const MODEL: &str = "_forge_version: \"5.4.0\"\ns:\n  x: 2\n  y: 0\n  d: 2026-10-31\n  per_point: {formula: \"=1 / (s.x - 1)\"}\na:\n  out: {formula: \"=s.x + s.y\"}\nb:\n  out: {formula: \"=s.per_point + s.y\"}\n";
        const SCALARS: &str = r#"["s.x", "s.y", "s.d", "s.per_point", "a.out", "b.out"]"#;

        fn file() -> (tempfile::TempDir, std::path::PathBuf) {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("m.yaml");
            std::fs::write(&path, MODEL).unwrap();
            (dir, path)
        }

        #[test]
        fn the_core_refuses_each_thing_it_checks() {
            let (_dir, path) = file();
            let refused = |file: &std::path::Path,
                           (vary, range): (&str, &str),
                           (vary2, range2): (Option<&str>, Option<&str>),
                           output: &str| {
                sensitivity_core(file, vary, range, vary2, range2, output)
                    .unwrap_err()
                    .to_string()
            };
            let missing = path.with_file_name("missing.yaml");
            assert_eq!(
                refused(&missing, ("s.x", "1,2,1"), (None, None), "a.out"),
                format!(
                    "IO error: {}: No such file or directory (os error 2)",
                    missing.display()
                )
            );
            assert_eq!(
                refused(&path, ("s.q", "1,2,1"), (None, None), "a.out"),
                format!("Validation error: Variable 's.q' not found. Available scalars: {SCALARS}")
            );
            assert_eq!(
                refused(
                    &path,
                    ("s.x", "1,2,1"),
                    (Some("s.q"), Some("0,1,1")),
                    "a.out"
                ),
                format!("Validation error: Variable 's.q' not found. Available scalars: {SCALARS}")
            );
            assert_eq!(
                refused(&path, ("s.x", "1,2,1"), (None, None), "out"),
                "Validation error: 'out' could be any of a.out, b.out; write the full name"
            );
            assert_eq!(
                refused(
                    &path,
                    ("s.x", "1,2,1"),
                    (Some("s.d"), Some("0,1,1")),
                    "a.out"
                ),
                "Validation error: 's.d' is a date (2026-10-31), and sensitivity varies \
                 numbers: varying a date is not supported (ADR-060)"
            );
            assert_eq!(
                refused(&path, ("s.x", "1,2"), (None, None), "a.out"),
                "Validation error: Invalid range format '1,2'. Expected: start,end,step (e.g., \
                 0.01,0.15,0.02)"
            );
            assert_eq!(
                refused(&path, ("s.x", "1,2,1"), (Some("s.y"), Some("0,1")), "a.out"),
                "Validation error: Invalid range format '0,1'. Expected: start,end,step (e.g., \
                 0.01,0.15,0.02)"
            );
        }

        #[test]
        fn an_output_the_model_does_not_have_fails_each_entry() {
            let (_dir, path) = file();
            let result = sensitivity_core(&path, "s.x", "1,2,1", None, None, "nope").unwrap();
            let json = serde_json::to_value(result.data).unwrap();
            let entry = |input: f64| {
                serde_json::json!({
                    "input": input,
                    "output": null,
                    "error": "Validation error: Output variable 'nope' not found in model",
                })
            };
            assert_eq!(
                json,
                serde_json::json!({"OneVar": {"entries": [entry(1.0), entry(2.0)]}})
            );
        }

        #[test]
        fn the_printed_commands_refuse_what_the_core_refuses() {
            let (_dir, path) = file();
            let p = path.to_str().unwrap();
            let missing = path.with_file_name("missing.yaml");
            let missing = missing.to_str().unwrap();
            for (args, want) in [
                (
                    vec![
                        missing,
                        "--vary",
                        "s.x",
                        "-r",
                        "1,2,1",
                        "--output-var",
                        "a.out",
                    ],
                    "No such file or directory (os error 2)",
                ),
                (
                    vec![p, "--vary", "s.x", "-r", "1,2", "--output-var", "a.out"],
                    "Invalid range format '1,2'",
                ),
                (
                    vec![
                        p,
                        "--vary",
                        "s.x",
                        "-r",
                        "1,2,1",
                        "--vary2",
                        "s.y",
                        "--range2",
                        "0,1",
                        "--output-var",
                        "a.out",
                    ],
                    "Invalid range format '0,1'",
                ),
                (
                    vec![
                        p,
                        "--vary",
                        "s.x",
                        "-r",
                        "1,2",
                        "--output-var",
                        "a.out",
                        "--format",
                        "json",
                    ],
                    "Invalid range format '1,2'",
                ),
            ] {
                let ran = forge(&[&["sensitivity"], args.as_slice()].concat());
                assert!(!ran.success(), "{args:?}");
                let text = ran.both();
                assert!(text.contains(want), "{text}");
                assert!(!text.contains("Sensitivity analysis complete"), "{text}");
            }
        }

        #[test]
        fn a_failed_value_shows_err_in_the_table_and_its_reason_under_it() {
            let (_dir, path) = file();
            let p = path.to_str().unwrap();
            let text = forge(&[
                "sensitivity",
                p,
                "--vary",
                "s.x",
                "-r",
                "1,2,1",
                "--output-var",
                "b.out",
                "--verbose",
            ])
            .printed();
            assert_eq!(
                text,
                format!(
                    "🔥 Forge - Sensitivity Analysis\n   File: {p}\n   Vary: s.x (1,2,1)\n   \
                     Output: b.out\n\n   Range 1: 2 values from 1 to 2\n\n📊 Sensitivity \
                     Table: s.x → b.out\n────────────\n   s.x b.out\n────────────\n1.0000   \
                     ERR\n2.0000  1.00\n────────────\nERR s.x = 1.0000: Formula evaluation \
                     error: 1 formula failed:\n  - s.per_point: Division by zero (formula: =1 \
                     / (s.x - 1))\n  Not calculated, because they use a failed formula:\n  - \
                     b.out (uses s.per_point)\n\n✅ Sensitivity analysis complete\n"
                )
            );
        }

        #[test]
        fn the_verbose_matrix_gives_both_ranges() {
            let (_dir, path) = file();
            let p = path.to_str().unwrap();
            let text = forge(&[
                "sensitivity",
                p,
                "--vary",
                "s.x",
                "-r",
                "1,2,1",
                "--vary2",
                "s.y",
                "--range2",
                "0,1,1",
                "--output-var",
                "b.out",
                "--verbose",
            ])
            .printed();
            assert_eq!(
                text,
                format!(
                    "🔥 Forge - Sensitivity Analysis\n   File: {p}\n   Vary: s.x (1,2,1)\n   \
                     Vary2: s.y (0,1,1)\n   Output: b.out\n\n   Range 1: 2 values from 1 to \
                     2\n   Range 2: 2 values from 0 to 1\n\n📊 Sensitivity Matrix: (s.x, s.y) \
                     → b.out\n   s.x       0.0000       1.0000\n────────────────────────────────\n\
                     1.0000          ERR          ERR\n2.0000         1.00         2.00\n\n✅ \
                     Sensitivity analysis complete\n"
                )
            );
        }
    }
}
