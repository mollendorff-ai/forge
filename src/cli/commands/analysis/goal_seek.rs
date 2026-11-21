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

    // Each end of the interval is an input with its output
    let eval = |x: f64| -> ForgeResult<(f64, f64)> {
        Ok((x, calculate_with_override(&base_model, vary, x, target)?))
    };
    let mut low = eval(lower)?;
    let mut high = eval(upper)?;
    if (low.1 - value) * (high.1 - value) > 0.0 {
        (low, high) = expand_search_range_quiet(&eval, vary, value, (lower, upper))?;
    }

    // Bisect until the output is within `tolerance` of the target, or the
    // interval cannot be split any further. `tolerance` is about the output,
    // so it is also what decides `converged`.
    let max_iterations = 200;
    let mut answer = low;
    let mut iteration = 0;

    while iteration < max_iterations {
        let mid = f64::midpoint(low.0, high.0);
        if mid <= low.0 {
            answer = low; // no float left between low and high
            break;
        }
        if mid >= high.0 {
            answer = high;
            break;
        }
        answer = eval(mid)?;
        iteration += 1;

        if (answer.1 - value).abs() <= tolerance {
            break;
        }

        if (low.1 - value) * (answer.1 - value) < 0.0 {
            high = answer;
        } else {
            low = answer;
        }
    }

    let error = (answer.1 - value).abs();

    Ok(crate::cli::commands::results::GoalSeekResult {
        vary: vary.clone(),
        target: target.clone(),
        target_value: value,
        solution: answer.0,
        achieved: answer.1,
        error,
        iterations: iteration,
        converged: error <= tolerance,
    })
}

/// Widen the search range, 10, 100 and 1000 times, until the output crosses the
/// target between its ends; each end comes back with its output.
fn expand_search_range_quiet(
    eval: &impl Fn(f64) -> ForgeResult<(f64, f64)>,
    vary: &str,
    value: f64,
    (lower, upper): (f64, f64),
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

        let (low, high) = (eval(exp_low)?, eval(exp_high)?);
        if (low.1 - value) * (high.1 - value) <= 0.0 {
            return Ok((low, high));
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
    print_goal_seek_result(
        c,
        vary,
        (target, value),
        result.solution,
        result.achieved,
        tolerance,
        result.iterations,
    );
    Ok(())
}

/// Print goal-seek result summary
fn print_goal_seek_result(
    c: &mut Console<'_>,
    vary: &str,
    (target, value): (&str, f64),
    mid: f64,
    final_value: f64,
    tolerance: f64,
    iteration: i32,
) {
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
        display_number(mid).bold().green(),
        target.bright_blue(),
        display_number(final_value).green()
    );

    let error = (final_value - value).abs();
    if error <= tolerance {
        outln!(c, "   {} Within tolerance", "✅".green());
    } else {
        outln!(
            c,
            "   {} Error: {} (tolerance: {})",
            "⚠️".yellow(),
            error,
            tolerance
        );
    }
    outln!(c, "{}", "─".repeat(50));
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
    use super::*;
    use crate::cli::console::capture;
    use std::path::PathBuf;

    /// A model with one input `m.x` and the output `m.y` = `formula`.
    fn model(x: f64, formula: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.yaml");
        std::fs::write(
            &path,
            format!("_forge_version: \"5.2.0\"\nm:\n  x: {x}\n  y:\n    formula: \"{formula}\"\n"),
        )
        .unwrap();
        (dir, path)
    }

    fn seek(path: &Path, value: f64, bounds: (Option<f64>, Option<f64>)) -> ForgeResult<f64> {
        goal_seek_core(path, "m.y", value, "m.x", bounds, 1e-6).map(|r| r.solution)
    }

    #[test]
    fn the_default_range_follows_the_sign_of_the_input() {
        // Negative input: from 100 times it to 1% of it
        let (_dir, negative) = model(-5.0, "=x * 2");
        let x = seek(&negative, -20.0, (None, None)).unwrap();
        assert!((x + 10.0).abs() < 1e-5, "{x}");
        // Zero input: from -1000 to 1000
        let (_dir, zero) = model(0.0, "=x * 2");
        let x = seek(&zero, 10.0, (None, None)).unwrap();
        assert!((x - 5.0).abs() < 1e-5, "{x}");
        // Given bounds are kept
        let (_dir, positive) = model(5.0, "=x * 2");
        let result =
            goal_seek_core(&positive, "m.y", 10.0, "m.x", (Some(1.0), Some(50.0)), 1e-9).unwrap();
        assert!(result.converged);
        assert!((result.solution - 5.0).abs() < 1e-8);
        assert!(result.error <= 1e-9);
    }

    #[test]
    fn a_range_that_misses_the_target_is_widened() {
        // x = 2500 is outside 0.05 to 500, and inside 0.005 to 5000
        let (_dir, path) = model(5.0, "=x * 2");
        let x = seek(&path, 5000.0, (None, None)).unwrap();
        assert!((x - 2500.0).abs() < 1e-3, "{x}");
    }

    #[test]
    fn a_target_no_input_reaches_is_refused() {
        let (_dir, path) = model(-5.0, "=x * 2");
        let error = seek(&path, 5.0, (None, None)).unwrap_err().to_string();
        assert!(
            error.contains("No solution found in search range") && error.contains("'m.x'"),
            "{error}"
        );
        let (_dir, positive) = model(5.0, "=x * 2");
        assert!(seek(&positive, 1e12, (None, None)).is_err());
    }

    #[test]
    fn a_jump_the_output_skips_stops_at_the_jump_and_says_so() {
        // The output is 0 up to x = 5 and 100 beyond: 50 is never reached
        let (_dir, path) = model(5.0, "=IF(x > 5, 100, 0)");
        let (result, out) =
            capture(|c| goal_seek(c, &path, ("m.y", 50.0), "m.x", (None, None), 1e-3, true));
        result.unwrap();
        for expected in [
            "Goal Seek",
            "Tolerance: 0.001",
            "Bisection stopped after",
            "Error: 50 (tolerance: 0.001)",
        ] {
            assert!(out.out.contains(expected), "{expected}\n{}", out.out);
        }
        assert!(!out.out.contains("Within tolerance"), "{}", out.out);
        let core = goal_seek_core(&path, "m.y", 50.0, "m.x", (None, None), 1e-3).unwrap();
        assert!(!core.converged);
        assert!((core.error - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn the_command_without_verbose_says_when_it_is_within_tolerance() {
        let (_dir, path) = model(5.0, "=x * 2");
        let (result, out) =
            capture(|c| goal_seek(c, &path, ("m.y", 10.0), "m.x", (None, None), 1e-6, false));
        result.unwrap();
        assert!(out.out.contains("Within tolerance"), "{}", out.out);
        assert!(!out.out.contains("Bisection stopped"), "{}", out.out);
    }

    #[test]
    fn break_even_with_bounds_finds_the_zero() {
        let (_dir, path) = model(5.0, "=x - 3");
        let (result, out) =
            capture(|c| break_even(c, &path, "m.y", "m.x", Some(0.0), Some(10.0), false));
        result.unwrap();
        assert!(out.out.contains("Finding where m.y = 0"), "{}", out.out);
        assert!(out.out.contains("m.x = 3"), "{}", out.out);
    }

    #[test]
    fn an_output_that_fails_where_the_search_looks_stops_the_search() {
        let cases = [
            // (formula, x, target, bounds): the failure is at the low end, the
            // high end, the middle of the range, and in the widened range
            ("=SQRT(x - 5)", 6.0, 1.0, (Some(1.0), Some(50.0))),
            ("=IF(x > 10, 1 / 0, x)", 5.0, 5.0, (Some(1.0), Some(20.0))),
            (
                "=IF(AND(x > 100, x < 300), 1 / 0, x)",
                5.0,
                400.0,
                (Some(1.0), Some(500.0)),
            ),
            ("=IF(x > 1000, 1 / 0, x * 2)", 5.0, 5000.0, (None, None)),
            ("=IF(x < 0.01, 1 / 0, x * 2)", 5.0, 5000.0, (None, None)),
        ];
        for (formula, x, target, bounds) in cases {
            let (_dir, path) = model(x, formula);
            assert!(
                goal_seek_core(&path, "m.y", target, "m.x", bounds, 1e-6).is_err(),
                "{formula}"
            );
        }
    }

    #[test]
    fn a_jump_is_found_to_the_last_float_whichever_way_the_midpoint_rounds() {
        // The last interval is the two floats around the jump; the midpoint of
        // two neighbours is the one with the even mantissa, low or high
        for jump in [3.0, 5.0, 5.5, 7.0, 9.0, 11.0, 13.0, 17.5] {
            let (_dir, path) = model(5.0, &format!("=IF(x > {jump}, 100, 0)"));
            let result = goal_seek_core(&path, "m.y", 50.0, "m.x", (None, None), 1e-6).unwrap();
            assert!(!result.converged, "{jump}");
            assert!(
                (result.solution - jump).abs() < 1e-9,
                "{jump}: {}",
                result.solution
            );
            assert!((result.error - 50.0).abs() < f64::EPSILON, "{jump}");
        }
    }

    #[test]
    fn an_ambiguous_target_is_refused() {
        let (_dir, path) = model(5.0, "=x * 2");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, format!("{text}n:\n  y: 1\n")).unwrap();
        let error = goal_seek_core(&path, "y", 1.0, "m.x", (None, None), 1e-6)
            .unwrap_err()
            .to_string();
        assert!(error.contains("could be any of m.y, n.y"), "{error}");
    }

    #[test]
    fn unknown_names_and_files_are_refused() {
        let (_dir, path) = model(5.0, "=x * 2");
        assert!(goal_seek_core(&path, "m.ghost", 1.0, "m.x", (None, None), 1e-6).is_err());
        assert!(goal_seek_core(&path, "m.y", 1.0, "m.ghost", (None, None), 1e-6).is_err());
        assert!(seek(Path::new("no/such/model.yaml"), 1.0, (None, None)).is_err());
    }

    mod printed_numbers {
        //! The solution `forge goal-seek` prints and the numbers beside it
        //! follow `forge report`'s rule (Session Prompt part 1, item 5): two decimals
        //! and thousands separators, four significant digits under 1, and money with
        //! the sign before the `$`. JSON and files keep the raw numbers.

        use crate::cli::forge;

        #[test]
        fn goal_seek_prints_the_solution_with_two_decimals() {
            // examples/mortgage: 397.04 a month saves 80,000 of interest (README)
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
                text.contains("loan.extra_per_month = 397.04 → saved.interest = 80,000.00"),
                "{text}"
            );
        }
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
