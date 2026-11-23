//! `forge real-options`: what the choice to wait, expand or abandon is worth.

use super::write_result;
use crate::cli::console::outln;
use crate::cli::Console;
use crate::core::precision::money;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use crate::real_options::{RealOptionsConfig, RealOptionsEngine};
use colored::Colorize;
use std::path::{Path, PathBuf};

/// The `real_options:` section of the model in `file`.
fn load(file: &Path) -> ForgeResult<RealOptionsConfig> {
    let yaml_content = parser::read_model_text(file)?;

    // The section is read from YAML as it is, whatever else the file holds
    let value: yaml_serde::Value = parser::read_yaml(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let Some(ro_value) = value.get("real_options") else {
        return Err(ForgeError::Validation(
            "No 'real_options' section found in YAML".to_string(),
        ));
    };
    yaml_serde::from_value(ro_value.clone())
        .map_err(|e| ForgeError::Validation(format!("real_options config error: {e}")))
}

/// Value real options and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no `real_options` section exists,
/// or the analysis fails.
pub fn real_options_core(file: &Path) -> ForgeResult<crate::real_options::OptionsResult> {
    let engine = RealOptionsEngine::new(load(file)?).map_err(ForgeError::Validation)?;
    Ok(engine.analyze())
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

    let result = real_options_core(file)?;

    // Display config
    outln!(c, "   {}", format!("Analysis: {}", result.name).bold());
    outln!(
        c,
        "      Method: {}",
        format!("{:?}", result.method).bright_blue()
    );
    outln!(
        c,
        "      Underlying value: {}",
        money(result.underlying.current_value)
    );
    outln!(
        c,
        "      Volatility: {:.1}%",
        result.underlying.volatility * 100.0
    );
    outln!(
        c,
        "      Risk-free rate: {:.1}%",
        result.underlying.risk_free_rate * 100.0
    );
    outln!(
        c,
        "      Time horizon: {} {}",
        result.underlying.time_horizon,
        if (result.underlying.time_horizon - 1.0).abs() < f64::EPSILON {
            "year"
        } else {
            "years"
        }
    );
    outln!(c);

    if verbose {
        outln!(c, "{}", "🔄 Valuing options...".cyan());
    }

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
    print_real_options_results(c, &result, option_filter, compare_npv);

    // Write output if specified
    if let Some(output_path) = output_file {
        write_result(c, &result, &output_path)?;
    }

    outln!(c, "{}", "✅ Real options analysis complete".bold().green());
    Ok(())
}

/// Print real options analysis results
fn print_real_options_results(
    c: &mut Console<'_>,
    result: &crate::real_options::OptionsResult,
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
    // The options the file lists, each of which has a result
    let shown = result
        .option_order
        .iter()
        .filter(|name| option_filter.is_none_or(|filter| filter == name.as_str()))
        .filter_map(|name| result.options.get(name));
    for opt_result in shown {
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
                    serde_json::from_str(&text).unwrap()
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

    mod acquisition_example {
        //! `examples/acquisition/halo_option.yaml`: Quillon's Halo platform, which a
        //! DCF values at -40. Cox-Ross-Rubinstein, 200 steps, American, written
        //! from the textbook: S=220, sigma=0.45, r=0.04, q=0.05, T=2. Defer (call,
        //! K=260) = 37.62128284340222; abandon (put, K=90) = 3.0696346363395386.

        use crate::cli::real_options_core;
        use std::path::Path;

        #[test]
        fn the_right_to_wait_turns_halo_from_minus_40_to_plus_37() {
            let r = real_options_core(Path::new("examples/acquisition/halo_option.yaml")).unwrap();
            let defer = r.options["Launch Halo within two years, or never"].value;
            let abandon = r.options["Sell Halo's patents if it flops"].value;
            assert!((defer - 37.621_282_843_402_22).abs() <= 1e-9, "{defer}");
            assert!(
                (abandon - 3.069_634_636_339_538_6).abs() <= 1e-9,
                "{abandon}"
            );
            assert_eq!(r.traditional_npv, Some(-40.0));
        }
    }

    mod command {
        //! `forge real-options` end to end: what it refuses, `--verbose`,
        //! `--compare-npv`, `--option` and `-o`.

        use crate::cli::forge;
        use crate::cli::real_options_core;
        use std::path::Path;

        const HEAD: &str = "_forge_version: \"5.1.0\"\nreal_options:\n  name: Probe\n  method: binomial\n  underlying: {current_value: 100, volatility: 0.3, risk_free_rate: 0.05, time_horizon: 2}\n  binomial_steps: 50\n";
        const WAIT: &str = "    - {type: defer, name: Wait, max_deferral: 1, exercise_cost: 90}\n";
        const SELL: &str = "    - {type: abandon, name: Sell, salvage_value: 60}\n";

        /// A file in a temporary folder
        fn file(text: &str) -> (tempfile::TempDir, String) {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("m.yaml");
            std::fs::write(&path, text).unwrap();
            let path = path.to_str().unwrap().to_string();
            (dir, path)
        }

        /// The analysis with `npv` as its traditional NPV, and `options` listed
        fn analysis(npv: Option<&str>, options: &[&str]) -> (tempfile::TempDir, String) {
            let npv = npv.map_or(String::new(), |n| format!("  traditional_npv: {n}\n"));
            file(&format!("{HEAD}{npv}  options:\n{}", options.concat()))
        }

        #[test]
        fn a_file_a_section_or_a_config_that_does_not_fit_stops_the_command() {
            let (_a, empty) = file("_forge_version: \"5.1.0\"\nx: 1\n");
            let (_b, unknown_key) = file("_forge_version: \"5.1.0\"\nreal_options: {name: X, bogus: 1, underlying: {current_value: 1, volatility: 0.3, risk_free_rate: 0.05, time_horizon: 1}}\n");
            let (_c, no_options) = file(HEAD);
            let (_e, not_yaml) = file("_forge_version: \"5.1.0\"\nreal_options: [1\n");
            let (_d, bad_volatility) = file(&format!(
                "{}  options:\n{WAIT}",
                HEAD.replace("volatility: 0.3", "volatility: -1")
            ));
            for (path, want) in [
                (
                    "/nonexistent/m.yaml",
                    "IO error: No such file or directory (os error 2)",
                ),
                (
                    &empty,
                    "Validation error: No 'real_options' section found in YAML",
                ),
                (
                    &unknown_key,
                    "real_options config error: unknown field `bogus`, expected one of `name`, `method`, `underlying`, `options`, `binomial_steps`, `monte_carlo_iterations`, `seed`, `traditional_npv`",
                ),
                (
                    &not_yaml,
                    "YAML parse error: did not find expected ',' or ']' at line 3 column 1",
                ),
                (&no_options, "At least one option must be defined"),
                (&bad_volatility, "Volatility must be between 0 and 200%"),
            ] {
                let ran = forge(&["real-options", path]);
                assert!(!ran.success(), "{path}");
                let text = ran.both();
                assert!(text.contains(want), "{want}\n{text}");
                assert!(!text.contains("Real options analysis complete"), "{text}");
            }
            // The same refusals without printing
            let err = real_options_core(Path::new(&empty))
                .unwrap_err()
                .to_string();
            assert_eq!(
                err,
                "Validation error: No 'real_options' section found in YAML"
            );
            let err = real_options_core(Path::new(&no_options))
                .unwrap_err()
                .to_string();
            assert_eq!(err, "Validation error: At least one option must be defined");
        }

        #[test]
        fn an_option_the_file_does_not_list_is_refused_and_one_that_it_does_is_alone() {
            let (_dir, path) = analysis(Some("5"), &[WAIT, SELL]);
            let ran = forge(&["real-options", &path, "--option", "nope"]);
            assert!(!ran.success());
            let text = ran.both();
            assert!(
                text.contains("Option 'nope' not found. Available: [\"Sell\", \"Wait\"]"),
                "{text}"
            );
            let text = forge(&["real-options", &path, "--option", "Sell"]).printed();
            assert!(text.contains("Sell (Abandon)"), "{text}");
            assert!(!text.contains("Wait (Defer)"), "{text}");
        }

        #[test]
        fn without_a_traditional_npv_the_options_that_need_one_say_so() {
            let (_dir, path) = analysis(None, &[WAIT, SELL]);
            let text = forge(&["real-options", &path]).printed();
            for line in [
                "      Wait (Defer)\n         Adds over the traditional NPV: needs traditional_npv\n         Project value with this option: $19.72\n",
                "      Sell (Abandon)\n         Adds over the traditional NPV: $1.09\n         Project value with this option: needs traditional_npv\n",
            ] {
                assert!(text.contains(line), "{line}\n{text}");
            }
            // Asked for, the missing NPV is a line of its own; otherwise nothing is said
            assert!(!text.contains("Traditional NPV"), "{text}");
            let text = forge(&["real-options", &path, "--compare-npv"]).printed();
            assert!(
                text.contains(
                    "   Traditional NPV: not given (set traditional_npv in real_options)\n"
                ),
                "{text}"
            );
        }

        #[test]
        fn an_option_with_no_rule_of_thumb_has_no_line_for_one() {
            let swap = "    - {type: switch, name: Swap, exercise_cost: 10}\n";
            let (_dir, path) = analysis(Some("5"), &[swap]);
            let text = forge(&["real-options", &path]).printed();
            assert!(text.contains("      Swap (Switch)\n"), "{text}");
            assert!(!text.contains("Rule of thumb"), "{text}");
        }

        #[test]
        fn verbose_says_when_it_values_the_options() {
            let (_dir, path) = analysis(Some("5"), &[WAIT]);
            let text = forge(&["real-options", &path, "--verbose"]).printed();
            assert!(
                text.contains("\n🔄 Valuing options...\n📊 Real Options Results:\n"),
                "{text}"
            );
            assert!(!forge(&["real-options", &path])
                .printed()
                .contains("Valuing"));
        }

        #[test]
        fn the_result_goes_to_a_file_or_the_command_says_why_it_could_not() {
            let (dir, path) = analysis(Some("5"), &[WAIT]);
            let out = dir.path().join("o.json");
            let text = forge(&["real-options", &path, "-o", out.to_str().unwrap()]).printed();
            assert!(
                text.contains(&format!("💾 Results written to {}\n", out.display())),
                "{text}"
            );
            let ran = forge(&["real-options", &path, "-o", "/nonexistent/o.json"]);
            assert!(!ran.success());
            let text = ran.both();
            assert!(
                text.contains("Error: IO error: No such file or directory (os error 2)"),
                "{text}"
            );
            assert!(!text.contains("Results written"), "{text}");
        }
    }

    /// The method and the file's order of the options are in the result
    /// (T1-CLI-FROM-CORES).
    #[test]
    fn the_result_has_the_method_and_the_order_of_the_file() {
        let result =
            super::real_options_core(std::path::Path::new("examples/real-options.yaml")).unwrap();
        assert_eq!(
            result.method,
            crate::real_options::ValuationMethod::Binomial
        );
        assert_eq!(
            result.option_order,
            [
                "Wait up to 2 years before building",
                "Double capacity if demand high",
                "Sell assets for salvage value"
            ]
        );
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["method"], "binomial");
    }
}
