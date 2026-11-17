//! `forge real-options`: what the choice to wait, expand or abandon is worth.

use super::serialize_for_file;
use crate::cli::console::outln;
use crate::cli::Console;
use crate::core::precision::money;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use crate::real_options::{RealOptionsConfig, RealOptionsEngine};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

/// Value real options and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no `real_options` section exists,
/// or the analysis fails.
pub fn real_options_core(file: &Path) -> ForgeResult<crate::real_options::OptionsResult> {
    let yaml_content = parser::read_model_text(file)?;

    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let config: RealOptionsConfig = if let Some(ro_value) = value.get("real_options") {
        yaml_serde::from_value(ro_value.clone())
            .map_err(|e| ForgeError::Validation(format!("real_options config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'real_options' section found in YAML".to_string(),
        ));
    };

    let engine = RealOptionsEngine::new(config).map_err(ForgeError::Validation)?;
    engine.analyze().map_err(ForgeError::Eval)
}

/// Execute the real-options command - value managerial flexibility
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no `real_options` section exists,
/// the option filter does not match, or the analysis fails.
pub fn real_options(
    c: &mut Console<'_>,
    file: &Path,
    option_filter: Option<&str>,
    compare_npv: bool,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "💎 Forge - Real Options Analysis".bold().green());
    outln!(c, "   File: {}", file.display());
    outln!(c);

    // Parse YAML
    let yaml_content = parser::read_model_text(file)?;

    // Parse real_options config
    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let config: RealOptionsConfig = if let Some(ro_value) = value.get("real_options") {
        yaml_serde::from_value(ro_value.clone())
            .map_err(|e| ForgeError::Validation(format!("real_options config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'real_options' section found in YAML".to_string(),
        ));
    };

    // Display config
    outln!(c, "   {}", format!("Analysis: {}", config.name).bold());
    outln!(
        c,
        "      Method: {}",
        format!("{:?}", config.method).bright_blue()
    );
    outln!(
        c,
        "      Underlying value: {}",
        money(config.underlying.current_value)
    );
    outln!(
        c,
        "      Volatility: {:.1}%",
        config.underlying.volatility * 100.0
    );
    outln!(
        c,
        "      Risk-free rate: {:.1}%",
        config.underlying.risk_free_rate * 100.0
    );
    outln!(
        c,
        "      Time horizon: {} {}",
        config.underlying.time_horizon,
        if (config.underlying.time_horizon - 1.0).abs() < f64::EPSILON {
            "year"
        } else {
            "years"
        }
    );
    outln!(c);

    // Create engine and value options
    let engine = RealOptionsEngine::new(config).map_err(ForgeError::Validation)?;

    if verbose {
        outln!(c, "{}", "🔄 Valuing options...".cyan());
    }

    let result = engine.analyze().map_err(ForgeError::Eval)?;

    // Filter if specific option requested
    if let Some(filter) = option_filter {
        if !result.options.contains_key(filter) {
            return Err(ForgeError::Validation(format!(
                "Option '{}' not found. Available: {:?}",
                filter,
                result.options.keys().collect::<Vec<_>>()
            )));
        }
    }

    // Display results, options in the order the file lists them
    let order: Vec<String> = engine
        .config()
        .options
        .iter()
        .map(|o| o.name.clone())
        .collect();
    print_real_options_results(c, &result, &order, option_filter, compare_npv);

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

    outln!(c, "{}", "✅ Real options analysis complete".bold().green());
    Ok(())
}

/// Print real options analysis results
fn print_real_options_results(
    c: &mut Console<'_>,
    result: &crate::real_options::OptionsResult,
    order: &[String],
    option_filter: Option<&str>,
    compare_npv: bool,
) {
    outln!(c, "{}", "📊 Real Options Results:".bold().green());
    outln!(c);

    match result.traditional_npv {
        Some(npv) => outln!(
            c,
            "   Traditional NPV (invest now, no flexibility): {}",
            money(npv).yellow()
        ),
        None if compare_npv => outln!(
            c,
            "   Traditional NPV: {}",
            "not given (set traditional_npv in real_options)".yellow()
        ),
        None => {},
    }
    outln!(c);

    outln!(c, "   {}", "Options (each valued on its own):".bold());
    for name in order {
        if option_filter.is_some_and(|filter| filter != name) {
            continue;
        }
        let Some(opt_result) = result.options.get(name) else {
            continue;
        };
        outln!(
            c,
            "      {} ({})",
            opt_result.name.bright_blue(),
            format!("{:?}", opt_result.option_type).dimmed(),
        );
        match opt_result.value_added {
            Some(v) => outln!(
                c,
                "         Adds over the traditional NPV: {}",
                money(v).bold().green()
            ),
            None => outln!(
                c,
                "         Adds over the traditional NPV: needs traditional_npv"
            ),
        }
        match opt_result.project_value {
            Some(pv) => outln!(c, "         Project value with this option: {}", money(pv)),
            None => outln!(
                c,
                "         Project value with this option: needs traditional_npv"
            ),
        }
        if let Some(ref trigger) = opt_result.optimal_trigger {
            outln!(c, "         Rule of thumb: exercise when {trigger}");
        }
        if let Some(prob) = opt_result.probability_exercise {
            outln!(c, "         P(exercise): {:.1}%", prob * 100.0);
        }
    }
    outln!(c);

    if let (Some(best), Some(pv)) = (&result.best_option, result.project_value_with_options) {
        outln!(
            c,
            "   Project value with the best option ({}): {}",
            best.bright_blue(),
            money(pv).bold().green()
        );
    }
    if result.options.len() > 1 {
        outln!(
            c,
            "   {}",
            "Option values overlap, so they are compared, not added.".dimmed()
        );
    }

    outln!(c);
    outln!(
        c,
        "   {}: {}",
        "Decision".bold(),
        result.decision.bright_yellow()
    );
    outln!(
        c,
        "   {}: {}",
        "Recommendation".bold(),
        result.recommendation
    );
    outln!(c);
}

#[cfg(test)]
mod tests {

    mod printed_numbers {
        //! The money `forge real-options` prints
        //! follow `forge report`'s rule (Session Prompt part 1, item 5): two decimals
        //! and thousands separators, four significant digits under 1, and money with
        //! the sign before the `$`. JSON and files keep the raw numbers.

        use crate::cli::forge;

        #[test]
        fn real_options_print_money_with_separators() {
            let text = forge(&["real-options", "examples/bagel/wait_option.yaml"]).printed();
            for expected in [
                "invest now, no flexibility): $60,000.00",
                "Adds over the traditional NPV: $4,604.68",
                "Project value with this option: $64,604.68",
                "Underlying value: $240,000.00",
                "exercise when value > $198,000",
                "project value $64,605.",
                "worth $4,605 more than deciding now",
            ] {
                assert!(text.contains(expected), "{expected}\n{text}");
            }
        }
    }

    mod output_file {
        //! `forge real-options -o results.json` wrote YAML into the JSON file;
        //! the other analyses write JSON for a `.json` path and YAML otherwise.

        use crate::cli::forge;

        #[test]
        fn a_json_path_gets_json_and_any_other_yaml() {
            let dir = tempfile::tempdir().unwrap();
            for (name, json) in [("ro.json", true), ("ro.yaml", false)] {
                let path = dir.path().join(name);
                let printed = forge(&[
                    "real-options",
                    "examples/real-options.yaml",
                    "-o",
                    path.to_str().unwrap(),
                ])
                .printed();
                assert!(printed.contains("Results written to"), "{printed}");
                let text = std::fs::read_to_string(&path).unwrap();
                let value: serde_json::Value = if json {
                    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text}"))
                } else {
                    yaml_serde::from_str(&text).unwrap()
                };
                assert_eq!(value["traditional_npv"], 2_000_000.0, "{name}");
                assert!(value["options"]["Double capacity if demand high"]["value"].is_f64());
            }
        }
    }

    mod horizon {
        //! A horizon of one is `1 year`, not `1 years` (the end-to-end pass of
        //! 2026-11-12).

        use crate::cli::forge;

        #[test]
        fn a_horizon_of_one_is_a_year() {
            let text = forge(&["real-options", "examples/bagel/wait_option.yaml"]).printed();
            assert!(text.contains("Time horizon: 1 year\n"), "{text}");
            let text = forge(&["real-options", "examples/real-options.yaml"]).printed();
            assert!(text.contains("Time horizon: 3 years\n"), "{text}");
        }
    }
}
