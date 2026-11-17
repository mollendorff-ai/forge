//! `forge decision-tree`: the best choice at each decision, by backward
//! induction.

use super::serialize_for_file;
use crate::cli::console::outln;
use crate::cli::Console;
use crate::core::precision::money;
use crate::decision_trees::{DecisionTreeConfig, DecisionTreeEngine};
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

/// Analyze a decision tree and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no `decision_tree` section exists,
/// or the analysis fails.
pub fn decision_tree_core(file: &Path) -> ForgeResult<crate::decision_trees::TreeResult> {
    let yaml_content = parser::read_model_text(file)?;

    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
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

    // Parse YAML
    let yaml_content = parser::read_model_text(file)?;

    // Parse decision_tree config
    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let config: DecisionTreeConfig = if let Some(dt_value) = value.get("decision_tree") {
        yaml_serde::from_value(dt_value.clone())
            .map_err(|e| ForgeError::Validation(format!("decision_tree config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'decision_tree' section found in YAML".to_string(),
        ));
    };

    // Display config
    outln!(c, "   {}", format!("Tree: {}", config.name).bold());
    if verbose {
        outln!(c, "      Root: {:?}", config.root.as_ref().map(|r| &r.name));
        outln!(c, "      Nodes: {}", config.nodes.len());
    }
    outln!(c);

    // Create engine and analyze
    let engine = DecisionTreeEngine::new(config).map_err(ForgeError::Validation)?;

    if verbose {
        outln!(c, "{}", "🔄 Running backward induction...".cyan());
    }

    let result = engine.analyze().map_err(ForgeError::Eval)?;

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
}
