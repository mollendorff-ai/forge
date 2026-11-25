//! `forge goal-seek` and `forge break-even`: the input that brings an output
//! to a value, or to zero, by bisection.

use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use colored::Colorize;
use std::path::Path;

use super::sensitivity::calculate_with_override;
use crate::cli::commands::display_number;

/// Run goal-seek and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, variables are not found,
/// or no solution exists in the search range.
pub fn goal_seek_core(
    file: &Path,
    target: &str,
    value: f64,
    vary: &str,
    bounds: (Option<f64>, Option<f64>),
    tolerance: f64,
) -> ForgeResult<crate::cli::commands::results::GoalSeekResult> {
    let (min, max) = bounds;
    let base_model = parser::parse_model(file)?;
    let vary = &crate::cli::commands::names::scalar_name(&base_model, vary, false)?;
    let target = &crate::cli::commands::names::scalar_name(&base_model, target, true)?;

    let current_value = crate::cli::commands::scalar(&base_model, vary)
        .and_then(|s| s.value)
        .unwrap_or(1.0);

    let lower = min.unwrap_or_else(|| {
        if current_value > 0.0 {
            current_value * 0.01
        } else if current_value < 0.0 {
            current_value * 100.0
        } else {
            -1000.0
        }
    });
    let upper = max.unwrap_or(if current_value > 0.0 {
        current_value * 100.0
    } else if current_value < 0.0 {
        current_value * 0.01
    } else {
        1000.0
    });

    // The output at each end of the interval, as calculated
    let y_low = calculate_with_override(&base_model, vary, lower, target)?;
    let y_high = calculate_with_override(&base_model, vary, upper, target)?;
    let ((mut low, mut y_low), (mut high, mut y_high)) = if (y_low - value) * (y_high - value) > 0.0
    {
        expand_search_range_quiet(&base_model, vary, target, value, lower, upper)?
    } else {
        ((lower, y_low), (upper, y_high))
    };

    // Bisect until the output is within `tolerance` of the target, or the
    // interval cannot be split any further. `tolerance` is about the output,
    // so it is also what decides `converged`. The answer is the last point
    // calculated: the midpoint, or the end the interval has shrunk to.
    let max_iterations = 200;
    let (mut mid, mut final_value) = (low, y_low);
    let mut iteration = 0;

    while iteration < max_iterations {
        let m = f64::midpoint(low, high);
        // No float left between low and high: the midpoint is one of them
        if m <= low || m >= high {
            (mid, final_value) = if m <= low {
                (low, y_low)
            } else {
                (high, y_high)
            };
            break;
        }
        let y_mid = calculate_with_override(&base_model, vary, m, target)?;
        iteration += 1;
        (mid, final_value) = (m, y_mid);

        if (y_mid - value).abs() <= tolerance {
            break;
        }

        if (y_low - value) * (y_mid - value) < 0.0 {
            (high, y_high) = (m, y_mid);
        } else {
            (low, y_low) = (m, y_mid);
        }
    }

    let error = (final_value - value).abs();

    Ok(crate::cli::commands::results::GoalSeekResult {
        vary: vary.clone(),
        target: target.clone(),
        target_value: value,
        solution: mid,
        achieved: final_value,
        error,
        iterations: iteration,
        converged: error <= tolerance,
    })
}

/// Widen the interval 10, 100 and then 1,000 times, away from zero at each
/// end, until the output crosses the target inside it: the two ends, each
/// with its output.
fn expand_search_range_quiet(
    base_model: &crate::types::ParsedModel,
    vary: &str,
    target: &str,
    value: f64,
    lower: f64,
    upper: f64,
) -> ForgeResult<((f64, f64), (f64, f64))> {
    for factor in [10.0, 100.0, 1000.0] {
        let exp_low = if lower > 0.0 {
            lower / factor
        } else {
            lower * factor
        };
        let exp_high = if upper > 0.0 {
            upper * factor
        } else {
            upper / factor
        };

        let y_low = calculate_with_override(base_model, vary, exp_low, target)?;
        let y_high = calculate_with_override(base_model, vary, exp_high, target)?;

        if (y_low - value) * (y_high - value) <= 0.0 {
            return Ok(((exp_low, y_low), (exp_high, y_high)));
        }
    }

    Err(ForgeError::Validation(format!(
        "No solution found in search range. The target value {value} may not be achievable by varying '{vary}'."
    )))
}

/// Execute the goal-seek command
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, variables are not found,
/// or no solution exists in the search range.
pub fn goal_seek(
    c: &mut Console<'_>,
    file: &Path,
    (target, value): (&str, f64),
    vary: &str,
    bounds: (Option<f64>, Option<f64>),
    tolerance: f64,
    verbose: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "🔥 Forge - Goal Seek".bold().green());
    outln!(c, "   File: {}", file.display());
    outln!(c, "   Target: {} = {}", target.bright_blue(), value);
    outln!(c, "   Vary: {}", vary.bright_yellow());
    outln!(c, "   Tolerance: {tolerance}\n");
    solve(c, file, (target, value), vary, bounds, tolerance, verbose)
}

/// Find the input, then print the solution (the part goal-seek and
/// break-even share, after their own headers).
fn solve(
    c: &mut Console<'_>,
    file: &Path,
    (target, value): (&str, f64),
    vary: &str,
    bounds: (Option<f64>, Option<f64>),
    tolerance: f64,
    verbose: bool,
) -> ForgeResult<()> {
    let result = goal_seek_core(file, target, value, vary, bounds, tolerance)?;
    if verbose {
        outln!(
            c,
            "   Bisection stopped after {} evaluations, error {}",
            result.iterations,
            result.error
        );
    }
    print_goal_seek_result(c, (vary, target), &result, tolerance);
    Ok(())
}

/// Print goal-seek result summary, with the names as they were asked for
fn print_goal_seek_result(
    c: &mut Console<'_>,
    (vary, target): (&str, &str),
    result: &crate::cli::commands::results::GoalSeekResult,
    tolerance: f64,
) {
    let iteration = result.iterations;
    outln!(c, "{}", "─".repeat(50));
    outln!(
        c,
        "{}",
        format!("🎯 Solution found in {iteration} iterations:")
            .bold()
            .green()
    );
    outln!(
        c,
        "   {} = {} → {} = {}",
        vary.bright_yellow().bold(),
        solution_text(result.solution).bold().green(),
        target.bright_blue(),
        display_number(result.achieved).green()
    );

    if result.converged {
        outln!(c, "   {} Within tolerance", "✅".green());
    } else {
        outln!(
            c,
            "   {} Error: {} (tolerance: {})",
            "⚠️".yellow(),
            result.error,
            tolerance
        );
    }
    outln!(c, "{}", "─".repeat(50));
}

/// The input goal-seek found, as a report shows a number, then every digit
/// Forge keeps when the two differ: `15.97 (15.966607933396)`. The two
/// decimals read well, and the digits are the input that gives the result
/// printed beside it: 15.97 makes the bagel shop's profit 79.88, not 0.
fn solution_text(solution: f64) -> String {
    let shown = display_number(solution);
    let exact = crate::core::precision::to_15_digits(solution);
    if shown.replace(',', "").parse::<f64>() == Ok(exact) {
        shown
    } else {
        format!("{shown} ({exact})")
    }
}

/// Execute the break-even command
///
/// # Errors
///
/// Returns an error if goal-seek fails to find a zero-crossing point.
pub fn break_even(
    c: &mut Console<'_>,
    file: &Path,
    output: &str,
    vary: &str,
    min: Option<f64>,
    max: Option<f64>,
    verbose: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "🔥 Forge - Break-Even Analysis".bold().green());
    outln!(c, "   File: {}", file.display());
    outln!(c, "   Finding where {} = 0", output.bright_blue());
    outln!(c, "   Vary: {}", vary.bright_yellow());
    outln!(c, "   Tolerance: {BREAK_EVEN_TOLERANCE}\n");

    // Break-even is goal-seek with value = 0
    solve(
        c,
        file,
        (output, 0.0),
        vary,
        (min, max),
        BREAK_EVEN_TOLERANCE,
        verbose,
    )
}

/// How close to zero the output must be for `forge break-even`.
const BREAK_EVEN_TOLERANCE: f64 = 0.0001;

#[cfg(test)]
mod tests {

    mod small_models {
        //! One input `x` and one output `y`, so the answer is known by hand:
        //! `y = x` reaches 500 at x = 500.

        use super::super::{goal_seek, goal_seek_core};
        use crate::cli::Console;
        use std::path::{Path, PathBuf};

        fn model(dir: &Path, x: &str, formula: &str) -> PathBuf {
            let path = dir.join("m.yaml");
            std::fs::write(
                &path,
                format!(
                    "_forge_version: \"5.0.0\"\nx:\n  value: {x}\ny:\n  value: 0\n  formula: \"{formula}\"\n"
                ),
            )
            .unwrap();
            path
        }

        /// `y` at `value`, varying `x` from its value, with tolerance 1e-9
        fn seek(x: &str, formula: &str, value: f64) -> Result<(f64, f64, bool), String> {
            let dir = tempfile::tempdir().unwrap();
            let path = model(dir.path(), x, formula);
            goal_seek_core(&path, "y", value, "x", (None, None), 1e-9)
                .map(|r| (r.solution, r.achieved, r.converged))
                .map_err(|e| e.to_string())
        }

        /// The search starts from x / 100 to x * 100 (x = 1: 0.01 to 100),
        /// from x * 100 to x / 100 for a negative x, and from -1,000 to 1,000
        /// for x = 0; it widens 10, 100 and 1,000 times away from zero until
        /// the target is inside.
        #[test]
        fn the_search_widens_from_the_input_until_the_target_is_inside() {
            for (x, value) in [("1", 500.0), ("-1", -500.0), ("0", 7.0), ("1", 50.0)] {
                let (solution, achieved, converged) = seek(x, "=x", value).unwrap();
                assert_eq!(
                    (
                        (solution - value).abs() <= 1e-9,
                        (achieved - value).abs() <= 1e-9,
                        converged
                    ),
                    (true, true, true),
                    "x = {x}, target {value}: {solution}, {achieved}"
                );
            }
        }

        #[test]
        fn a_target_out_of_reach_is_refused() {
            assert_eq!(
                seek("1", "=x * x + 1", 0.0),
                Err("Validation error: No solution found in search range. The target value 0 may not be achievable by varying 'x'.".to_string())
            );
        }

        /// The output cannot be calculated at an end of the first range (x =
        /// 0.01 or 100), at a widened end (x = 10,000, or -10,000 for a negative x),
        /// or at the first midpoint (50.005): the search stops with that error.
        #[test]
        fn an_output_that_cannot_be_calculated_stops_the_search() {
            for (x, formula, value) in [
                ("1", "=SQRT(x - 1)", 5.0),
                ("1", "=SQRT(50 - x)", 5.0),
                ("1", "=SQRT(5000 - x)", 100.0),
                ("-1", "=SQRT(x + 5000)", 100.0),
                ("1", "=IF(AND(x > 49, x < 51), SQRT(-1), x - 75)", 0.0),
            ] {
                assert_eq!(
                    seek(x, formula, value),
                    Err(format!(
                        "Formula evaluation error: y: SQRT of negative number (formula: {formula})"
                    )),
                    "{formula}"
                );
            }
        }

        /// A step from -1 to 1 never comes within the tolerance of 0: the
        /// search closes in on the step until no float is left between the
        /// ends, and says it did not converge. The last midpoint is one end
        /// or the other (the midpoint of two neighbouring floats rounds to the
        /// even one): the top one for a step at 50, where the output is 1,
        /// and the bottom one for a step at 10, where it is -1. (Forge compares
        /// to 15 significant digits, as Excel does (ADR-065), so `x > 10` is
        /// false just above 10.)
        #[test]
        fn a_step_the_target_falls_inside_does_not_converge() {
            for (step, output) in [(50.0, 1.0), (10.0, -1.0)] {
                let (solution, achieved, converged) =
                    seek("1", &format!("=IF(x > {step}, 1, -1)"), 0.0).unwrap();
                assert_eq!(
                    (
                        solution > step && solution - step < 1e-12,
                        achieved,
                        converged
                    ),
                    (true, output, false),
                    "{step}: {solution}"
                );
            }
        }

        /// `forge goal-seek` prints the error beside the tolerance when it is
        /// over it, and with `--verbose` how the bisection stopped
        #[test]
        fn goal_seek_prints_an_error_over_the_tolerance_and_with_verbose_the_steps() {
            let dir = tempfile::tempdir().unwrap();
            let path = model(dir.path(), "1", "=IF(x > 50, 1, -1)");
            let mut out = Vec::new();
            let mut err = Vec::new();
            let mut input = std::io::empty();
            let mut console = Console::new(&mut out, &mut err, &mut input);
            goal_seek(
                &mut console,
                &path,
                ("y", 0.0),
                "x",
                (None, None),
                0.0001,
                true,
            )
            .unwrap();
            let text = String::from_utf8(out).unwrap();
            let after_file = text.split_once("m.yaml\n").unwrap().1;
            assert_eq!(
                after_file,
                format!(
                    "   Target: y = 0\n   Vary: x\n   Tolerance: 0.0001\n\n   Bisection stopped after 54 evaluations, error 1\n{line}\n🎯 Solution found in 54 iterations:\n   x = 50.00 (50.0000000000001) → y = 1.00\n   ⚠\u{fe0f} Error: 1 (tolerance: 0.0001)\n{line}\n",
                    line = "─".repeat(50)
                )
            );
        }

        #[test]
        fn a_file_or_a_name_that_is_not_there_is_refused() {
            let dir = tempfile::tempdir().unwrap();
            let missing = dir.path().join("none.yaml");
            assert!(goal_seek_core(&missing, "y", 1.0, "x", (None, None), 1e-9).is_err());
            let path = model(dir.path(), "1", "=x");
            for (target, vary) in [("y", "nope"), ("@nope.y", "x")] {
                let err = goal_seek_core(&path, target, 1.0, vary, (None, None), 1e-9)
                    .unwrap_err()
                    .to_string();
                assert!(err.contains("nope"), "{target}, {vary}: {err}");
            }
        }
    }

    mod printed_numbers {
        //! The solution `forge goal-seek` prints and the numbers beside it
        //! follow `forge report`'s rule (Session Prompt part 1, item 5): two decimals
        //! and thousands separators, four significant digits under 1, and money with
        //! the sign before the `$`. JSON and files keep the raw numbers.

        use crate::cli::forge;

        #[test]
        fn goal_seek_prints_the_solution_with_two_decimals_and_every_digit() {
            // examples/mortgage: 397.04 a month saves 80,000 of interest
            // (README), and the input that gives exactly 80,000 is beside it:
            // two decimals alone hide that 397.04 is not it (agents, 2026-11-24)
            let text = forge(&[
                "goal-seek",
                "examples/mortgage/mortgage.yaml",
                "--target",
                "saved.interest",
                "--value",
                "80000",
                "--vary",
                "loan.extra_per_month",
            ])
            .printed();
            assert!(
                text.contains(
                    "loan.extra_per_month = 397.04 (397.044207903935) → saved.interest = 80,000.00"
                ),
                "{text}"
            );
        }
    }

    /// Every digit beside the two decimals only where they differ
    #[test]
    fn a_solution_shows_its_digits_only_where_two_decimals_hide_them() {
        assert_eq!(super::solution_text(12.0), "12.00");
        assert_eq!(super::solution_text(1_450.5), "1,450.50");
        assert_eq!(super::solution_text(0.0479), "0.0479");
        assert_eq!(
            super::solution_text(15.966_607_933_396),
            "15.97 (15.966607933396)"
        );
        assert_eq!(super::solution_text(1_234.567), "1,234.57 (1234.567)");
    }

    mod break_even_header {
        //! `forge break-even` said it was goal-seek (the end-to-end pass of 2026-11-12).

        use crate::cli::forge;

        #[test]
        fn break_even_has_a_header_of_its_own() {
            let text = forge(&[
                "break-even",
                "examples/bagel/bagel.yaml",
                "--output",
                "year.profit",
                "--vary",
                "assumptions.price_per_dozen",
            ])
            .printed();
            assert!(text.contains("Break-Even Analysis"), "{text}");
            assert!(text.contains("Finding where year.profit = 0"), "{text}");
            assert!(!text.contains("Goal Seek"), "{text}");
            assert!(
                text.contains("assumptions.price_per_dozen = 15.97"),
                "{text}"
            );
        }
    }

    mod microgrid_break_even {
        //! examples/microgrid: the diesel price at which the island's microgrid
        //! is worth nothing. The `run` of the Python in prediction/scenarios.rs
        //! (`microgrid_example`), bisected on the price: 0.5840419536912178.

        use crate::cli::goal_seek_core;
        use std::path::Path;

        #[test]
        fn diesel_would_have_to_fall_to_58_cents() {
            let result = goal_seek_core(
                Path::new("examples/microgrid/microgrid.yaml"),
                "verdict.project_npv",
                0.0,
                "fuel.diesel_price",
                (Some(0.1), Some(1.45)),
                0.0001,
            )
            .unwrap();
            assert!(result.converged);
            assert!(
                (result.solution - 0.584_041_953_691_217_8).abs() <= 1e-6,
                "{}",
                result.solution
            );
        }
    }

    mod fund_goal_seek {
        //! examples/fund: how much better every exit must be for the investors
        //! to earn 15% after fees and carry. The `flows` of the Python in
        //! prediction/scenarios.rs (`fund_example`), solved with brentq on the
        //! factor: 1.2323832625047246.

        use crate::cli::goal_seek_core;
        use std::path::Path;

        #[test]
        fn exits_a_quarter_better_give_the_investors_15_percent() {
            let result = goal_seek_core(
                Path::new("examples/fund/fund.yaml"),
                "fund.net_irr",
                0.15,
                "market.multiple_factor",
                (Some(0.5), Some(3.0)),
                1e-12,
            )
            .unwrap();
            assert!(result.converged);
            assert!(
                (result.solution - 1.232_383_262_504_724_6).abs() <= 1e-9,
                "{}",
                result.solution
            );
        }
    }

    mod vineyard_break_even {
        //! examples/vineyard: the price a tonne at which the season breaks even.
        //! The `season` of the Python in prediction/scenarios.rs
        //! (`vineyard_example`), bisected on the price: 1593.8063063063057, the
        //! costs 283,060 over the 177.6 price-weighted tonnes.

        use crate::cli::goal_seek_core;
        use std::path::Path;

        #[test]
        fn the_season_breaks_even_at_1_593_81_a_tonne() {
            let result = goal_seek_core(
                Path::new("examples/vineyard/vineyard.yaml"),
                "season.profit",
                0.0,
                "harvest.price_per_tonne",
                (None, None),
                0.0001,
            )
            .unwrap();
            assert!(result.converged);
            assert!(
                (result.solution - 1_593.806_306_306_305_7).abs() <= 1e-6,
                "{}",
                result.solution
            );
        }
    }

    mod realestate_break_even {
        //! examples/realestate: the price at which Linden Court is worth what it
        //! costs. The `run` of the Python in prediction/scenarios.rs
        //! (`realestate_example`), bisected on the price: 4753414.102522027.

        use crate::cli::goal_seek_core;
        use std::path::Path;

        #[test]
        fn the_building_is_worth_its_cost_at_4_75_million() {
            let result = goal_seek_core(
                Path::new("examples/realestate/realestate.yaml"),
                "valuation.npv",
                0.0,
                "deal.price",
                (Some(4_000_000.0), Some(5_500_000.0)),
                0.0001,
            )
            .unwrap();
            assert!(result.converged);
            assert!(
                (result.solution - 4_753_414.102_522_027).abs() <= 1e-3,
                "{}",
                result.solution
            );
        }

        /// Without bounds the search tries 1% and 100 times the price, where
        /// the equity return has no answer (IRR needs a cash flow of each
        /// sign). The value does not use it, so the search finds the same
        /// price; it failed with the IRR's error.
        #[test]
        fn a_formula_the_value_does_not_use_does_not_stop_the_search() {
            let result = goal_seek_core(
                Path::new("examples/realestate/realestate.yaml"),
                "valuation.npv",
                0.0,
                "deal.price",
                (None, None),
                0.0001,
            )
            .unwrap();
            assert!(result.converged);
            assert!(
                (result.solution - 4_753_414.102_522_027).abs() <= 1e-3,
                "{}",
                result.solution
            );
        }
    }

    mod acquisition_example {
        //! examples/acquisition: the most Valdane can pay, and the synergies or
        //! the cash it needs for earnings a share not to fall in year 3. The `run`
        //! of the Python in prediction/scenarios.rs (`acquisition_example`),
        //! bisected: a premium of 0.4834215142306294 (35.60 a share), cost synergies
        //! of 49.4216 a year, or 79.77% of the price in cash.

        use crate::cli::goal_seek_core;
        use std::path::Path;

        const MODEL: &str = "examples/acquisition/deal.yaml";

        #[test]
        fn valdane_walks_away_above_35_60_a_share() {
            let result = goal_seek_core(
                Path::new(MODEL),
                "valuation.value_created",
                0.0,
                "offer.premium",
                (Some(0.3), Some(0.6)),
                1e-9,
            )
            .unwrap();
            assert!(result.converged);
            assert!(
                (result.solution - 0.483_421_514_230_629_4).abs() <= 1e-9,
                "{}",
                result.solution
            );
        }

        #[test]
        fn earnings_a_share_hold_in_year_3_with_more_synergies_or_more_cash() {
            let seek = |vary: &str, bounds: (f64, f64)| {
                let result = goal_seek_core(
                    Path::new(MODEL),
                    "eps.year_3_accretion",
                    0.0,
                    vary,
                    (Some(bounds.0), Some(bounds.1)),
                    1e-12,
                )
                .unwrap();
                assert!(result.converged, "{vary}");
                result.solution
            };
            let cost = seek("synergies.cost_run_rate", (0.0, 200.0));
            assert!((cost - 49.421_6).abs() <= 1e-3, "{cost}");
            let cash = seek("financing.cash_share", (0.0, 1.0));
            assert!((cash - 0.797_691_888).abs() <= 1e-6, "{cash}");
        }
    }

    mod included_inputs {
        //! Roadmap T1-VARY-INCLUDED: goal-seek and break-even move an input of
        //! a file the model includes, written as a scenario override writes it.
        //! examples/acquisition/deal.yaml includes target.yaml as `target`; the
        //! deal creates no value once Quillon's free cash flow falls far enough,
        //! and the plain model with that cash flow says so.

        use crate::cli::{forge, goal_seek_core};
        use crate::core::ArrayCalculator;
        use std::path::Path;

        const MODEL: &str = "examples/acquisition/deal.yaml";
        const CASH: &str = "@target.company.free_cash_flow_next_year";

        fn value_created(cash: f64) -> f64 {
            let mut model = crate::parser::parse_model(Path::new(MODEL)).unwrap();
            model.override_scalar(CASH, Some(cash), None).unwrap();
            let result = ArrayCalculator::new(model).calculate_all().unwrap();
            result.scalars["valuation.value_created"].value.unwrap()
        }

        #[test]
        fn goal_seek_moves_an_input_of_an_included_file() {
            // No bounds: the search starts around the included value, 62
            let result = goal_seek_core(
                Path::new(MODEL),
                "valuation.value_created",
                0.0,
                CASH,
                (None, None),
                1e-9,
            )
            .unwrap();
            assert!(result.converged);
            assert_eq!(result.vary, CASH);
            assert!(
                result.solution > 40.0 && result.solution < 62.0,
                "{}",
                result.solution
            );
            assert!(value_created(result.solution).abs() <= 1e-6);
            // A bare name inside the included file is found there
            let bare = goal_seek_core(
                Path::new(MODEL),
                "valuation.value_created",
                0.0,
                "@target.free_cash_flow_next_year",
                (Some(40.0), Some(62.0)),
                1e-9,
            )
            .unwrap();
            assert_eq!(bare.vary, CASH);
        }

        #[test]
        fn break_even_prints_the_included_input() {
            let text = forge(&[
                "break-even",
                MODEL,
                "--output",
                "valuation.value_created",
                "--vary",
                CASH,
            ])
            .printed();
            assert!(text.contains(&format!("{CASH} = ")), "{text}");
        }

        #[test]
        fn an_include_that_is_not_there_is_refused_with_the_includes_listed() {
            let err = goal_seek_core(
                Path::new(MODEL),
                "valuation.value_created",
                0.0,
                "@seller.company.free_cash_flow_next_year",
                (None, None),
                1e-9,
            )
            .unwrap_err()
            .to_string();
            assert!(
                err.contains("as 'seller'") && err.contains("buyer, target"),
                "{err}"
            );
        }
    }
}
