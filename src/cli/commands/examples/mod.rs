//! Examples command - display runnable YAML examples for Forge capabilities

use crate::error::{ForgeError, ForgeResult};
// Printing the examples and running one belong to the command line
#[cfg(feature = "cli")]
use {
    crate::cli::console::outln, crate::cli::Console, colored::Colorize, std::ffi::OsStr,
    std::io::Write,
};

mod catalog;
use catalog::EXAMPLES;

/// Return examples as structured JSON (no printing).
///
/// If `name` is `Some`, returns the specific example's content.
/// If `name` is `None`, returns the list of all available examples.
///
/// # Errors
///
/// Returns an error if the requested example name is not found.
pub fn examples_core(name: Option<&str>) -> ForgeResult<serde_json::Value> {
    if let Some(name) = name {
        let example = EXAMPLES.iter().find(|e| e.name == name).ok_or_else(|| {
            ForgeError::Validation(format!(
                "Unknown example '{name}'. Available: {}",
                EXAMPLES
                    .iter()
                    .map(|e| e.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;
        Ok(serde_json::json!({
            "name": example.name,
            "description": example.description,
            "command": example.command,
            "content": example.content
        }))
    } else {
        let list: Vec<serde_json::Value> = EXAMPLES
            .iter()
            .map(|e| {
                serde_json::json!({
                    "name": e.name,
                    "description": e.description,
                    "command": e.command
                })
            })
            .collect();
        Ok(serde_json::json!(list))
    }
}

/// Display example YAML models for Forge capabilities.
///
/// # Errors
///
/// Returns an error if the requested example name is not found,
/// or if execution of the example fails when `--run` is specified.
///
/// # Panics
///
/// Panics if `name` is `Some` and `.unwrap()` is called after the `None` check.
/// This is safe because we only unwrap after verifying `name.is_some()`.
#[cfg(feature = "cli")]
pub fn examples(
    c: &mut Console<'_>,
    name: Option<String>,
    run: bool,
    json: bool,
) -> ForgeResult<()> {
    // JSON output mode for tooling
    if json {
        print_examples_json(c);
        return Ok(());
    }

    // No name specified - show list
    if name.is_none() {
        print_examples_list(c);
        return Ok(());
    }

    let name = name.unwrap();

    // Find the requested example
    let example = EXAMPLES.iter().find(|e| e.name == name).ok_or_else(|| {
        ForgeError::Validation(format!(
            "Unknown example '{name}'. Run 'forge examples' to see available examples.",
        ))
    })?;

    // Print the example
    outln!(c, "{}", format!("# {}", example.name).bold().green());
    outln!(c, "{}", format!("# {}", example.description).dimmed());
    outln!(
        c,
        "{}",
        format!("# Run with: {} <file>", example.command).dimmed()
    );
    outln!(c);
    outln!(c, "{}", example.content);

    // Run the example if requested
    if run {
        outln!(c);
        outln!(c, "{}", "─".repeat(60).dimmed());
        outln!(c, "{}", "Running example...".bold().cyan());
        outln!(c);
        run_example(c, example)?;
    }

    Ok(())
}

/// Print list of available examples
#[cfg(feature = "cli")]
fn print_examples_list(c: &mut Console<'_>) {
    outln!(c, "{}", "Available Forge Examples".bold().green());
    outln!(c);
    outln!(
        c,
        "{}",
        "Start with `starter`; the others show Forge's analysis commands:".dimmed()
    );
    outln!(c);

    for example in EXAMPLES {
        outln!(
            c,
            "  {:16} {}",
            example.name.cyan().bold(),
            example.description
        );
    }

    outln!(c);
    outln!(c, "Usage: {}", "forge examples <name>".yellow());
    outln!(c, "       {}", "forge examples <name> --run".yellow());
    outln!(c);
    outln!(c, "Example: {}", "forge examples monte-carlo".dimmed());
}

/// Print examples as JSON for tooling
#[cfg(feature = "cli")]
fn print_examples_json(c: &mut Console<'_>) {
    let examples_json: Vec<serde_json::Value> = EXAMPLES
        .iter()
        .map(|e| {
            serde_json::json!({
                "name": e.name,
                "description": e.description,
                "command": e.command
            })
        })
        .collect();

    outln!(
        c,
        "{}",
        serde_json::to_string_pretty(&examples_json).unwrap()
    );
}

/// Run an example by writing to temp file and executing
#[cfg(feature = "cli")]
fn run_example(c: &mut Console<'_>, example: &catalog::Example) -> ForgeResult<()> {
    // Write example to temp file
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("forge-example-{}.yaml", example.name));

    let mut file = std::fs::File::create(&temp_file).map_err(|e| {
        ForgeError::Io(std::io::Error::other(format!(
            "Failed to create temp file: {e}"
        )))
    })?;

    file.write_all(example.content.as_bytes()).map_err(|e| {
        ForgeError::Io(std::io::Error::other(format!(
            "Failed to write temp file: {e}"
        )))
    })?;

    // The command that runs it: `forge game` runs `game`
    let command = example.command.trim_start_matches("forge ");

    // Run it as `forge` would, in this same process, on the same console
    let code = crate::cli::run_from(
        [
            OsStr::new("forge"),
            OsStr::new(command),
            temp_file.as_os_str(),
        ],
        c,
    );

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_file);

    if code == 0 {
        Ok(())
    } else {
        Err(ForgeError::Validation(format!(
            "Example execution failed with exit code: {code}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::console::capture;

    #[test]
    fn test_all_examples_are_valid_yaml() {
        for example in EXAMPLES {
            let parsed: Result<yaml_serde::Value, _> = yaml_serde::from_str(example.content);
            assert!(
                parsed.is_ok(),
                "Example '{}' should be valid YAML: {:?}",
                example.name,
                parsed.err()
            );
        }
    }

    #[test]
    fn the_list_names_every_example() {
        let (result, printed) = capture(|c| examples(c, None, false, false));
        result.unwrap();
        assert!(
            printed.out.starts_with("Available Forge Examples\n"),
            "{}",
            printed.out
        );
        for example in EXAMPLES {
            let line = format!("  {:16} {}\n", example.name, example.description);
            assert!(printed.out.contains(&line), "{line:?}: {}", printed.out);
        }
        assert!(printed.out.contains("Usage: forge examples <name>\n"));
    }

    #[test]
    fn json_lists_every_example_with_its_command() {
        let (result, printed) = capture(|c| examples(c, Some("ignored".to_string()), false, true));
        result.unwrap();
        let listed: serde_json::Value = serde_json::from_str(&printed.out).unwrap();
        let listed = listed.as_array().unwrap();
        assert_eq!(listed.len(), EXAMPLES.len());
        for (json, example) in listed.iter().zip(EXAMPLES) {
            assert_eq!(json["name"], example.name);
            assert_eq!(json["description"], example.description);
            assert_eq!(json["command"], example.command);
        }
    }

    #[test]
    fn an_example_is_printed_with_its_header() {
        for example in EXAMPLES {
            let (result, printed) =
                capture(|c| examples(c, Some(example.name.to_string()), false, false));
            result.unwrap();
            let expected = format!(
                "# {}\n# {}\n# Run with: {} <file>\n\n{}\n",
                example.name, example.description, example.command, example.content
            );
            assert_eq!(printed.out, expected, "{}", example.name);
        }
    }

    #[test]
    fn an_unknown_example_is_refused() {
        let (result, printed) =
            capture(|c| examples(c, Some("nonexistent".to_string()), false, false));
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unknown example 'nonexistent'"), "{err}");
        assert!(err.contains("forge examples"), "{err}");
        assert_eq!(printed.both(), "");
    }

    #[test]
    fn run_runs_the_example_with_its_command_in_process() {
        let (result, printed) = capture(|c| examples(c, Some("tornado".to_string()), true, false));
        result.unwrap();
        let (header, run) = printed.out.split_once("Running example...\n\n").unwrap();
        assert!(header.starts_with("# tornado\n"), "{header}");
        // What `forge tornado` prints, not `forge calculate`
        assert!(run.starts_with("🌪️ Forge - Tornado Diagram\n"), "{run}");
        assert_eq!(printed.err, "");
    }

    /// `examples/expected_loss.yaml` (ADR-074, step 6): its 17 figures, the
    /// network's posteriors and Bayes' rule by hand, hold in the file and both
    /// scenarios, and a figure moved fails with its scenario named
    #[test]
    fn the_expected_loss_states_its_figures_and_validate_checks_them() {
        let (result, printed) =
            capture(|c| examples(c, Some("expected-loss".to_string()), true, false));
        result.unwrap();
        assert!(
            printed
                .out
                .contains("✅ 17 expected results match (the file as written and 2 scenarios)"),
            "{}",
            printed.out
        );
        let file = std::path::Path::new("examples/expected_loss.yaml");
        let found = crate::cli::commands::validate_core(file).unwrap();
        assert!(found.expectations_met);
        let dir = tempfile::tempdir().unwrap();
        let broken = dir.path().join("m.yaml");
        let text = std::fs::read_to_string(file)
            .unwrap()
            .replace("loss.expected: 45180", "loss.expected: 45000");
        std::fs::write(&broken, text).unwrap();
        let found = crate::cli::commands::validate_core(&broken).unwrap();
        let wrong: Vec<_> = found
            .expectation_differences
            .iter()
            .map(|d| {
                let forge_gives = d.got.is_some_and(|got| (got - 45180.0).abs() < 1e-6);
                (d.scenario.as_deref(), d.name.as_str(), forge_gives)
            })
            .collect();
        assert_eq!(wrong, [(Some("boom"), "loss.expected", true)]);
    }

    #[test]
    fn an_example_that_fails_says_so_with_its_exit_code() {
        let broken = catalog::Example {
            name: "broken",
            description: "A formula that reads nothing",
            command: "forge calculate",
            content:
                "_forge_version: \"5.0.0\"\nm:\n  x:\n    value: null\n    formula: \"=nowhere\"\n",
        };
        let (result, printed) = capture(|c| run_example(c, &broken));
        assert_eq!(
            result.unwrap_err().to_string(),
            "Validation error: Example execution failed with exit code: 1"
        );
        assert!(printed.err.starts_with("Error: "), "{}", printed.err);
        assert!(printed.err.contains("nowhere"), "{}", printed.err);
    }

    #[test]
    fn test_example_count() {
        assert_eq!(EXAMPLES.len(), 21, "Should have exactly 21 examples");
    }

    #[test]
    fn test_all_examples_have_forge_version() {
        for example in EXAMPLES {
            assert!(
                example.content.contains("_forge_version:"),
                "Example '{}' should have _forge_version field",
                example.name
            );
        }
    }

    mod bagel_example {
        //! The numbers in examples/bagel/README.md, as computed outside Forge.
        //!
        //! The Python that produced them (independent of Forge's evaluator):
        //! ```python
        //! season = [0.85, 1.25, 0.80, 0.80, 0.90, 0.95, 1.05, 1.15, 1.25, 1.25, 1.05, 1.00]
        //! w = 18.5 * 1.15
        //! def profit(flour=1.15, price=16.0):
        //!     total = 0
        //!     for m, se in enumerate(season, 1):
        //!         dz = 55 * 1.03 ** (m - 1) * se * 30
        //!         r = dz * price
        //!         c = dz * 12 / 9 * flour + dz * 12 * 6 / 1000 * 9.40 + dz * 0.35 + 32 * 30 * w + 5750 + r * 0.026
        //!         total += r - c
        //!     return total
        //! profit()            # 786.3203617
        //! profit(flour=1.70)  # -16943.271075
        //! ```

        use crate::cli::{calculate_core, goal_seek_core, scenarios_core};
        use std::path::Path;

        const MODEL: &str = "examples/bagel/bagel.yaml";

        fn close(a: f64, b: f64, tol: f64) -> bool {
            (a - b).abs() <= tol
        }

        #[test]
        fn year_profit_matches_the_independent_calculation() {
            let result = calculate_core(Path::new(MODEL), true, None).unwrap();
            let profit = result.scalars["year.profit"].unwrap();
            assert!(close(profit, 786.320_361_7, 1e-6), "{profit}");
            assert!(close(
                result.scalars["year.revenue"].unwrap(),
                386_827.449_537_8,
                1e-6
            ));
        }

        /// The first year's return on the fit-out: profit / 95,000. It replaced
        /// `payback_months`, which was 1,449.79 in the base case (120 years) and
        /// negative when the year lost money: a payback that is not a number cannot
        /// be averaged by `forge scenarios`, a return can (786.3203617 / 95000 and
        /// -16943.271075 / 95000 in Python).
        #[test]
        fn first_year_return_is_defined_whatever_the_profit() {
            let result = calculate_core(Path::new(MODEL), true, None).unwrap();
            let base = result.scalars["year.first_year_return"].unwrap();
            assert!(close(base, 786.320_361_7 / 95_000.0, 1e-9), "{base}");
            let scenarios = vec!["flour_shock".to_string()];
            let shock = compare_core(Path::new(MODEL), &scenarios).unwrap();
            let value = shock.values["year.first_year_return"]["flour_shock"].unwrap();
            assert!(close(value, -16_943.271_075 / 95_000.0, 1e-9), "{value}");
        }

        #[test]
        fn flour_shock_changes_the_included_price() {
            let results = scenarios_core(Path::new(MODEL), Some("flour_shock")).unwrap();
            let shock = &results.scenarios[0];
            assert_eq!(shock.name, "flour_shock");
            assert!(close(shock.outputs["year.profit"], -16_943.271_075, 1e-6));
            // The expected value spans all four scenarios: 0.5*786.32 + 0.1*(-16943.27) + 0.2*(-71197.20) + 0.2*80811.24
            assert!(close(
                results.expected_values["year.profit"],
                621.641_286,
                1e-5
            ));
        }

        #[test]
        fn break_even_price_and_staffing_for_sixty_thousand() {
            let model = Path::new(MODEL);
            let price = goal_seek_core(
                model,
                "year.profit",
                0.0,
                "assumptions.price_per_dozen",
                (Some(10.0), Some(30.0)),
                0.0001,
            )
            .unwrap();
            assert!(price.converged);
            assert!(close(price.solution, 15.966_6, 1e-3), "{}", price.solution);

            let hours = goal_seek_core(
                model,
                "year.profit",
                60_000.0,
                "assumptions.staff_hours_per_day",
                (Some(8.0), Some(40.0)),
                0.0001,
            )
            .unwrap();
            assert!(hours.converged);
            assert!(close(hours.solution, 24.268_7, 1e-3), "{}", hours.solution);
        }

        /// The tornado in the README: each input moved alone to its low and high.
        ///
        /// The profit function of the comment at the top of this file, with every
        /// input as an argument (rent 4800 + firewood 950 were its 5750):
        /// ```python
        /// for k, (lo, hi) in dict(opening=(45, 65), price=(14, 18), staff=(26, 36),
        ///                         rent=(4000, 6000), fee=(0.018, 0.03)).items():
        ///     profit(**{k: lo}), profit(**{k: hi})
        /// # opening -56463.56 58036.20, price -46309.92 47882.56,
        /// # staff 46740.32 -29849.68, rent 10386.32 -13613.68, fee 3880.94 -760.99
        /// ```
        #[test]
        fn tornado_matches_the_independent_calculation() {
            let result = crate::cli::tornado_core(Path::new(MODEL), None).unwrap();
            let expected = [
                ("assumptions.opening_dozens_per_day", -56_463.56, 58_036.20),
                ("assumptions.price_per_dozen", -46_309.92, 47_882.56),
                ("assumptions.staff_hours_per_day", 46_740.32, -29_849.68),
                ("assumptions.rent_per_month", 10_386.32, -13_613.68),
                ("assumptions.card_fee_rate", 3_880.94, -760.99),
            ];
            assert_eq!(result.bars.len(), expected.len());
            // In the order of the swing, largest first
            for (bar, (name, low, high)) in result.bars.iter().zip(expected) {
                assert_eq!(bar.input_name, name);
                assert!(
                    close(bar.output_at_low, low, 0.006),
                    "{name} {}",
                    bar.output_at_low
                );
                assert!(
                    close(bar.output_at_high, high, 0.006),
                    "{name} {}",
                    bar.output_at_high
                );
            }
        }

        // ─────────────────────────────────────────────────────────────────────────────
        // The rest of examples/bagel: one model per analytics command (roadmap T1-USE).
        // Each expected value was computed by Python written independently of Forge;
        // the computation is shown with the test.
        // ─────────────────────────────────────────────────────────────────────────────

        use crate::cli::{
            bayesian_core, bootstrap_core, compare_core, decision_tree_core, real_options_core,
            variance_core,
        };

        /// Plan vs what happened. Rent and staff hours went up, so they are
        /// unfavorable; the NPV got worse, so its percentage is negative even though
        /// the budget NPV is negative; price has no direction, so it is not judged.
        #[test]
        fn variance_judges_costs_as_costs() {
            let v = variance_core(
                Path::new(MODEL),
                Path::new("examples/bagel/actual.yaml"),
                10.0,
            )
            .unwrap();
            let get = |n: &str| v.results.iter().find(|r| r.name == n).unwrap();
            assert_eq!(get("assumptions.rent_per_month").is_favorable, Some(false));
            assert_eq!(get("assumptions.staff_hours_per_day").better, "lower");
            assert_eq!(
                get("assumptions.staff_hours_per_day").is_favorable,
                Some(false)
            );
            assert_eq!(get("assumptions.price_per_dozen").is_favorable, None);
            let npv = get("year.npv_first_year");
            // -123258.79 vs -95579.52: 27679.27 worse, 28.96% of the size of the budget
            assert!(
                close(npv.variance_pct, -28.959_419, 1e-5),
                "{}",
                npv.variance_pct
            );
            assert_eq!(npv.is_favorable, Some(false));
            assert!(v.not_compared.is_empty(), "{:?}", v.not_compared);
        }

        /// ```python
        /// open_now = 0.45 * 420_000 + 0.55 * 90_000 - 180_000             # 58,500
        /// busy  = max(0.8 * 400_000 + 0.2 * 85_000 - 190_000, 0)          # 147,000
        /// quiet = max(0.1 * 400_000 + 0.9 * 85_000 - 190_000, 0)          # 0
        /// popup = 0.5 * busy + 0.5 * quiet - 12_000                       # 61,500
        /// ```
        #[test]
        fn decision_tree_prefers_the_popup() {
            let r = decision_tree_core(Path::new("examples/bagel/second_shop.yaml")).unwrap();
            assert!(close(r.root_expected_value, 61_500.0, 1e-9));
            let root = &r.node_results["root"];
            assert_eq!(root.optimal_choice.as_deref(), Some("popup_first"));
            assert!(root.tied_with.is_empty(), "{:?}", root.tied_with);
            assert!(close(root.branch_values["open_now"], 58_500.0, 1e-9));
            assert!(close(r.risk_profile.probability_positive, 0.4, 1e-12));
            assert!(close(r.risk_profile.worst_case, -117_000.0, 1e-9));
        }

        /// Cox-Ross-Rubinstein, 200 steps, American, written from the textbook:
        /// S=240000, sigma=0.35, r=0.04, q=0.06, T=1.
        /// Defer (call, K=180000) = 64604.68351396034; abandon (put, K=110000) = 279.7243076076951.
        #[test]
        fn real_options_match_an_independent_binomial_tree() {
            let r = real_options_core(Path::new("examples/bagel/wait_option.yaml")).unwrap();
            let defer = r.options["Wait up to a year"].value;
            let abandon = r.options["Sell the oven and fit-out if it fails"].value;
            assert!(close(defer, 64_604.683_513_960_34, 1e-6), "{defer}");
            assert!(close(abandon, 279.724_307_607_695_1, 1e-6), "{abandon}");
        }

        /// numpy, 200,000 resamples of the same 30 days: mean 50.7, standard error
        /// 2.0162, 95% interval [46.8, 54.667]. Forge's own resampler (seed 3) must
        /// agree within resampling noise.
        #[test]
        fn bootstrap_agrees_with_numpy() {
            let r = bootstrap_core(
                Path::new("examples/bagel/first_month.yaml"),
                None,
                None,
                None,
            )
            .unwrap();
            assert!(close(r.original_estimate, 50.7, 1e-12));
            assert!(
                close(r.bootstrap_std_error, 2.0162, 0.03),
                "{}",
                r.bootstrap_std_error
            );
            let ci95 = r
                .confidence_intervals
                .iter()
                .find(|c| close(c.level, 0.95, 1e-12))
                .unwrap();
            assert!(close(ci95.lower, 46.8, 0.15) && close(ci95.upper, 54.667, 0.15));
        }

        /// Enumerating the joint distribution of weather x roadwork x sales:
        /// P(roadwork | slow) = 0.4; P(roadwork | slow, rainy) = 0.272727 (rain
        /// explains the slow week, so roadwork becomes less likely).
        #[test]
        fn bayesian_two_parents_explain_away() {
            let file = Path::new("examples/bagel/slow_week.yaml");
            let p_yes = |evidence: &[&str]| {
                let ev: Vec<String> = evidence.iter().map(ToString::to_string).collect();
                let r = bayesian_core(file, Some("roadwork"), &ev, None).unwrap();
                let q = &r.queries["roadwork"];
                q.probabilities[q.states.iter().position(|s| s == "yes").unwrap()]
            };
            assert!(close(p_yes(&["sales=slow"]), 0.4, 1e-9));
            assert!(close(
                p_yes(&["sales=slow", "weather=rainy"]),
                3.0 / 11.0,
                1e-9
            ));
        }

        /// The same input gives the same output, in the same order (STABILITY.md 4).
        #[test]
        fn compare_and_real_options_output_is_the_same_every_time() {
            let scenarios = vec!["tourist_summer".to_string(), "base".to_string()];
            let a = serde_json::to_string(&compare_core(Path::new(MODEL), &scenarios).unwrap())
                .unwrap();
            for _ in 0..5 {
                let b = serde_json::to_string(&compare_core(Path::new(MODEL), &scenarios).unwrap())
                    .unwrap();
                assert_eq!(a, b);
            }
            let file = Path::new("examples/bagel/wait_option.yaml");
            let a = serde_json::to_string(&real_options_core(file).unwrap()).unwrap();
            for _ in 0..5 {
                assert_eq!(
                    a,
                    serde_json::to_string(&real_options_core(file).unwrap()).unwrap()
                );
            }
        }

        /// `forge simulate` on the bagel model with uncertain demand, staffing, flour
        /// price and wage (the last two in the included `costs_risk.yaml`).
        ///
        /// Checked outside Forge: the model typed again below (and in numpy) gives
        /// Forge's outputs from Forge's own input samples to 1e-9, and on those
        /// samples numpy gives the threshold probabilities and
        /// `scipy.stats.spearmanr` the rank correlations asserted here (1,000
        /// iterations, seed 7).
        #[test]
        // The model is typed as in the Python check, row by row, so it reads the same
        #[allow(clippy::needless_range_loop, clippy::suboptimal_flops)]
        fn simulation_drivers_and_thresholds_match_scipy() {
            use crate::cli::{simulate_core, SimulateOptions};
            let options = SimulateOptions {
                iterations: Some(1000),
                ..SimulateOptions::default()
            };
            let result =
                simulate_core(Path::new("examples/bagel/bagel_simulate.yaml"), options).unwrap();
            let inputs = &result.input_samples;
            let profit = &result.outputs["year.profit"];

            // The model again, from bagel_simulate.yaml and costs.yaml
            let season = [
                0.85, 1.25, 0.80, 0.80, 0.90, 0.95, 1.05, 1.15, 1.25, 1.25, 1.05, 1.00,
            ];
            for i in 0..1000 {
                let flour = inputs["@costs.ingredients.flour_per_kg"][i];
                let wage = inputs["@costs.labour.wage_per_hour"][i];
                let opening = inputs["assumptions.opening_dozens_per_day"][i];
                let staff = inputs["assumptions.staff_hours_per_day"][i];
                let mut total = 0.0;
                for (m, se) in season.iter().enumerate() {
                    let dz = opening * 1.03_f64.powi(i32::try_from(m).unwrap()) * se * 30.0;
                    let r = dz * 16.0;
                    let c = dz * 12.0 / 9.0 * flour
                        + dz * 12.0 * 6.0 / 1000.0 * 9.40
                        + dz * 0.35
                        + staff * 30.0 * wage * 1.15
                        + 4800.0
                        + 950.0
                        + r * 0.026;
                    total += r - c;
                }
                assert!(close(total, profit.samples[i], 1e-6), "iteration {i}");
            }

            let thresholds: Vec<(&str, f64)> = profit
                .threshold_probabilities
                .iter()
                .map(|(t, p)| (t.as_str(), *p))
                .collect();
            assert_eq!(
                thresholds,
                [("> 0", 0.349), ("> 20000", 0.196), ("< -50000", 0.205)]
            );
            let drivers: Vec<(&str, f64)> = profit
                .drivers
                .iter()
                .map(|d| (d.input.as_str(), d.rank_correlation))
                .collect();
            for ((name, rho), (want_name, want)) in drivers.iter().zip([
                ("assumptions.opening_dozens_per_day", 0.845_145_641_146),
                ("assumptions.staff_hours_per_day", -0.417_853_641_854),
                ("@costs.labour.wage_per_hour", -0.322_749_874_750),
                ("@costs.ingredients.flour_per_kg", -0.098_687_702_688),
            ]) {
                assert_eq!(*name, want_name);
                assert!(close(*rho, want, 1e-11), "{name}: {rho}");
            }
            assert_eq!(drivers.len(), 4);
            // Outputs in the order configured
            let names: Vec<&String> = result.outputs.keys().take(3).collect();
            assert_eq!(
                names,
                [
                    "year.profit",
                    "year.worst_month_profit",
                    "months.profit (row 1)"
                ]
            );

            // The rent hike moves every sample by exactly 12 months of 800
            let hiked = simulate_core(
                Path::new("examples/bagel/bagel_simulate.yaml"),
                SimulateOptions {
                    scenario: Some("rent_hike"),
                    ..options
                },
            )
            .unwrap();
            for (a, b) in profit
                .samples
                .iter()
                .zip(&hiked.outputs["year.profit"].samples)
            {
                assert!(close(a - b, 9600.0, 1e-6), "{a} {b}");
            }
        }

        /// `forge simulate --seek`: the price at which 80% of runs make money.
        ///
        /// Checked outside Forge: profit is linear in the price, so each run has a
        /// break-even price (costs before card fees / (dozens x 0.974)), computed in
        /// numpy from Forge's input samples (1,000 runs, seed 7). The year makes
        /// money in 80% of runs just above the 800th lowest of them,
        /// 18.354117372865204 (the 801st is 18.359180292780614).
        #[test]
        fn price_for_an_eighty_percent_chance_of_profit() {
            use crate::cli::{probability_seek_core, SeekGoal, SimulateOptions};
            let goal = SeekGoal::parse("P(year.profit > 0) = 0.8").unwrap();
            let result = probability_seek_core(
                Path::new("examples/bagel/bagel_simulate.yaml"),
                SimulateOptions {
                    iterations: Some(1000),
                    ..SimulateOptions::default()
                },
                &goal,
                "price_per_dozen",
                14.0,
                24.0,
            )
            .unwrap();
            assert_eq!(result.input, "assumptions.price_per_dozen");
            assert!(
                result.value > 18.354_117_372_865_204
                    && result.value < 18.354_117_372_865_204 + 1e-4,
                "{}",
                result.value
            );
            assert!(close(result.probability, 0.8, 1e-12));
        }

        /// Each month as a simulation output, and the expected shortfall of the year.
        #[test]
        #[allow(clippy::suboptimal_flops)] // the model typed as in the Python check
        fn simulation_months_and_expected_shortfall() {
            use crate::cli::{simulate_core, SimulateOptions};
            let result = simulate_core(
                Path::new("examples/bagel/bagel_simulate.yaml"),
                SimulateOptions {
                    iterations: Some(1000),
                    ..SimulateOptions::default()
                },
            )
            .unwrap();
            let inputs = &result.input_samples;
            let profit = &result.outputs["year.profit"];
            let season = [
                0.85, 1.25, 0.80, 0.80, 0.90, 0.95, 1.05, 1.15, 1.25, 1.25, 1.05, 1.00,
            ];
            assert_eq!(result.outputs.len(), 2 + 12, "one output per month");

            // Each month's profit is the model's month, run by run
            for (m, se) in season.iter().enumerate() {
                let month = &result.outputs[&format!("months.profit (row {})", m + 1)];
                for i in [0, 499, 999] {
                    let opening = inputs["assumptions.opening_dozens_per_day"][i];
                    let dz = opening * 1.03_f64.powi(i32::try_from(m).unwrap()) * se * 30.0;
                    let r = dz * 16.0;
                    let c = dz * 12.0 / 9.0 * inputs["@costs.ingredients.flour_per_kg"][i]
                        + dz * 12.0 * 6.0 / 1000.0 * 9.40
                        + dz * 0.35
                        + inputs["assumptions.staff_hours_per_day"][i]
                            * 30.0
                            * inputs["@costs.labour.wage_per_hour"][i]
                            * 1.15
                        + 5750.0
                        + r * 0.026;
                    assert!(close(r - c, month.samples[i], 1e-6), "month {m} run {i}");
                }
            }
            // The mean of the worst 5% and 10% of years (numpy: sort, average the
            // lowest 50 and 100 of the 1,000)
            let mut sorted = profit.samples.clone();
            sorted.sort_by(f64::total_cmp);
            let worst =
                |k: usize| sorted[..k].iter().sum::<f64>() / f64::from(u32::try_from(k).unwrap());
            let shortfall: Vec<(u8, f64)> = profit.expected_shortfall.clone();
            assert_eq!(shortfall.len(), 2);
            assert!(close(shortfall[0].1, worst(50), 1e-9));
            assert!(close(shortfall[1].1, worst(100), 1e-9));
        }

        /// `price_war.yaml`, by hand: a month's margin from the plan's numbers,
        /// nashpy 0.0.43 for the equilibrium, and each match of the tournament
        /// as (1 - w) times its expected total.
        /// ```python
        /// food = 12 / 9 * 1.15 + 12 * 6 / 1000 * 9.40 + 0.35        # 2.5601333
        /// hold, cut = 16 * 0.974 - food, 14 * 0.974 - food          # 13.0238667, 11.0758667
        /// R, T = 0.5 * 110 * hold * 30, 0.70 * 110 * cut * 30       # 21489.38, 25585.252
        /// S, P = 0.30 * 110 * hold * 30, 0.5 * 115.5 * cut * 30     # 12893.628, 19188.939
        /// # nashpy: one equilibrium, both cut; with a share of 0.55, both hold
        /// # tit for tat or grim against always cut: (1 - w) S + w P = 18874.17339 at w = 0.95;
        /// # generous (forgive 1/3): (1 - w) S + w (S / 3 + 2 P / 3) = 16880.6583
        /// # each score is the mean over the five opponents, itself included
        /// ```
        /// axelrod 4.14.0 (4,000 seeded matches of each pair, `prob_end` 0.05,
        /// `GTFT(p=1/3)`) agrees within 1.19 standard errors.
        #[test]
        fn the_price_war_is_a_dilemma_that_matching_settles() {
            let file = Path::new("examples/bagel/price_war.yaml");
            let calculated = |scenario: Option<&str>| {
                let mut model = crate::parser::parse_model(file).unwrap();
                if let Some(name) = scenario {
                    crate::cli::commands::apply_scenario(&mut model, name).unwrap();
                }
                crate::core::ArrayCalculator::new(model)
                    .calculate_all()
                    .unwrap()
            };
            let value =
                |model: &crate::types::ParsedModel, name: &str| model.scalars[name].value.unwrap();
            let base = calculated(None);
            for (name, expected) in [
                ("market.R", 21_489.38),
                ("market.T", 25_585.252),
                ("market.S", 12_893.628),
                ("market.P", 19_188.939),
                ("war.cost_per_month", 2_300.441),
                ("war.cutting_dominates", 1.0),
                ("war.matching_holds_above", 0.640_348_901),
                ("price_war.equilibria", 1.0),
                ("price_war.equilibrium_1.lune.cut", 1.0),
                ("price_war.equilibrium_1.rival.cut", 1.0),
                ("rivalry.score.steady", 19_770.229_6),
                ("rivalry.score.matcher", 20_966.338_69),
                ("rivalry.score.grudge", 20_966.338_69),
                ("rivalry.score.discounter", 21_065.190_813_33),
                ("rivalry.score.forgiving", 20_567.635_66),
                ("rivalry.match.forgiving.discounter", 16_880.658_3),
            ] {
                assert!(close(value(&base, name), expected, 1e-6), "{name}");
            }
            let stable = |model: &crate::types::ParsedModel| -> Vec<f64> {
                ["steady", "matcher", "grudge", "discounter", "forgiving"]
                    .iter()
                    .map(|p| value(model, &format!("rivalry.stable.{p}")))
                    .collect()
            };
            assert_eq!(stable(&base), [0.0, 1.0, 1.0, 1.0, 0.0]);
            // Loyal regulars: cutting no longer pays, and the discounter comes last
            let loyal = calculated(Some("loyal_regulars"));
            assert!(close(value(&loyal, "market.T"), 20_102.698, 1e-6));
            assert!(close(value(&loyal, "war.cutting_dominates"), 0.0, 0.0));
            assert!(close(value(&loyal, "war.matching_holds_above"), 0.0, 0.0));
            assert!(close(
                value(&loyal, "price_war.equilibrium_1.lune.hold"),
                1.0,
                0.0
            ));
            assert!(close(
                value(&loyal, "rivalry.score.discounter"),
                19_456.974_973_33,
                1e-6
            ));
            assert_eq!(stable(&loyal), [1.0, 1.0, 1.0, 0.0, 1.0]);
            // A lease ending: at w = 0.6, below 0.640, matching no longer holds
            let ending = calculated(Some("lease_ending"));
            assert!(close(
                value(&ending, "rivalry.score.discounter"),
                22_259.169_24,
                1e-6
            ));
            assert!(close(
                value(&ending, "rivalry.score.matcher"),
                20_525.666_92,
                1e-6
            ));
            assert_eq!(stable(&ending), [0.0, 0.0, 0.0, 1.0, 0.0]);
        }
    }

    mod demo_examples {
        //! The headline numbers of examples/mortgage and examples/household, as
        //! computed outside Forge.
        //!
        //! The mortgage, as a month-by-month amortisation loop (no closed forms):
        //! ```python
        //! r = (1 + 0.0479 / 2) ** (1 / 6) - 1        # semi-annual compounding
        //! P, n = 450000, 300
        //! pay = P * r / (1 - (1 + r) ** -n)            # 2563.6876295653
        //! def run(pmt):
        //!     bal, months, interest, y1i, y1p = P, 0, 0.0, 0.0, 0.0
        //!     while bal > 1e-9:
        //!         i = bal * r; p = min(pmt, bal + i)
        //!         bal = bal + i - p; months += 1; interest += i
        //!         if months <= 12: y1i += i; y1p += p - i
        //!     return months, p, interest, y1i, y1p
        //! run(pay)        # interest 319106.2888696
        //! # the extra payment that saves 80,000 of interest, as in the README
        //! brentq(lambda e: run(pay)[2] - run(pay + e)[2] - 80000, 1, 2000)  # 397.044208
        //! run(pay + 300)  # 247 payments, the last 42.248775, interest 254509.4056479,
        //!                 # first year interest 21056.173122, principal 13308.078432
        //! ```
        //! The `schedule` table of the example is that loop, written with PREV; the
        //! balances the loop prints for months 1, 12, 60, 120, 246 and 247 are in the
        //! test below.
        //!
        //! examples/periods, in Python:
        //! ```python
        //! # monthly_plan.yaml
        //! season = [0.85, 0.80, 0.95, 1.00, 1.05, 1.10, 0.90, 0.85, 1.00, 1.05, 1.15, 1.30]
        //! trend, cash, sales, profit, low = 18000, 12000, 0, 0, 1e9
        //! for m in range(12):
        //!     if m: trend *= 1.01
        //!     s = trend * season[m]; p = s * (1 - 0.42) - 9500
        //!     cash += p; sales += s; profit += p; low = min(low, cash)
        //! # sales 229045.2297709736, profit 18846.23326716473, cash 30846.23326716473,
        //! # lowest 10309.52, two months at a loss
        //! # dcf.yaml
        //! rev, pv = 4200000, 0
        //! for t in range(1, 6):
        //!     rev *= 1.08; f = rev * 0.18 * (1 - 0.265) * (1 - 0.30)
        //!     df = 1 / 1.095 ** t; pv += f * df
        //! tv = f * 1.025 / (0.095 - 0.025)
        //! # pv 1866331.1790654135, tv 8368580.101335552, tv * df 5315953.599495626,
        //! # equity 5682284.77856104
        //! ```
        //!
        //! examples/options/compare-options.yaml, in Python:
        //! ```python
        //! T, cap = 4800, 0.35
        //! for rent, util, net, transit, move_in, inc in options:
        //!     mo = rent + util + net + transit
        //!     row = (mo, mo * 12 + move_in, rent * (1 + inc) + util + net + transit,
        //!            T - mo, mo / T, T * cap - mo)
        //! # maple_studio    1582 19884 1622.5             3218 0.32958333333333334  98
        //! # harbour_one_bed 1650 21000 1689.8749999999998 3150 0.34375              30
        //! # park_loft       1752 21674 1811.2             3048 0.365               -72
        //! # river_studio    1542 20004 1587.1499999999999 3258 0.32125             138
        //! # garden_one_bed  1660 20320 1736.25            3140 0.3458333333333333   20
        //! # lowest monthly 1542, lowest first year 19884
        //! ```
        //!
        //! The household: income 5200 + 400 = 5600 budgeted, 5200 + 150 = 5350
        //! actual; expenses 3100 budgeted, 1650 + 812.43 + 96.50 + 163.20 + 120 +
        //! 388.75 + 74.97 = 3305.85 actual; left over 2500 and 2044.15.
        //!
        //! examples/rolling-forecast, in Python:
        //! ```python
        //! su, sp = [985, 1032, 1001, 1047, 1052, 1038, 1066], [50, 50, 49.5, 49.5, 49.8, 49.8, 50.1]
        //! pu, pp = [214, 220, 231, 228, 240, 244, 251], [118, 118, 119, 119, 119.5, 119.5, 120]
        //! for _ in range(12):   # November 2026 to October 2027, grown from the month before
        //!     su.append(su[-1] * 1.008); sp.append(sp[-1] * 1.002)
        //!     pu.append(pu[-1] * 1.015); pp.append(pp[-1] * 1.003)
        //! rev = [su[i] * sp[i] + pu[i] * pp[i] for i in range(19)]
        //! fy27 = sum(rev[:12])                                    # 987621.9306764038
        //! bs, bp = sum(1000 + 10 * i for i in range(12)), sum(200 + 5 * i for i in range(12))
        //! budget = bs * 50 + bp * 120                             # 960600
        //! quarters = [sum(rev[k:k + 3]) for k in (0, 3, 6, 9)]   # 229100.5, 240878.5,
        //! #                                   253830.2732159771, 263812.6574604267
        //! next_12 = sum(rev[7:])                                  # 1090944.3557895026
        //! ls, lp = sum(su[:12]), sum(pu[:12]); L, B = ls + lp, bs + bp
        //! volume = (L - B) * budget / B                          # 14412.743557430787
        //! mix = L * ((ls / L - bs / B) * 50 + (lp / L - bp / B) * 120)   # 11875.965095038919
        //! price = sum(su[i] * sp[i] - su[i] * 50 + pu[i] * pp[i] - pu[i] * 120 for i in range(12))
        //! #                                                       # 733.22202393...
        //! ```

        use crate::cli::commands::{render, report_core, Cell, ReportFormat, ReportOptions};
        use crate::cli::{calculate_core, goal_seek_core, variance_core};
        use crate::core::ArrayCalculator;
        use crate::ColumnValue;
        use std::path::Path;

        fn close(a: f64, b: f64, tol: f64) -> bool {
            (a - b).abs() <= tol
        }

        #[test]
        fn the_mortgage_matches_an_amortisation_loop() {
            let result =
                calculate_core(Path::new("examples/mortgage/mortgage.yaml"), true, None).unwrap();
            let get = |name: &str| result.scalars[name].unwrap();
            for (name, expected) in [
                ("terms.payment", 2_563.687_629_565_3),
                ("as_agreed.total_interest", 319_106.288_869_6),
                ("with_extra.last_payment", 42.248_774_9),
                ("with_extra.total_interest", 254_509.405_647_9),
                ("saved.interest", 64_596.883_221_6),
                ("saved.months", 53.0),
                ("year_one.interest", 21_056.173_122_35),
                ("year_one.principal_paid", 13_308.078_432_43),
            ] {
                assert!(close(get(name), expected, 1e-5), "{name}: {}", get(name));
            }
            // 246 full payments and a smaller last one: 247, as in the loop
            assert!(close(get("with_extra.full_months"), 246.0, 0.0));
            // The schedule and the closed forms agree
            for name in ["check.interest", "check.last_payment", "check.payments"] {
                assert!(close(get(name), 0.0, 0.0), "{name}: {}", get(name));
            }
            assert!(close(get("paid_off.payments"), 247.0, 0.0));
            assert!(close(get("paid_off.year"), 2047.0, 0.0));
            assert!(close(get("paid_off.month_of_year"), 6.0, 0.0));
        }

        fn numbers(values: &ColumnValue) -> &[f64] {
            match values {
                ColumnValue::Number(v) => v,
                other => panic!("not numbers: {other:?}"),
            }
        }

        /// T2-TIME: the schedule, written with PREV, is the Python loop month by
        /// month, to well under a cent.
        #[test]
        fn the_mortgage_schedule_matches_the_loop_month_by_month() {
            let model =
                crate::parser::parse_model(Path::new("examples/mortgage/mortgage.yaml")).unwrap();
            let model = ArrayCalculator::new(model).calculate_all().unwrap();
            let schedule = &model.tables["schedule"];
            let column = |name: &str| numbers(&schedule.columns[name].values);
            assert_eq!(column("closing").len(), 300);
            // (month, opening, interest, payment, closing) from the loop
            for (month, opening, interest, payment, closing) in [
                (
                    1,
                    450_000.0,
                    1_778.582_904_770_109_5,
                    2_863.687_629_565_315_5,
                    448_914.895_275_204_8,
                ),
                (
                    12,
                    437_825.146_262_861,
                    1_730.462_934_270_216_5,
                    2_863.687_629_565_315_5,
                    436_691.921_567_565_9,
                ),
                (
                    60,
                    378_057.801_437_760_8,
                    1_494.238_094_782_608_9,
                    2_863.687_629_565_315_5,
                    376_688.351_902_978_15,
                ),
                (
                    120,
                    285_535.300_994_710_65,
                    1_128.551_566_794_622_2,
                    2_863.687_629_565_315_5,
                    283_800.164_931_94,
                ),
                (
                    246,
                    2_894.330_506_949_832,
                    11.439_570_578_701_28,
                    2_863.687_629_565_315_5,
                    42.082_447_963_217_874,
                ),
                (
                    247,
                    42.082_447_963_217_874,
                    0.166_326_938_973_904_52,
                    42.248_774_902_191_78,
                    0.0,
                ),
                (248, 0.0, 0.0, 0.0, 0.0),
                (300, 0.0, 0.0, 0.0, 0.0),
            ] {
                for (name, expected) in [
                    ("opening", opening),
                    ("interest", interest),
                    ("payment", payment),
                    ("closing", closing),
                ] {
                    let got = column(name)[month - 1];
                    assert!(close(got, expected, 1e-6), "month {month} {name}: {got}");
                }
            }
            let total = |name: &str| column(name).iter().sum::<f64>();
            assert!(close(total("interest"), 254_509.405_647_968_52, 1e-5));
            assert!(close(total("payment"), 704_509.405_647_969_8, 1e-5));
            match &schedule.columns["month"].values {
                ColumnValue::Date(dates) => {
                    assert_eq!(dates[0], "2026-12-01");
                    assert_eq!(dates[246], "2047-06-01");
                },
                other => panic!("{other:?}"),
            }
        }

        #[test]
        fn the_period_examples_match_python() {
            let plan = calculate_core(Path::new("examples/periods/monthly_plan.yaml"), true, None)
                .unwrap();
            for (name, expected) in [
                ("year.sales", 229_045.229_770_973_6),
                ("year.profit", 18_846.233_267_164_73),
                ("year.closing_cash", 30_846.233_267_164_73),
                ("year.lowest_cash", 10_309.52),
                ("year.months_at_a_loss", 2.0),
            ] {
                let got = plan.scalars[name].unwrap();
                assert!(close(got, expected, 1e-8), "{name}: {got}");
            }
            let dcf = calculate_core(Path::new("examples/periods/dcf.yaml"), true, None).unwrap();
            for (name, expected) in [
                ("valuation.forecast_years", 1_866_331.179_065_413_5),
                ("valuation.terminal_value", 8_368_580.101_335_552),
                ("valuation.terminal_value_today", 5_315_953.599_495_626),
                ("valuation.equity_value", 5_682_284.778_561_04),
                ("valuation.check", 0.0),
            ] {
                let got = dcf.scalars[name].unwrap();
                assert!(close(got, expected, 1e-6), "{name}: {got}");
            }
        }

        #[test]
        fn the_extra_payment_that_saves_eighty_thousand() {
            let result = goal_seek_core(
                Path::new("examples/mortgage/mortgage.yaml"),
                "saved.interest",
                80_000.0,
                "loan.extra_per_month",
                (None, None),
                0.0001,
            )
            .unwrap();
            assert!(result.converged);
            assert!(
                close(result.solution, 397.044_208, 1e-4),
                "{}",
                result.solution
            );
        }

        #[test]
        fn the_household_variance_has_the_right_directions_and_totals() {
            let report = variance_core(
                Path::new("examples/household/budget.yaml"),
                Path::new("examples/household/actual.yaml"),
                10.0,
            )
            .unwrap();
            let get = |name: &str| {
                report
                    .results
                    .iter()
                    .find(|r| r.name == name)
                    .unwrap_or_else(|| panic!("{name}"))
            };
            for (name, budget, actual, better, favourable) in [
                // On target: neither favorable nor unfavorable
                ("income.salary", 5200.0, 5200.0, "higher", None),
                ("income.side_work", 400.0, 150.0, "higher", Some(false)),
                ("income.total", 5600.0, 5350.0, "higher", Some(false)),
                ("expenses.transport", 180.0, 96.5, "lower", Some(true)),
                ("expenses.eating_out", 250.0, 388.75, "lower", Some(false)),
                ("expenses.total", 3100.0, 3305.85, "lower", Some(false)),
                ("result.left_over", 2500.0, 2044.15, "higher", Some(false)),
            ] {
                let r = get(name);
                assert!(close(r.budget, budget, 1e-9), "{name} budget {}", r.budget);
                assert!(close(r.actual, actual, 1e-9), "{name} actual {}", r.actual);
                assert_eq!(r.better, better, "{name}");
                assert_eq!(r.is_favorable, favourable, "{name}");
            }
            // Over the 10% threshold: side work, groceries, transport, utilities,
            // eating out, subscriptions, left over and the savings rate
            let alerts: Vec<&str> = report
                .results
                .iter()
                .filter(|r| r.exceeds_threshold)
                .map(|r| r.name.as_str())
                .collect();
            assert_eq!(
                alerts,
                [
                    "income.side_work",
                    "expenses.groceries",
                    "expenses.transport",
                    "expenses.utilities",
                    "expenses.eating_out",
                    "expenses.subscriptions",
                    "result.left_over",
                    "result.savings_rate",
                ]
            );
        }

        // The fund's last exit is the latest of its exit dates, a date (ADR-060
        // addendum, roadmap T1-DATE-OF-MAX). The README's Python gives it as
        // `max(d for d, _ in exits)`: Lumen, 78 months after
        // 2023-03-08, which is 2029-09-08 (the day number 47369).
        #[test]
        fn the_funds_last_exit_is_a_date() {
            let model = calculate_core(Path::new("examples/fund/fund.yaml"), true, None).unwrap();
            assert_eq!(model.scalars["fund.last_exit"], Some(47369.0));
            assert!(model.dates.contains("fund.last_exit"));
            assert_eq!(model.shown("fund.last_exit").as_deref(), Some("2029-09-08"));
            assert_eq!(model.formatted["fund.last_exit"], "8 Sep 2029");
        }

        #[test]
        fn the_rolling_forecast_matches_python() {
            let model = calculate_core(
                Path::new("examples/rolling-forecast/forecast.yaml"),
                true,
                None,
            )
            .unwrap();
            for (name, expected) in [
                ("fy2027.revenue", 987_621.930_676_403_8),
                ("fy2027.budget", 960_600.0),
                ("fy2027.variance", 27_021.930_676_403_8),
                ("fy2027.actual_so_far", 553_505.6),
                ("next_12_months.revenue", 1_090_944.355_789_502_6),
                ("pvm.volume", 14_412.743_557_430_787),
                ("pvm.mix", 11_875.965_095_038_919),
                ("pvm.price", 733.222_023_934),
                ("pvm.check", 0.0),
            ] {
                let got = model.scalars[name].unwrap();
                assert!(close(got, expected, 1e-6), "{name}: {got}");
            }
            // The table of quarters (ADR-058): the quarters of the same Python, and
            // the budget's months by quarter, by hand (Q1: 3,030 x 50 + 615 x 120)
            let quarter = |column: &str| -> Vec<f64> {
                model.tables["quarters"].values[column]
                    .iter()
                    .map(|v| v.as_f64().unwrap())
                    .collect()
            };
            let revenue = [
                229_100.5,
                240_878.5,
                253_830.273_215_977_1,
                263_812.657_460_426_7,
            ];
            let budget = [225_300.0, 235_200.0, 245_100.0, 255_000.0];
            for (i, (r, b)) in revenue.iter().zip(&budget).enumerate() {
                assert!(close(quarter("revenue")[i], *r, 1e-6), "quarter {}", i + 1);
                assert!(close(quarter("budget")[i], *b, 1e-9), "quarter {}", i + 1);
                assert!(
                    close(quarter("variance")[i], r - b, 1e-6),
                    "quarter {}",
                    i + 1
                );
                let share = r / 987_621.930_676_403_8;
                assert!(
                    close(quarter("share_of_year")[i], share, 1e-9),
                    "quarter {}",
                    i + 1
                );
            }
        }

        #[test]
        fn each_demo_calculates_in_under_a_second() {
            for model in [
                "examples/mortgage/mortgage.yaml",
                "examples/household/budget.yaml",
                "examples/household/actual.yaml",
                "examples/bagel/bagel.yaml",
                "examples/periods/monthly_plan.yaml",
                "examples/periods/dcf.yaml",
                "examples/options/compare-options.yaml",
                "examples/rolling-forecast/forecast.yaml",
            ] {
                let start = std::time::Instant::now();
                calculate_core(Path::new(model), true, None).unwrap();
                let took = start.elapsed();
                // A debug build is slower than a release build; a second is still ample
                assert!(took.as_secs_f64() < 1.0, "{model}: {took:?}");
            }
        }

        /// Every model under examples/ passes the schema of its format (the parser
        /// checks it before anything else) and parses.
        #[test]
        fn every_example_passes_its_schema() {
            fn models(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
                for entry in std::fs::read_dir(dir).unwrap() {
                    let path = entry.unwrap().path();
                    if path.is_dir() {
                        models(&path, out);
                    } else if path.extension().is_some_and(|e| e == "yaml") {
                        out.push(path);
                    }
                }
            }
            let mut files = Vec::new();
            models(Path::new("examples"), &mut files);
            assert!(files.len() >= 20, "{files:?}");
            for file in files {
                if let Err(e) = crate::parser::parse_model(&file) {
                    panic!("{}: {e}", file.display());
                }
            }
        }

        /// T2-COMPARE-EXAMPLE: forge calculate, then forge report, gives Python's
        /// numbers, the five options in one table and the summary in its own.
        #[test]
        fn the_comparison_example_matches_python_through_report() {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("compare-options.yaml");
            std::fs::copy("examples/options/compare-options.yaml", &file).unwrap();
            calculate_core(&file, false, None).unwrap();
            let before = std::fs::read_to_string(&file).unwrap();
            let report = report_core(
                &file,
                &ReportOptions {
                    no_inputs: true,
                    ..ReportOptions::default()
                },
            )
            .unwrap();
            assert_eq!(std::fs::read_to_string(&file).unwrap(), before);

            let expected: [(&str, [f64; 6]); 5] = [
                (
                    "maple_studio",
                    [
                        1582.0,
                        19884.0,
                        1622.5,
                        3218.0,
                        0.329_583_333_333_333_34,
                        98.0,
                    ],
                ),
                (
                    "harbour_one_bed",
                    [
                        1650.0,
                        21000.0,
                        1_689.874_999_999_999_8,
                        3150.0,
                        0.34375,
                        30.0,
                    ],
                ),
                ("park_loft", [1752.0, 21674.0, 1811.2, 3048.0, 0.365, -72.0]),
                (
                    "river_studio",
                    [
                        1542.0,
                        20004.0,
                        1_587.149_999_999_999_9,
                        3258.0,
                        0.32125,
                        138.0,
                    ],
                ),
                (
                    "garden_one_bed",
                    [
                        1660.0,
                        20320.0,
                        1736.25,
                        3140.0,
                        0.345_833_333_333_333_3,
                        20.0,
                    ],
                ),
            ];
            assert_eq!(report.sections.len(), 2, "{report:?}");
            let options = &report.sections[0];
            assert_eq!(
                options.keys,
                [
                    "monthly",
                    "first_year",
                    "second_year_monthly",
                    "left_each_month",
                    "share_of_pay",
                    "room_under_cap"
                ]
            );
            assert_eq!(options.rows.len(), 5);
            for (row, (group, values)) in options.rows.iter().zip(expected) {
                assert_eq!(row.group, group);
                for (cell, want) in row.cells.iter().zip(values) {
                    let Cell::Value(got) = cell else {
                        panic!("{group}: {cell:?}")
                    };
                    assert!(close(*got, want, 1e-9), "{group}: {got} vs {want}");
                }
            }
            let summary = &report.sections[1];
            assert_eq!(summary.rows[0].group, "summary");
            assert_eq!(
                summary.rows[0].cells,
                [Cell::Value(1542.0), Cell::Value(19884.0)]
            );
            assert_eq!(report.uncalculated, Vec::<String>::new());

            // docs/REPORT.md shows this report as Markdown, as it is printed.
            let doc = std::fs::read_to_string("docs/REPORT.md").unwrap();
            let markdown = render(&report, ReportFormat::Markdown);
            assert!(
                doc.contains(&markdown),
                "docs/REPORT.md must show:\n{markdown}"
            );
        }

        /// examples/capex-plan against a month-by-month loan and a least-squares fit.
        /// In Python: the loan loop gives payment 2347.9377862474407, year-2 interest
        /// 5775.928291150727 and principal 22399.32514381856; with numpy,
        /// `polyfit(x, y, 1)` is [6300, 34500] and `(66000 / 41000) ** .25 - 1` is
        /// 0.12639319927790216.
        #[test]
        fn the_capex_plan_matches_python() {
            let result =
                calculate_core(Path::new("examples/capex-plan/capex.yaml"), true, None).unwrap();
            let value = |name: &str| result.scalars[name].unwrap();
            assert!(close(value("bank.payment"), 2_347.937_786_247_440_7, 1e-9));
            assert!(close(
                value("bank.year2_interest"),
                5_775.928_291_150_727,
                1e-9
            ));
            assert!(close(
                value("bank.year2_principal"),
                22_399.325_143_818_56,
                1e-9
            ));
            assert!(value("bank.check").abs() < 1e-9);
            assert!(close(value("writedown.straight_line"), 13_500.0, 1e-12));
            assert!(close(value("writedown.sum_of_years"), 18_000.0, 1e-12));
            assert!(close(value("writedown.double_declining"), 16_875.0, 1e-12));
            // 17.49% of 120,000, the third rate of the IRS table
            assert!(close(value("writedown.tax_macrs"), 20_988.0, 1e-12));
            assert!(close(value("writedown.total"), 108_000.0, 1e-12));
            assert!(close(value("growth.cagr"), 0.126_393_199_277_902_16, 1e-9));
            assert!(close(value("growth.slope"), 6_300.0, 1e-9));
            assert!(close(value("growth.year6"), 72_300.0, 1e-9));
        }
    }
}
