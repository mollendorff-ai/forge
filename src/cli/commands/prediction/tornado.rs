//! `forge tornado`: how far each input, from low to high, moves the output.

use super::serialize_for_file;
use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use crate::tornado::{TornadoConfig, TornadoEngine};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

/// Generate tornado sensitivity analysis and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no tornado section exists,
/// or the analysis fails.
pub fn tornado_core(
    file: &Path,
    output_var: Option<&str>,
) -> ForgeResult<crate::tornado::TornadoResult> {
    let yaml_content = parser::read_model_text(file)?;
    let model = parser::parse_model(file)?;

    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let mut config: TornadoConfig = if let Some(tornado_value) = value.get("tornado") {
        yaml_serde::from_value(tornado_value.clone())
            .map_err(|e| ForgeError::Validation(format!("tornado config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'tornado' section found in YAML".to_string(),
        ));
    };

    if let Some(out_var) = output_var {
        config.output = out_var.to_string();
    }

    let engine = TornadoEngine::new(config, model).map_err(ForgeError::Validation)?;
    engine.analyze().map_err(ForgeError::Eval)
}

/// Execute the tornado command - sensitivity tornado diagram
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no tornado section exists,
/// or the analysis fails.
pub fn tornado(
    c: &mut Console<'_>,
    file: &Path,
    output_var: Option<&str>,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "🌪️ Forge - Tornado Diagram".bold().green());
    outln!(c, "   File: {}", file.display());
    outln!(c);

    // Parse YAML and model
    let yaml_content = parser::read_model_text(file)?;
    let model = parser::parse_model(file)?;

    // Parse tornado config
    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let mut config: TornadoConfig = if let Some(tornado_value) = value.get("tornado") {
        yaml_serde::from_value(tornado_value.clone())
            .map_err(|e| ForgeError::Validation(format!("tornado config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'tornado' section found in YAML".to_string(),
        ));
    };

    // Override output variable if specified
    if let Some(out_var) = output_var {
        config.output = out_var.to_string();
    }

    // Display config
    outln!(c, "   Output variable: {}", config.output.bright_blue());
    outln!(c, "   Inputs to vary: {}", config.inputs.len());
    if verbose {
        for input in &config.inputs {
            outln!(
                c,
                "      {} [{:.2} - {:.2}]",
                input.name.cyan(),
                input.low,
                input.high
            );
        }
    }
    outln!(c);

    // Create engine and analyze
    let engine = TornadoEngine::new(config, model).map_err(ForgeError::Validation)?;

    if verbose {
        outln!(c, "{}", "🔄 Calculating sensitivities...".cyan());
    }

    let result = engine.analyze().map_err(ForgeError::Eval)?;

    // Display results
    outln!(c, "{}", "📊 Tornado Diagram:".bold().green());
    outln!(
        c,
        "   Base value: {}",
        crate::cli::commands::display_number(result.base_value).bold()
    );
    outln!(c);

    // Tornado bars (already sorted by impact)
    let max_impact = result
        .bars
        .iter()
        .map(|b| b.swing.abs())
        .fold(0.0f64, f64::max);

    let name_width = result
        .bars
        .iter()
        .map(|b| b.input_name.chars().count())
        .max()
        .unwrap_or(0);

    for bar in &result.bars {
        // Truncation/sign-loss impossible: value is always in 0.0..=30.0
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let bar_width = if max_impact > 0.0 {
            ((bar.swing.abs() / max_impact) * 30.0) as usize
        } else {
            0
        };
        let bar_str = "█".repeat(bar_width.max(1));

        // `swing` is the output at the high input minus the output at the low input.
        outln!(
            c,
            "   {:name_width$} |{:30}| swing {}  (output {} at low, {} at high)",
            bar.input_name.bright_blue(),
            bar_str.cyan(),
            crate::cli::commands::display_number(bar.swing.abs()),
            crate::cli::commands::display_number(bar.output_at_low),
            crate::cli::commands::display_number(bar.output_at_high)
        );
        if let Some(note) = &bar.note {
            outln!(c, "   {:name_width$}  {}", "", note.yellow());
        }
    }
    outln!(c);

    // Write output if specified
    if let Some(output_path) = output_file {
        let output_str = serialize_for_file(&result, &output_path)?;
        fs::write(&output_path, output_str).map_err(ForgeError::Io)?;
        outln!(
            c,
            "{}",
            format!("💾 Results written to {}", output_path.display())
                .bold()
                .green()
        );
    }

    outln!(c, "{}", "✅ Tornado diagram complete".bold().green());
    Ok(())
}

#[cfg(test)]
mod tests {

    mod tornado_note {
        //! `forge tornado` says when an input cannot move the output, instead of
        //! listing it with a swing of 0 and no reason (roadmap T1-PIPELINE-FINDINGS).

        use crate::cli::forge;

        #[test]
        fn the_printed_tornado_names_the_input_that_cannot_move_the_output() {
            let dir = tempfile::tempdir().expect("temp dir");
            let file = dir.path().join("t.yaml");
            std::fs::write(
                &file,
                r#"_forge_version: "5.1.0"
price: 10
tax_rate: 0.2
revenue: {formula: "=price * 100", value: 1000}
tornado:
  output: revenue
  inputs:
    - {name: price, low: 5, high: 15}
    - {name: tax_rate, low: 0.1, high: 0.3}
"#,
            )
            .expect("write");
            let ran = forge(&["tornado", file.to_str().expect("path")]);
            assert!(ran.success(), "{}", ran.both());
            assert_eq!(ran.out.matches("does not use it").count(), 1, "{}", ran.out);
        }
    }

    mod printed_numbers {
        //! The amounts `forge tornado` prints
        //! follow `forge report`'s rule (Session Prompt part 1, item 5): two decimals
        //! and thousands separators, four significant digits under 1, and money with
        //! the sign before the `$`. JSON and files keep the raw numbers.

        use crate::cli::forge;

        #[test]
        fn tornado_prints_amounts_with_separators() {
            let text = forge(&["tornado", "examples/tornado.yaml"]).printed();
            assert!(text.contains("Base value: -625,000.00"), "{text}");
            // initial_investment 700,000 x 1: -425,000 at low, -1,125,000 at high
            assert!(
                text.contains(
                    "swing 700,000.00  (output -425,000.00 at low, -1,125,000.00 at high)"
                ),
                "{text}"
            );
        }

        #[test]
        fn tornado_keeps_the_digits_of_a_rate() {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("rate.yaml");
            // margin = profit / revenue = (1000 - cost) / 1000: cost 840 gives 0.16,
            // cost 903 gives 0.097; the swing is 0.063 (0.06 at two decimals)
            std::fs::write(
                &file,
                r#"_forge_version: "5.1.0"
tornado:
  output: plan.margin
  inputs:
    - name: plan.cost
      low: 840
      high: 903
plan:
  revenue: 1000
  cost: 870
  margin: {formula: "=(plan.revenue - plan.cost) / plan.revenue"}
"#,
            )
            .unwrap();
            let text = forge(&["tornado", file.to_str().unwrap()]).printed();
            assert!(text.contains("swing 0.063"), "{text}");
            assert!(
                text.contains("0.16 at low") || text.contains("0.160 at low"),
                "{text}"
            );
        }
    }
}
