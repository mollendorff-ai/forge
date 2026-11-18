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

    // Override the variable
    if let Some(scalar) = model.scalars.get_mut(var_name) {
        scalar.value = Some(var_value);
        scalar.formula = None; // Clear formula since we're using override
    } else {
        // Create new scalar
        model.scalars.insert(
            var_name.to_string(),
            crate::types::Variable::new(var_name.to_string(), Some(var_value), None),
        );
    }

    // Calculate for the output alone: a formula it does not use may fail at
    // this value of the input without failing the answer
    ArrayCalculator::new(model).calculate_scalar(output_name)
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
    let base_model = parser::parse_model(file)?;
    let vary = &crate::cli::commands::names::scalar_name(&base_model, vary, false)?;
    let vary2 = vary2
        .map(|v| crate::cli::commands::names::scalar_name(&base_model, v, false))
        .transpose()?;
    let vary2 = vary2.as_deref();
    let output = &crate::cli::commands::names::scalar_name(&base_model, output, true)?;
    for name in std::iter::once(vary.as_str()).chain(vary2) {
        base_model
            .refuse_date_input(name, "sensitivity")
            .map_err(ForgeError::Validation)?;
    }

    let values1 = parse_range(range)?;

    let data = if let (Some(v2), Some(r2)) = (vary2, range2) {
        let values2 = parse_range(r2)?;
        let mut matrix = Vec::new();
        for val1 in &values1 {
            let mut row = Vec::new();
            for val2 in &values2 {
                let mut model = base_model.clone();
                if let Some(s) = model.scalars.get_mut(vary) {
                    s.value = Some(*val1);
                    s.formula = None;
                }
                if let Some(s) = model.scalars.get_mut(v2) {
                    s.value = Some(*val2);
                    s.formula = None;
                }
                // A formula the output does not use may fail here
                row.push(ArrayCalculator::new(model).calculate_scalar(output).ok());
            }
            matrix.push(row);
        }
        crate::cli::commands::results::SensitivityData::TwoVar {
            vary2: v2.to_string(),
            row_values: values1,
            col_values: values2,
            matrix,
        }
    } else {
        let mut entries = Vec::new();
        for val in &values1 {
            match calculate_with_override(&base_model, vary, *val, output) {
                Ok(result) => entries.push(crate::cli::commands::results::SensitivityEntry {
                    input: *val,
                    output: Some(result),
                    error: None,
                }),
                Err(e) => entries.push(crate::cli::commands::results::SensitivityEntry {
                    input: *val,
                    output: None,
                    error: Some(e.to_string()),
                }),
            }
        }
        crate::cli::commands::results::SensitivityData::OneVar { entries }
    };

    Ok(crate::cli::commands::results::SensitivityResult {
        vary: vary.clone(),
        output: output.clone(),
        data,
    })
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

    let base_model = parser::parse_model(file)?;
    let vary = &crate::cli::commands::names::scalar_name(&base_model, vary, false)?;
    let vary2 = vary2
        .map(|v| crate::cli::commands::names::scalar_name(&base_model, v, false))
        .transpose()?;
    let vary2 = vary2.as_deref();
    let output = &crate::cli::commands::names::scalar_name(&base_model, output, true)?;

    for name in std::iter::once(vary.as_str()).chain(vary2) {
        base_model
            .refuse_date_input(name, "sensitivity")
            .map_err(ForgeError::Validation)?;
    }

    // Parse range
    let values1 = parse_range(range)?;

    if verbose {
        outln!(
            c,
            "   Range 1: {} values from {} to {}",
            values1.len(),
            values1.first().unwrap_or(&0.0),
            values1.last().unwrap_or(&0.0)
        );
    }

    // Two-variable analysis
    if let (Some(v2), Some(r2)) = (vary2, range2) {
        run_two_var_sensitivity(c, &base_model, vary, (v2, r2), output, &values1, verbose)?;
    } else {
        print_one_var_sensitivity(c, &base_model, vary, output, &values1);
    }

    outln!(c, "\n{}", "✅ Sensitivity analysis complete".bold().green());
    Ok(())
}

/// The table of a one-variable sensitivity: each input value and the output
/// it gives, in columns wide enough for the names and the numbers.
fn print_one_var_sensitivity(
    c: &mut Console<'_>,
    base_model: &crate::types::ParsedModel,
    vary: &str,
    output: &str,
    values: &[f64],
) {
    outln!(
        c,
        "\n{} {} → {}",
        "📊 Sensitivity Table:".bold().cyan(),
        vary.yellow(),
        output.bright_blue()
    );
    // Wide enough for the names and the numbers, so the header lines up
    let rows: Vec<(String, Result<f64, String>)> = values
        .iter()
        .map(|val| {
            (
                format!("{val:.4}"),
                calculate_with_override(base_model, vary, *val, output).map_err(|e| e.to_string()),
            )
        })
        .collect();
    let shown = |result: &Result<f64, String>| match result {
        Ok(v) => display_number(*v),
        Err(e) => format!("ERR: {e}"),
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
    let json = serde_json::to_string_pretty(&crate::core::precision::Rounded(&result))
        .map_err(|e| ForgeError::Validation(format!("Failed to write JSON: {e}")))?;
    outln!(c, "{json}");
    Ok(())
}

/// Run two-variable sensitivity analysis matrix
fn run_two_var_sensitivity(
    c: &mut Console<'_>,
    base_model: &crate::types::ParsedModel,
    vary: &str,
    (v2, r2): (&str, &str),
    output: &str,
    values1: &[f64],
    verbose: bool,
) -> ForgeResult<()> {
    for name in [vary, v2] {
        base_model
            .refuse_date_input(name, "sensitivity")
            .map_err(ForgeError::Validation)?;
    }
    if !base_model.scalars.contains_key(v2) {
        return Err(ForgeError::Validation(format!(
            "Variable '{}' not found. Available scalars: {:?}",
            v2,
            base_model.scalars.keys().collect::<Vec<_>>()
        )));
    }

    let values2 = parse_range(r2)?;

    if verbose {
        outln!(
            c,
            "   Range 2: {} values from {} to {}",
            values2.len(),
            values2.first().unwrap_or(&0.0),
            values2.last().unwrap_or(&0.0)
        );
    }

    // Calculate matrix
    outln!(
        c,
        "\n{} {} → {}",
        "📊 Sensitivity Matrix:".bold().cyan(),
        format!("({vary}, {v2})").yellow(),
        output.bright_blue()
    );

    // Calculate every cell first, so the columns can be as wide as their numbers
    let cell = |val1: f64, val2: f64| {
        let mut model = base_model.clone();
        if let Some(s) = model.scalars.get_mut(vary) {
            s.value = Some(val1);
            s.formula = None;
        }
        if let Some(s) = model.scalars.get_mut(v2) {
            s.value = Some(val2);
            s.formula = None;
        }
        // A formula the output does not use may fail here
        ArrayCalculator::new(model)
            .calculate_scalar(output)
            .map_or_else(|_| "ERR".to_string(), display_number)
    };
    let rows: Vec<(String, Vec<String>)> = values1
        .iter()
        .map(|val1| {
            (
                format!("{val1:.4}"),
                values2.iter().map(|val2| cell(*val1, *val2)).collect(),
            )
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_range;

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
        use crate::cli::commands::results::SensitivityData;
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
            match result.data {
                SensitivityData::TwoVar { matrix, .. } => matrix,
                SensitivityData::OneVar { .. } => panic!("one input"),
            }
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
}
