//! `forge simulate --seek`: the value of one input at which an output meets
//! a threshold with a given probability ("the price at which the year makes
//! money in 80% of runs").
//!
//! Every simulation uses the same configuration and seed, so the uncertain
//! inputs are drawn identically for every value tried, and the probability
//! is a step function of the input. Bisection finds the step where it first
//! reaches the target.

use super::simulate::{prepare, run_model, SimulateOptions};
use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::monte_carlo::statistics::{evaluate_threshold, parse_threshold};
use serde::Serialize;
use std::path::Path;

/// What to seek: `P(<output> <op> <value>) = <probability>`.
#[derive(Debug, Clone, PartialEq)]
pub struct SeekGoal {
    /// The output (a scalar, or one row of a column: `months.profit (row 1)`)
    pub output: String,
    /// The threshold, as in `thresholds:` (`"> 0"`)
    pub threshold: String,
    /// The probability to reach, between 0 and 1
    pub probability: f64,
}

impl SeekGoal {
    /// Read `P(year.profit > 0) = 0.8` (a probability may be written `80%`).
    ///
    /// # Errors
    ///
    /// Returns an error that shows the expected form.
    pub fn parse(text: &str) -> Result<Self, String> {
        let form =
            || format!("--seek '{text}': write it as \"P(year.profit > 0) = 0.8\" (or = 80%)");
        let (inner, target) = text.rsplit_once('=').ok_or_else(form)?;
        let inner = inner.trim();
        let inner = inner
            .strip_prefix("P(")
            .or_else(|| inner.strip_prefix("p("))
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(form)?;
        let target = target.trim();
        let probability = target
            .strip_suffix('%')
            .map_or_else(
                || target.parse::<f64>(),
                |percent| percent.trim().parse::<f64>().map(|p| p / 100.0),
            )
            .map_err(|_| form())?;
        if !(probability > 0.0 && probability < 1.0) {
            return Err(format!(
                "--seek '{text}': the probability must be between 0 and 1 (0% and 100%), not included"
            ));
        }
        let split = inner.find(['>', '<', '=']).ok_or_else(form)?;
        let (output, threshold) = inner.split_at(split);
        let output = output.trim().to_string();
        let threshold = threshold.trim().to_string();
        parse_threshold(&threshold).map_err(|e| format!("--seek '{text}': {e}"))?;
        if output.is_empty() {
            return Err(form());
        }
        Ok(Self {
            output,
            threshold,
            probability,
        })
    }
}

/// The answer.
#[derive(Debug, Clone, Serialize)]
pub struct SeekResult {
    /// The input varied
    pub input: String,
    /// The value found
    pub value: f64,
    /// The probability at that value
    pub probability: f64,
    /// The goal, as given
    pub goal: String,
    /// The probability at each end of the range searched
    pub probability_at_min: f64,
    pub probability_at_max: f64,
    /// Simulations run
    pub simulations: usize,
    /// Iterations in each
    pub iterations: usize,
}

/// Find the value of `input` in `[min, max]` at which `goal` is first met.
///
/// # Errors
///
/// Returns an error if the model or settings are invalid, `input` is not an
/// input of the model (a scalar without a formula), the output is not
/// simulated, or the probability is not reached within the range.
#[allow(clippy::too_many_arguments)] // the CLI's arguments, one each
pub fn probability_seek_core(
    file: &Path,
    options: SimulateOptions<'_>,
    goal: &SeekGoal,
    input: &str,
    min: f64,
    max: f64,
) -> ForgeResult<SeekResult> {
    if min >= max || !min.is_finite() || !max.is_finite() {
        return Err(ForgeError::Validation(format!(
            "--min ({min}) must be below --max ({max})"
        )));
    }
    let (mut config, model) = prepare(file, options)?;
    // Simulate only the output the goal is about
    let output = config
        .outputs
        .iter()
        .find(|o| o.variable == goal.output)
        .cloned()
        .ok_or_else(|| {
            ForgeError::Validation(format!(
                "--seek: '{}' is not an output of monte_carlo (outputs: {})",
                goal.output,
                config
                    .outputs
                    .iter()
                    .map(|o| o.variable.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;
    config.outputs = vec![output];

    let resolved = model
        .resolve_scalar_name(input)
        .map_err(ForgeError::Validation)?;
    if model.scalars[&resolved].formula.is_some() {
        return Err(ForgeError::Validation(format!(
            "--vary {resolved}: it has a formula; vary an input (a scalar with a plain value)"
        )));
    }

    let (op, threshold) = parse_threshold(&goal.threshold).map_err(ForgeError::Validation)?;
    let plan = crate::core::array_calculator::CalcPlan::new();
    let mut simulations = 0;
    let mut probability_at = |value: f64| -> ForgeResult<f64> {
        let mut trial = model.clone();
        trial
            .override_scalar(&resolved, Some(value), None)
            .map_err(ForgeError::Validation)?;
        let result = run_model(&config, &trial, &plan, true)?;
        simulations += 1;
        Ok(evaluate_threshold(
            &result.outputs[&goal.output].samples,
            &op,
            threshold,
        ))
    };

    let at_min = probability_at(min)?;
    let at_max = probability_at(max)?;
    let reaches = |p: f64| p >= goal.probability;
    let (mut lo, mut hi, rising) = match (reaches(at_min), reaches(at_max)) {
        (false, true) => (min, max, true),
        (true, false) => (min, max, false),
        _ => {
            return Err(ForgeError::Validation(format!(
                "{} is {:.2}% at {min} and {:.2}% at {max}: the goal {:.2}% is not crossed in that range",
                format_args!("P({} {})", goal.output, goal.threshold),
                at_min * 100.0,
                at_max * 100.0,
                goal.probability * 100.0
            )))
        },
    };
    // The probability changes in steps; narrow down to where it first
    // reaches the goal, to a millionth of the range
    let mut at_answer = if rising { at_max } else { at_min };
    let tolerance = (max - min) * 1e-6;
    // 60 halvings reach any tolerance a f64 range can hold
    for _ in 0..60 {
        if hi - lo <= tolerance {
            break;
        }
        let mid = lo.midpoint(hi);
        let p = probability_at(mid)?;
        if reaches(p) == rising {
            hi = mid;
            if rising {
                at_answer = p;
            }
        } else {
            lo = mid;
            if !rising {
                at_answer = p;
            }
        }
    }
    let value = if rising { hi } else { lo };
    Ok(SeekResult {
        input: resolved,
        value,
        probability: at_answer,
        goal: format!(
            "P({} {}) = {}",
            goal.output, goal.threshold, goal.probability
        ),
        probability_at_min: at_min,
        probability_at_max: at_max,
        simulations,
        iterations: config.iterations,
    })
}

/// Execute `forge simulate --seek`.
///
/// # Errors
///
/// Returns an error if the goal cannot be read or [`probability_seek_core`] fails.
pub fn probability_seek(
    c: &mut Console<'_>,
    file: &Path,
    options: SimulateOptions<'_>,
    seek: &str,
    vary: &str,
    min: f64,
    max: f64,
) -> ForgeResult<()> {
    use colored::Colorize;
    let goal = SeekGoal::parse(seek).map_err(ForgeError::Validation)?;
    outln!(c, "{}", "🎯 Forge - Probability Seek".bold().green());
    outln!(c, "   File: {}", file.display());
    outln!(
        c,
        "   Goal: P({} {}) = {:.2}%",
        goal.output,
        goal.threshold,
        goal.probability * 100.0
    );
    outln!(c, "   Vary: {vary} from {min} to {max}");
    outln!(c);
    let result = probability_seek_core(file, options, &goal, vary, min, max)?;
    outln!(
        c,
        "   {} = {}",
        result.input.bright_blue().bold(),
        super::display_number(result.value).bold()
    );
    outln!(
        c,
        "   P({} {}) there: {:.2}%  ({:.2}% at {min}, {:.2}% at {max})",
        goal.output,
        goal.threshold,
        result.probability * 100.0,
        result.probability_at_min * 100.0,
        result.probability_at_max * 100.0
    );
    outln!(
        c,
        "   {} simulations of {} iterations, the same draws each time",
        result.simulations,
        result.iterations
    );
    outln!(c);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goals_are_read_in_both_forms() {
        let goal = SeekGoal::parse("P(year.profit > 0) = 0.8").unwrap();
        assert_eq!(
            goal,
            SeekGoal {
                output: "year.profit".to_string(),
                threshold: "> 0".to_string(),
                probability: 0.8
            }
        );
        let goal = SeekGoal::parse("P(months.profit (row 1) >= -500) = 95%").unwrap();
        assert_eq!(goal.output, "months.profit (row 1)");
        assert_eq!(goal.threshold, ">= -500");
        assert!((goal.probability - 0.95).abs() < 1e-12);
        for bad in [
            "year.profit > 0",
            "P(year.profit) = 0.8",
            "P(x > 0) = 1.5",
            "P(x > a) = 0.5",
        ] {
            assert!(SeekGoal::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn goals_that_are_not_written_as_a_probability_say_how_to() {
        let form = |text: &str| {
            format!("--seek '{text}': write it as \"P(year.profit > 0) = 0.8\" (or = 80%)")
        };
        for bad in [
            "P(> 0) = 0.8",
            "x > 0 = 0.8",
            "P(x > 0) = abc",
            "P(x > 0) = 5x%",
        ] {
            assert_eq!(SeekGoal::parse(bad).unwrap_err(), form(bad), "{bad}");
        }
        // A lower case p is as good as P
        let lower = SeekGoal::parse("p(x > 0) = 50%").unwrap();
        assert!((lower.probability - 0.5).abs() <= 1e-12);
    }

    mod failures {
        //! Each way a seek can fail says what it was given.

        use super::super::{probability_seek_core, SeekGoal};
        use crate::cli::{forge, SimulateOptions};
        use std::path::PathBuf;

        fn model(extra: &str) -> (tempfile::TempDir, PathBuf) {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("m.yaml");
            let text = format!(
                "_forge_version: \"5.4.0\"\ns:\n  x: 1\n  d: \"2026-01-31\"\n  noise: {{value: 0, formula: \"=MC.Normal(0, 1)\"}}\n  out: {{formula: \"=x + noise{extra}\"}}\nmonte_carlo:\n  iterations: 400\n  seed: 5\n  outputs:\n    - variable: s.out\n      thresholds: [\"> 0\"]\n"
            );
            std::fs::write(&path, text).unwrap();
            (dir, path)
        }

        fn seek(path: &std::path::Path, goal: &SeekGoal, input: &str, range: (f64, f64)) -> String {
            probability_seek_core(
                path,
                SimulateOptions::default(),
                goal,
                input,
                range.0,
                range.1,
            )
            .unwrap_err()
            .to_string()
        }

        fn goal() -> SeekGoal {
            SeekGoal::parse("P(s.out > 0) = 0.75").unwrap()
        }

        #[test]
        fn the_range_the_input_the_output_and_the_threshold_must_be_right() {
            let (_dir, path) = model("");
            assert_eq!(
                seek(&path, &goal(), "s.x", (2.0, 0.0)),
                "Validation error: --min (2) must be below --max (0)"
            );
            assert_eq!(
                seek(&path, &goal(), "s.x", (0.0, f64::INFINITY)),
                "Validation error: --min (0) must be below --max (inf)"
            );
            assert_eq!(
                seek(&path, &goal(), "s.nope", (0.0, 2.0)),
                "Validation error: Scenario override 's.nope' does not name a scalar in the model. Did you mean 's.noise'?"
            );
            assert_eq!(
                seek(&path, &goal(), "s.d", (0.0, 2.0)),
                "Validation error: 's.d' is a date (2026-01-31), so it is set to a date, not a number (ADR-060)"
            );
            let other = SeekGoal::parse("P(s.zzz > 0) = 0.75").unwrap();
            assert_eq!(
                seek(&path, &other, "s.x", (0.0, 2.0)),
                "Validation error: --seek: 's.zzz' is not an output of monte_carlo (outputs: s.out)"
            );
            // The fields are public: a threshold that was not read by `parse`
            let bad = SeekGoal {
                threshold: "~ 3".to_string(),
                ..goal()
            };
            assert_eq!(
                seek(&path, &bad, "s.x", (0.0, 2.0)),
                "Validation error: Invalid threshold format: ~ 3. Use '> 0', '< 100', etc."
            );
            assert_eq!(
                seek(
                    &path.with_file_name("none.yaml"),
                    &goal(),
                    "s.x",
                    (0.0, 2.0)
                ),
                "IO error: No such file or directory (os error 2)"
            );
        }

        /// The output fails where the input is at the top of the range, or at
        /// the first value the search tries between the ends: each says so.
        #[test]
        fn a_value_where_the_output_fails_stops_the_seek_there() {
            // x - 2 is 0 at the top of the range, x - 1 at its middle
            for divisor in ["x - 2", "x - 1"] {
                let (_dir, path) = model(&format!(" + 0 * (1 / ({divisor}))"));
                let err = seek(&path, &goal(), "s.x", (0.0, 2.0));
                assert!(err.contains("Division by zero"), "{divisor}: {err}");
            }
        }

        #[test]
        fn the_command_says_what_it_could_not_read() {
            let (_dir, path) = model("");
            let ran = forge(&[
                "simulate",
                path.to_str().unwrap(),
                "--seek",
                "oops",
                "--vary",
                "s.x",
                "--min",
                "0",
                "--max",
                "2",
            ]);
            assert_eq!(ran.code, 1);
            assert_eq!(
                ran.err,
                "Error: Validation error: --seek 'oops': write it as \"P(year.profit > 0) = 0.8\" (or = 80%)\n"
            );
        }
    }

    mod printed_numbers {
        //! The solution `forge simulate --seek` prints
        //! follow `forge report`'s rule (Session Prompt part 1, item 5): two decimals
        //! and thousands separators, four significant digits under 1, and money with
        //! the sign before the `$`. JSON and files keep the raw numbers.

        use crate::cli::forge;

        #[test]
        fn a_probability_seek_prints_the_solution_with_two_decimals() {
            let text = forge(&[
                "simulate",
                "examples/bagel/bagel_simulate.yaml",
                "--seek",
                "P(year.profit > 0) = 0.8",
                "--vary",
                "price_per_dozen",
                "--min",
                "14",
                "--max",
                "24",
            ])
            .printed();
            assert!(
                text.contains("assumptions.price_per_dozen = 18.32\n"),
                "{text}"
            );
            assert!(!text.contains("18.324083"), "{text}");
        }
    }

    mod unused_formulas {
        //! Roadmap T1-SEEK-UNUSED-FORMULA: a seek that moves an input to a value
        //! where a formula its output does not use fails (`=1 / x` at 0) still
        //! gives its answer; `forge simulate` itself, which calculates every
        //! formula, still reports that failure.

        use super::super::{probability_seek_core, SeekGoal};
        use crate::cli::{simulate_core, SimulateOptions};

        const MODEL: &str = "_forge_version: \"5.4.0\"\ns:\n  x: 1\n  noise: {value: 0, formula: \"=MC.Normal(0, 1)\"}\n  out: {formula: \"=x + noise\"}\n  ratio: {formula: \"=1 / x\"}\nmonte_carlo:\n  iterations: 400\n  seed: 5\n  outputs:\n    - variable: s.out\n      thresholds: [\"> 0\"]\n";

        fn file() -> (tempfile::TempDir, std::path::PathBuf) {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("m.yaml");
            std::fs::write(&path, MODEL).unwrap();
            (dir, path)
        }

        #[test]
        fn the_seek_passes_through_a_value_the_output_does_not_depend_on() {
            let (_dir, path) = file();
            // x runs from 0, where 1 / x fails, to 2; out > 0 in 75% of futures
            // when x is 0.674 (the 75th percentile of a standard normal)
            let goal = SeekGoal::parse("P(s.out > 0) = 0.75").unwrap();
            let r =
                probability_seek_core(&path, SimulateOptions::default(), &goal, "s.x", 0.0, 2.0)
                    .unwrap();
            assert!((r.value - 0.674).abs() <= 0.05, "{}", r.value);
            assert!((r.probability - 0.75).abs() <= 0.02, "{}", r.probability);
            assert!(r.probability_at_min < 0.55 && r.probability_at_max > 0.95);
        }

        #[test]
        fn a_row_of_a_table_column_is_an_output_too() {
            let (dir, _) = file();
            let path = dir.path().join("t.yaml");
            let text = MODEL
                .replace(
                    "monte_carlo:",
                    "t:\n  n: [1, 2]\n  y: \"=n - 1 + x + noise\"\nmonte_carlo:",
                )
                .replace("s.out", "t.y");
            std::fs::write(&path, text).unwrap();
            // Row 1 is x + noise, the same as s.out: the answer is the same, and
            // 1 / x at 0 fails there too without stopping it
            let goal = SeekGoal::parse("P(t.y (row 1) > 0) = 0.75").unwrap();
            let r =
                probability_seek_core(&path, SimulateOptions::default(), &goal, "s.x", 0.0, 2.0)
                    .unwrap();
            assert!((r.value - 0.674).abs() <= 0.05, "{}", r.value);
        }

        #[test]
        fn a_failure_the_output_reads_still_stops_the_seek_and_simulate_reports_any() {
            let (dir, path) = file();
            let text = MODEL.replace("=x + noise", "=x + noise + 1 / x");
            std::fs::write(&path, text).unwrap();
            let goal = SeekGoal::parse("P(s.out > 0) = 0.75").unwrap();
            let err =
                probability_seek_core(&path, SimulateOptions::default(), &goal, "s.x", 0.0, 2.0)
                    .unwrap_err()
                    .to_string();
            assert!(err.contains("Division by zero"), "{err}");
            // Plain simulate calculates every formula, so a failure anywhere is reported
            let other = dir.path().join("n.yaml");
            std::fs::write(&other, MODEL.replace("x: 1", "x: 0")).unwrap();
            let err = simulate_core(&other, SimulateOptions::default())
                .unwrap_err()
                .to_string();
            assert!(err.contains("Division by zero"), "{err}");
        }

        #[test]
        fn the_acquisition_seek_starts_at_a_premium_of_zero() {
            let goal = SeekGoal::parse("P(valuation.value_created > 0) = 0.75").unwrap();
            let options = SimulateOptions {
                iterations: Some(200),
                ..SimulateOptions::default()
            };
            let seek = |min: f64| {
                probability_seek_core(
                    std::path::Path::new("examples/acquisition/deal_risk.yaml"),
                    options,
                    &goal,
                    "offer.premium",
                    min,
                    0.6,
                )
                .unwrap()
            };
            let from_zero = seek(0.0);
            let from_a_tenth = seek(0.1);
            assert!((from_zero.value - from_a_tenth.value).abs() <= 1e-3);
            assert!((from_zero.probability_at_min - 1.0).abs() <= 0.01);
        }
    }

    mod acquisition_example {
        //! `examples/acquisition/deal_risk.yaml`: the highest premium at which the
        //! deal creates value in three futures of four. deal.yaml's walk-away
        //! premium is 0.4834 (`goal_seek.rs`, `acquisition_example`); under
        //! uncertainty it is lower. The README quotes 10,000 runs (0.3424); this
        //! runs 1,000 of the same draws, so the answer moves by sampling noise.

        use super::super::{probability_seek_core, SeekGoal};
        use crate::cli::SimulateOptions;
        use std::path::Path;

        #[test]
        fn three_futures_in_four_allow_a_smaller_premium_than_the_plan() {
            let goal = SeekGoal::parse("P(valuation.value_created > 0) = 0.75").unwrap();
            let options = SimulateOptions {
                iterations: Some(1000),
                ..SimulateOptions::default()
            };
            let r = probability_seek_core(
                Path::new("examples/acquisition/deal_risk.yaml"),
                options,
                &goal,
                "offer.premium",
                0.1,
                0.6,
            )
            .unwrap();
            assert!((r.value - 0.342_4).abs() <= 0.02, "{}", r.value);
            assert!((r.probability - 0.75).abs() <= 0.01, "{}", r.probability);
            assert!(r.value < 0.483_4);
        }
    }
}
