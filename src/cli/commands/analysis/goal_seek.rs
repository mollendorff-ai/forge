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

    let current_value = base_model
        .scalars
        .get(vary)
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

    let mut low = lower;
    let mut high = upper;

    let f_low = calculate_with_override(&base_model, vary, low, target)? - value;
    let f_high = calculate_with_override(&base_model, vary, high, target)? - value;

    if f_low * f_high > 0.0 {
        let expanded = expand_search_range_quiet(&base_model, vary, target, value, lower, upper)?;
        low = expanded.0;
        high = expanded.1;
    }

    // Bisect until the output is within `tolerance` of the target, or the
    // interval cannot be split any further. `tolerance` is about the output,
    // so it is also what decides `converged`.
    let max_iterations = 200;
    let mut f_low = calculate_with_override(&base_model, vary, low, target)? - value;
    let mut mid = f64::midpoint(low, high);
    let mut iteration = 0;

    while iteration < max_iterations {
        mid = f64::midpoint(low, high);
        if mid <= low || mid >= high {
            break; // no float left between low and high
        }
        let f_mid = calculate_with_override(&base_model, vary, mid, target)? - value;
        iteration += 1;

        if f_mid.abs() <= tolerance {
            break;
        }

        if f_low * f_mid < 0.0 {
            high = mid;
        } else {
            low = mid;
            f_low = f_mid;
        }
    }

    let final_value = calculate_with_override(&base_model, vary, mid, target)?;
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

/// Expand search range without printing (for core function)
fn expand_search_range_quiet(
    base_model: &crate::types::ParsedModel,
    vary: &str,
    target: &str,
    value: f64,
    lower: f64,
    upper: f64,
) -> ForgeResult<(f64, f64)> {
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

        let f_exp_low = calculate_with_override(base_model, vary, exp_low, target)? - value;
        let f_exp_high = calculate_with_override(base_model, vary, exp_high, target)? - value;

        if f_exp_low * f_exp_high <= 0.0 {
            return Ok((exp_low, exp_high));
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
                "value.npv",
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
    }
}
