//! `forge sensitivity`: an output for each value of one input, or a table
//! for two.

use crate::cli::console::{out, outln};
use crate::cli::Console;
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
    #[allow(clippy::cast_precision_loss)] // step index is always small enough for f64
    let values: Vec<f64> = (0..num_steps)
        .map(|i| step.mul_add(i as f64, start))
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

    // Calculate
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all()?;

    // Get output value
    result.scalars.get(output_name).map_or_else(
        || {
            Err(ForgeError::Validation(format!(
                "Output variable '{output_name}' not found in model"
            )))
        },
        |scalar| {
            scalar.value.ok_or_else(|| {
                ForgeError::Validation(format!("Output variable '{output_name}' has no value"))
            })
        },
    )
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
                let calculator = ArrayCalculator::new(model);
                let cell = match calculator.calculate_all() {
                    Ok(result) => result.scalars.get(output).and_then(|s| s.value),
                    Err(_) => None,
                };
                row.push(cell);
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
        match ArrayCalculator::new(model).calculate_all() {
            Ok(result) => result.scalars.get(output).map_or_else(
                || "?".to_string(),
                |scalar| scalar.value.map_or_else(|| "-".to_string(), display_number),
            ),
            Err(_) => "ERR".to_string(),
        }
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
                "ERR" | "?" => out!(c, "{:>width$}", text.red()),
                "-" => out!(c, "{:>width$}", text.dimmed()),
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
}
