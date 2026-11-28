//! `forge decision-tree`: the best choice at each decision, by backward
//! induction.

use super::write_result;
use crate::cli::console::outln;
use crate::cli::Console;
use crate::core::precision::money;
use crate::decision_trees::{DecisionTreeConfig, DecisionTreeEngine};
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use colored::Colorize;
use std::path::{Path, PathBuf};

/// Analyze a decision tree and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no `decision_tree` section exists,
/// or the analysis fails.
pub fn decision_tree_core(file: &Path) -> ForgeResult<crate::decision_trees::TreeResult> {
    let yaml_content = parser::read_model_text(file)?;

    let value: yaml_serde::Value = parser::read_yaml(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let config: DecisionTreeConfig = if let Some(dt_value) = value.get("decision_tree") {
        yaml_serde::from_value(dt_value.clone())
            .map_err(|e| ForgeError::Validation(format!("decision_tree config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'decision_tree' section found in YAML".to_string(),
        ));
    };

    let engine = DecisionTreeEngine::new(config).map_err(ForgeError::Validation)?;
    engine.analyze().map_err(ForgeError::Eval)
}

/// Execute the decision-tree command - backward induction analysis
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no `decision_tree` section exists,
/// or the analysis fails.
pub fn decision_tree(
    c: &mut Console<'_>,
    file: &Path,
    export_dot: bool,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "🌳 Forge - Decision Tree Analysis".bold().green());
    outln!(c, "   File: {}", file.display());
    outln!(c);

    let result = decision_tree_core(file)?;

    // Display config
    outln!(c, "   {}", format!("Tree: {}", result.name).bold());
    if verbose {
        outln!(c, "      Root: {:?}", Some(&result.root_name));
        outln!(c, "      Nodes: {}", result.node_count);
    }
    outln!(c);

    if verbose {
        outln!(c, "{}", "🔄 Running backward induction...".cyan());
    }

    print_decision_tree_result(c, &result);

    // Export DOT if requested (feature not yet implemented, just note it)
    if export_dot {
        outln!(
            c,
            "{}",
            "⚠️  DOT export not yet implemented. Use --output for YAML export.".yellow()
        );
    }

    if let Some(output_path) = output_file {
        write_result(c, &result, &output_path)?;
    }

    outln!(c, "{}", "✅ Decision tree analysis complete".bold().green());
    Ok(())
}

/// What backward induction found: the optimal path, its expected value, the
/// choice at each decision, and the outcomes the optimal policy can lead to.
fn print_decision_tree_result(c: &mut Console<'_>, result: &crate::decision_trees::TreeResult) {
    // Display results
    outln!(c, "{}", "📊 Decision Tree Results:".bold().green());
    outln!(c);

    // Optimal path
    outln!(c, "   {}", "Optimal Path:".bold());
    for step in &result.optimal_path {
        outln!(c, "      → {}", step.cyan());
    }
    outln!(c);

    // Expected value at root
    outln!(
        c,
        "   Expected Value: {}",
        money(result.root_expected_value).bold().green()
    );

    // Decision policy
    outln!(c);
    outln!(c, "   {}", "Decision Policy:".bold());
    for (node, choice) in &result.decision_policy {
        outln!(
            c,
            "      At \"{}\": choose \"{}\"",
            node.bright_blue(),
            choice
        );
        if let Some(tied) = result
            .node_results
            .get(node)
            .filter(|n| !n.tied_with.is_empty())
        {
            outln!(c, "         {} equally good: {} (the expected values are the same; the choice is not made by the numbers)",
                "⚠️".yellow(),
                tied.tied_with.join(", ")
            );
        }
    }

    // Risk profile
    outln!(c);
    outln!(c, "   {}", "Risk Profile:".bold());
    outln!(
        c,
        "      Best case:  {}",
        money(result.risk_profile.best_case)
    );
    outln!(
        c,
        "      Worst case: {}",
        money(result.risk_profile.worst_case)
    );
    outln!(
        c,
        "      P(value > 0): {:.1}%",
        result.risk_profile.probability_positive * 100.0
    );
    outln!(c, "      Outcomes under the optimal policy:");
    for outcome in &result.risk_profile.outcomes {
        outln!(
            c,
            "         {} with probability {:.1}%: {}",
            money(outcome.value),
            outcome.probability * 100.0,
            outcome.path.join(" → ")
        );
    }
    outln!(c);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::console::capture;
    use std::fs;

    #[test]
    fn test_decision_tree_config_parsing() {
        let yaml = r#"
_forge_version: "9.0.0"

decision_tree:
  name: "Test Tree"
  root:
    type: decision
    name: "Start"
    branches:
      yes:
        value: 100
      no:
        value: 0
"#;
        let value: yaml_serde::Value = yaml_serde::from_str(yaml).unwrap();
        assert!(value.get("decision_tree").is_some());
    }

    #[test]
    fn test_decision_tree_output_is_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("t.yaml");
        let example = std::fs::read_to_string("examples/decision-tree.yaml").unwrap();
        std::fs::write(&model, example).unwrap();
        let out = dir.path().join("out.yaml");
        capture(|c| decision_tree(c, &model, false, Some(out.clone()), false))
            .0
            .unwrap();
        let doc: yaml_serde::Value =
            yaml_serde::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert!(doc["root_expected_value"].as_f64().is_some());
    }

    fn model(text: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.yaml");
        fs::write(&path, text).unwrap();
        (dir, path)
    }

    const TIED: &str = "decision_tree:
  name: Tied
  root:
    type: decision
    name: Pick
    branches:
      a: {value: 10}
      b: {value: 10}
";

    fn refusals() -> Vec<(tempfile::TempDir, PathBuf, &'static str)> {
        let cases = [
            ("a: 1\n", "No 'decision_tree' section"),
            ("a: [\n", "YAML parse error"),
            ("decision_tree: 5\n", "decision_tree config error"),
            (
                "decision_tree:\n  root:\n    type: decision\n    branches: {}\n",
                "has no branches",
            ),
        ];
        cases
            .into_iter()
            .map(|(text, expected)| {
                let (dir, path) = model(text);
                (dir, path, expected)
            })
            .collect()
    }

    #[test]
    fn core_and_command_refuse_a_model_that_is_not_a_tree() {
        for (_dir, path, expected) in refusals() {
            let error = decision_tree_core(&path).unwrap_err().to_string();
            assert!(error.contains(expected), "{expected}: {error}");
            let (result, _) = capture(|c| decision_tree(c, &path, false, None, false));
            let error = result.unwrap_err().to_string();
            assert!(error.contains(expected), "{expected}: {error}");
        }
        let missing = Path::new("no/such/model.yaml");
        assert!(decision_tree_core(missing).is_err());
        let (result, _) = capture(|c| decision_tree(c, missing, false, None, false));
        assert!(result.is_err());
    }

    #[test]
    fn verbose_names_the_root_and_the_nodes_and_dot_is_not_implemented() {
        let (result, out) = capture(|c| {
            decision_tree(
                c,
                Path::new("examples/bagel/second_shop.yaml"),
                true,
                None,
                true,
            )
        });
        result.unwrap();
        for expected in [
            "Root: Some(",
            "Nodes: ",
            "Running backward induction...",
            "DOT export not yet implemented",
            "Decision tree analysis complete",
        ] {
            assert!(out.out.contains(expected), "{expected}\n{}", out.out);
        }
    }

    #[test]
    fn a_tie_is_said_out_loud() {
        let (_dir, path) = model(TIED);
        let (result, out) = capture(|c| decision_tree(c, &path, false, None, false));
        result.unwrap();
        assert!(out.out.contains("equally good: b"), "{}", out.out);
        assert!(out.out.contains("Expected Value: $10.00"), "{}", out.out);
    }

    #[test]
    fn an_unwritable_output_is_an_io_error() {
        let (_dir, path) = model(TIED);
        let out = PathBuf::from("no/such/dir/out.yaml");
        let (result, _) = capture(|c| decision_tree(c, &path, false, Some(out), false));
        assert_eq!(
            result.unwrap_err().to_string(),
            "IO error: No such file or directory (os error 2)"
        );
    }

    #[test]
    fn a_branch_with_nothing_on_it_is_an_error_from_the_analysis() {
        let (_dir, path) = model(
            "decision_tree:\n  name: T\n  root:\n    type: decision\n    name: Pick\n    branches:\n      a: {}\n",
        );
        let (result, _) = capture(|c| decision_tree(c, &path, false, None, false));
        assert_eq!(
            result.unwrap_err().to_string(),
            "Formula evaluation error: Branch has neither value nor next node"
        );
    }

    mod printed_numbers {
        //! The money `forge decision-tree` prints
        //! follow `forge report`'s rule (Session Prompt part 1, item 5): two decimals
        //! and thousands separators, four significant digits under 1, and money with
        //! the sign before the `$`. JSON and files keep the raw numbers.

        use crate::cli::forge;

        #[test]
        fn a_decision_tree_prints_money_with_separators() {
            let text = forge(&["decision-tree", "examples/bagel/second_shop.yaml"]).printed();
            for expected in [
                "Expected Value: $61,500.00",
                "Best case:  $198,000.00",
                "Worst case: -$117,000.00",
                "-$12,000.00 with probability 50.0%",
            ] {
                assert!(text.contains(expected), "{expected}\n{text}");
            }
            assert!(!text.contains("$-"), "{text}");
        }

        #[test]
        fn the_json_of_an_analysis_keeps_the_numbers_not_the_display() {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("tree.json");
            let printed = forge(&[
                "decision-tree",
                "examples/bagel/second_shop.yaml",
                "-o",
                file.to_str().unwrap(),
            ])
            .printed();
            assert!(printed.contains("Results written to"), "{printed}");
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
            assert_eq!(json["root_expected_value"], 61_500.0);
        }
    }

    mod microgrid_example {
        //! `examples/microgrid/microgrid_decision.yaml`: which system for Ilha
        //! Corva, and now or in a year.
        //!
        //! Its 36 numbers are microgrid.yaml calculated for each branch, so the
        //! first test calculates them again and fails when the model and the
        //! tree part. The expected values are the `run` of the Python in
        //! prediction/scenarios.rs (`microgrid_example`), branch by branch:
        //! ```python
        //! # now: (capex, npv + capex); a year later, with demand 4305, diesel
        //! # grown a year and the battery at 266000 or 380000 a MWh:
        //! # (capex / 1.08, (npv + capex - year 20's present value) / 1.08)
        //! # diesel cheap / plan / shock 0.25 / 0.5 / 0.25, sun 1550 / 1350 0.7 / 0.3
        //! # expected net value: big now 13792484.177027194, small now 12826402.581436146,
        //! # wait 0.5 * 14349941.950175395 + 0.5 * 13045396.067948803 = 13697669.0090621
        //! ```

        use crate::cli::decision_tree_core;
        use crate::core::ArrayCalculator;
        use std::path::Path;

        const MODEL: &str = "examples/microgrid/microgrid.yaml";
        const TREE: &str = "examples/microgrid/microgrid_decision.yaml";

        fn close(a: f64, b: f64, tol: f64) -> bool {
            (a - b).abs() <= tol
        }

        /// The model with these inputs: capex, NPV and the present value of
        /// year 20's cash.
        fn island(inputs: &[(&str, f64)]) -> (f64, f64, f64) {
            let mut model = crate::parser::parse_model(Path::new(MODEL)).unwrap();
            for (name, value) in inputs {
                model.override_scalar(name, Some(*value), None).unwrap();
            }
            let result = ArrayCalculator::new(model).calculate_all().unwrap();
            let scalar = |name: &str| result.scalars[name].value.unwrap();
            let column = |name: &str| {
                result.tables["money"].columns[name]
                    .values
                    .numbers()
                    .unwrap()
                    .to_vec()
            };
            let last_year = column("cfads")[19] * column("discount")[19];
            (
                scalar("capex.total"),
                scalar("verdict.project_npv"),
                last_year,
            )
        }

        #[test]
        fn every_number_in_the_tree_is_the_model_for_that_branch() {
            let text = std::fs::read_to_string(TREE).unwrap();
            let doc: yaml_serde::Value = yaml_serde::from_str(&text).unwrap();
            let tree = &doc["decision_tree"];
            let systems = [("small", 2.8, 9.0), ("big", 3.4, 10.0)];
            let diesel = [
                ("cheap", 1.10, 0.01),
                ("plan", 1.45, 0.03),
                ("shock", 1.95, 0.045),
            ];
            let sun = [("sunny", 1550.0), ("cloudy", 1350.0)];
            let mut checked = 0;
            for (system, mwp, mwh) in systems {
                for timing in ["now", "fall", "flat"] {
                    for (outlook, price, growth) in diesel {
                        for (weather, yield_per_mwp) in sun {
                            let mut inputs = vec![
                                ("system.solar_mwp", mwp),
                                ("system.battery_mwh", mwh),
                                ("fuel.price_escalation", growth),
                                ("system.solar_yield_mwh_per_mwp", yield_per_mwp),
                            ];
                            let (cost, value) = if timing == "now" {
                                inputs.push(("fuel.diesel_price", price));
                                let (capex, npv, _) = island(&inputs);
                                let cost = &tree["root"]["branches"][format!("build_{system}_now")];
                                (capex - cost["cost"].as_f64().unwrap(), npv + capex)
                            } else {
                                inputs.push(("fuel.diesel_price", price * (1.0 + growth)));
                                inputs.push(("demand.first_year_mwh", 4305.0));
                                let battery = if timing == "fall" {
                                    266_000.0
                                } else {
                                    380_000.0
                                };
                                inputs.push(("costs.battery_per_mwh", battery));
                                let (capex, npv, last_year) = island(&inputs);
                                let cost = &tree["nodes"][format!("choose_after_{timing}")]
                                    ["branches"][format!("build_{system}")];
                                (
                                    capex / 1.08 - cost["cost"].as_f64().unwrap(),
                                    (npv + capex - last_year) / 1.08,
                                )
                            };
                            // Each number in the file is rounded to the dollar
                            assert!(cost.abs() <= 0.5, "{system} {timing}: capex off by {cost}");
                            let written = tree["nodes"][format!("outcome_{timing}_{system}")]
                                ["branches"][format!("{outlook}_{weather}")]["value"]
                                .as_f64()
                                .unwrap();
                            assert!(
                                close(written, value, 0.5),
                                "{system} {timing} {outlook} {weather}: the tree says {written}, \
                                 the model {value}"
                            );
                            checked += 1;
                        }
                    }
                }
            }
            assert_eq!(checked, 36);
            // Waiting costs nothing in itself; its cost is the year it loses
            let wait = tree["root"]["branches"]["wait_a_year"]["cost"].as_f64();
            assert!(close(wait.unwrap(), 0.0, 0.0), "{wait:?}");
        }

        #[test]
        fn build_the_larger_system_now() {
            let result = decision_tree_core(Path::new(TREE)).unwrap();
            // The leaves are rounded to the dollar, the Python is not
            assert!(close(
                result.root_expected_value,
                13_792_484.177_027_194,
                1.0
            ));
            let root = &result.node_results["root"];
            assert_eq!(root.optimal_choice.as_deref(), Some("build_big_now"));
            assert!(close(
                root.branch_values["build_small_now"],
                12_826_402.581_436_146,
                1.0
            ));
            assert!(close(
                root.branch_values["wait_a_year"],
                13_697_669.009_062_1,
                1.0
            ));
            // A year later the larger system is the better one either way
            for node in ["choose_after_fall", "choose_after_flat"] {
                let choice = result.node_results[node].optimal_choice.as_deref();
                assert_eq!(choice, Some("build_big"), "{node}");
            }
        }
    }

    mod acquisition_example {
        //! `examples/acquisition/deal_bid.yaml`: open at 30% and risk Thornvale's
        //! counter, or knock it out at 40%.
        //!
        //! Its three deal values are deal.yaml calculated at each premium, so the
        //! first test calculates them again and fails when the model and the tree
        //! part. The expected values, from the tree's own numbers:
        //! ```python
        //! fee = 30 - 12                                     # break fee less costs
        //! after = max(0.35 * fee + 0.65 * 40.91, fee)       # raise to 45%: 32.8915
        //! offer_30 = 0.4 * 224.51 + 0.6 * after             # 109.5389
        //! knockout = 0.85 * 102.11 + 0.15 * after           # 91.727225
        //! # opening at 30% is better while Thornvale counters it less than
        //! # (224.51 - knockout) / (224.51 - after) = 0.6929538379644973 of the time
        //! ```

        use crate::cli::decision_tree_core;
        use crate::core::ArrayCalculator;
        use std::path::Path;

        const TREE: &str = "examples/acquisition/deal_bid.yaml";

        fn close(a: f64, b: f64, tol: f64) -> bool {
            (a - b).abs() <= tol
        }

        #[test]
        fn every_deal_value_in_the_tree_is_the_model_at_that_price() {
            let text = std::fs::read_to_string(TREE).unwrap();
            let doc: yaml_serde::Value = yaml_serde::from_str(&text).unwrap();
            let nodes = &doc["decision_tree"]["nodes"];
            for (premium, node, branch) in [
                (0.30, "thornvale_after_30", "stays_out"),
                (0.40, "thornvale_after_40", "stays_out"),
                (0.45, "thornvale_after_45", "gives_up"),
            ] {
                let mut model =
                    crate::parser::parse_model(Path::new("examples/acquisition/deal.yaml"))
                        .unwrap();
                model
                    .override_scalar("offer.premium", Some(premium), None)
                    .unwrap();
                let result = ArrayCalculator::new(model).calculate_all().unwrap();
                let value = result.scalars["valuation.value_created"].value.unwrap();
                let written = nodes[node]["branches"][branch]["value"].as_f64().unwrap();
                // Each number in the file is rounded to the cent
                assert!(
                    close(written, value, 0.005),
                    "{premium}: the tree says {written}, the model {value}"
                );
            }
        }

        #[test]
        fn open_at_30_percent_and_raise_if_thornvale_counters() {
            let result = decision_tree_core(Path::new(TREE)).unwrap();
            assert!(close(result.root_expected_value, 109.538_9, 1e-9));
            let root = &result.node_results["root"];
            assert_eq!(root.optimal_choice.as_deref(), Some("offer_30_percent"));
            assert!(close(
                root.branch_values["knockout_40_percent"],
                91.727_225,
                1e-9
            ));
            let after = &result.node_results["valdane_after_counter"];
            assert_eq!(after.optimal_choice.as_deref(), Some("raise_to_45_percent"));
            assert!(close(result.risk_profile.worst_case, 18.0, 1e-12));
        }
    }

    /// The root's name and how many nodes the tree lists are in the result
    /// (T1-CLI-FROM-CORES).
    #[test]
    fn the_result_names_the_root_and_counts_the_nodes() {
        let result = decision_tree_core(Path::new("examples/decision-tree.yaml")).unwrap();
        assert_eq!(result.root_name, "Invest in R&D?");
        assert_eq!(result.node_count, 4);
    }

    mod settlement {
        //! `examples/questions/settlement.yaml`: the offer is taken, and
        //! refusing is the better choice only above 85 chances in 130 of
        //! winning.
        //!
        //! Checked outside Forge:
        //! ```python
        //! def refuse(p):
        //!     trial = p * 120000 + (1 - p) * -10000 - 20000
        //!     better = max(50000, trial)        # the offer of 50,000, or the trial
        //!     none = max(trial, 0)              # the trial, or drop the claim
        //!     return 0.4 * better + 0.6 * none - 15000
        //! refuse(0.6)        # 33800.0, against the 40,000 offered
        //! refuse(85 / 130)   # 40000.0: the trial is then worth 55,000
        //! ```

        use super::super::decision_tree_core;
        use std::path::Path;

        const MODEL: &str = "examples/questions/settlement.yaml";

        /// The tree with the claim winning `win` times in 1,000.
        fn with_chance(dir: &tempfile::TempDir, win: u32) -> std::path::PathBuf {
            let text = std::fs::read_to_string(MODEL).unwrap();
            let text = text
                .replacen(
                    "probability: 0.6\n          value: 120000",
                    &format!("probability: 0.{win}\n          value: 120000"),
                    1,
                )
                .replacen(
                    "probability: 0.4\n          value: -10000",
                    &format!("probability: 0.{}\n          value: -10000", 1_000 - win),
                    1,
                );
            let path = dir.path().join(format!("{win}.yaml"));
            std::fs::write(&path, text).unwrap();
            path
        }

        #[test]
        fn the_offer_is_taken_though_the_trial_alone_is_worth_more() {
            let tree = decision_tree_core(Path::new(MODEL)).unwrap();
            let root = &tree.node_results["root"];
            assert_eq!(root.optimal_choice.as_deref(), Some("accept"));
            assert!((root.branch_values["accept"] - 40_000.0).abs() <= 1e-9);
            assert!((root.branch_values["refuse"] - 33_800.0).abs() <= 1e-9);
            let door = &tree.node_results["the_court_door"];
            assert_eq!(door.optimal_choice.as_deref(), Some("trial"));
            assert!((door.expected_value - 48_000.0).abs() <= 1e-9);
            let better = &tree.node_results["the_better_offer"];
            assert_eq!(better.optimal_choice.as_deref(), Some("accept"));
            assert!((tree.root_expected_value - 40_000.0).abs() <= 1e-9);
        }

        #[test]
        fn refusing_is_better_only_above_85_chances_in_130() {
            // 85 / 130 = 0.6538...: accept at 0.653, refuse at 0.654
            let dir = tempfile::tempdir().unwrap();
            let below = decision_tree_core(&with_chance(&dir, 653)).unwrap();
            let root = &below.node_results["root"];
            assert_eq!(root.optimal_choice.as_deref(), Some("accept"));
            assert!((root.branch_values["refuse"] - 39_890.0).abs() <= 1e-6);
            let above = decision_tree_core(&with_chance(&dir, 654)).unwrap();
            let root = &above.node_results["root"];
            assert_eq!(root.optimal_choice.as_deref(), Some("refuse"));
            assert!((root.branch_values["refuse"] - 40_020.0).abs() <= 1e-6);
            // Refused, the claim ends 45,000 down in 34.6 cases in 100
            let worst = above.risk_profile.worst_case;
            assert!((worst + 45_000.0).abs() <= 1e-6, "{worst}");
        }
    }
}
