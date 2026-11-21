//! `forge bayesian`: the probabilities of a Bayesian network, with evidence.

use super::serialize_for_file;
use crate::bayesian::{BayesianConfig, BayesianEngine};
use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use colored::Colorize;
use num_traits::ToPrimitive;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Run Bayesian network inference and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no `bayesian_network` section exists,
/// or inference fails.
pub fn bayesian_core(
    file: &Path,
    query_var: Option<&str>,
    evidence: &[String],
) -> ForgeResult<crate::bayesian::BayesianResult> {
    let yaml_content = parser::read_model_text(file)?;

    let value: yaml_serde::Value = parser::read_yaml(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let config: BayesianConfig = if let Some(bn_value) = value.get("bayesian_network") {
        yaml_serde::from_value(bn_value.clone())
            .map_err(|e| ForgeError::Validation(format!("bayesian_network config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'bayesian_network' section found in YAML".to_string(),
        ));
    };

    let mut evidence_map: HashMap<String, &str> = HashMap::new();
    for ev in evidence {
        let parts: Vec<&str> = ev.split('=').collect();
        if parts.len() == 2 {
            evidence_map.insert(parts[0].to_string(), parts[1]);
        }
    }

    let network_name = config.name.clone();
    let engine = BayesianEngine::new(config).map_err(ForgeError::Validation)?;

    if let Some(target) = query_var {
        let var_result = if evidence_map.is_empty() {
            engine.query(target).map_err(ForgeError::Eval)?
        } else {
            engine
                .query_with_evidence(target, &evidence_map)
                .map_err(ForgeError::Eval)?
        };
        Ok(crate::bayesian::BayesianResult {
            name: network_name,
            queries: std::iter::once((target.to_string(), var_result)).collect(),
            evidence: evidence_map
                .iter()
                .map(|(k, v)| (k.clone(), (*v).to_string()))
                .collect(),
        })
    } else {
        let all_results = if evidence_map.is_empty() {
            engine.query_all().map_err(ForgeError::Eval)?
        } else {
            engine
                .query_all_with_evidence(&evidence_map)
                .map_err(ForgeError::Eval)?
        };
        Ok(all_results)
    }
}

/// The network's name and size, and with `--verbose` each node's states and
/// parents.
fn print_bayesian_network(c: &mut Console<'_>, config: &BayesianConfig, verbose: bool) {
    outln!(c, "   {}", format!("Network: {}", config.name).bold());
    outln!(c, "   Nodes: {}", config.nodes.len());
    if verbose {
        for (name, node) in &config.nodes {
            let parents = if node.parents.is_empty() {
                "root".to_string()
            } else {
                node.parents.join(", ")
            };
            outln!(
                c,
                "      {} ({} states, parents: {})",
                name.bright_blue(),
                node.states.len(),
                parents
            );
        }
    }
    outln!(c);
}

/// Execute the bayesian command - Bayesian network inference
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no `bayesian_network` section exists,
/// or inference fails.
pub fn bayesian(
    c: &mut Console<'_>,
    file: &Path,
    query_var: Option<&str>,
    evidence: &[String],
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    outln!(
        c,
        "{}",
        "🔮 Forge - Bayesian Network Inference".bold().green()
    );
    outln!(c, "   File: {}", file.display());
    outln!(c);

    // Parse YAML
    let yaml_content = parser::read_model_text(file)?;

    // Parse bayesian_network config
    let value: yaml_serde::Value = parser::read_yaml(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let config: BayesianConfig = if let Some(bn_value) = value.get("bayesian_network") {
        yaml_serde::from_value(bn_value.clone())
            .map_err(|e| ForgeError::Validation(format!("bayesian_network config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'bayesian_network' section found in YAML".to_string(),
        ));
    };

    print_bayesian_network(c, &config, verbose);

    // Parse evidence
    let mut evidence_map: HashMap<String, &str> = HashMap::new();
    for ev in evidence {
        let parts: Vec<&str> = ev.split('=').collect();
        if parts.len() == 2 {
            evidence_map.insert(parts[0].to_string(), parts[1]);
        }
    }

    if !evidence_map.is_empty() {
        outln!(c, "   {}", "Evidence:".bold());
        for (var, val) in &evidence_map {
            outln!(c, "      {} = {}", var.bright_blue(), val.cyan());
        }
        outln!(c);
    }

    // Create engine
    let engine = BayesianEngine::new(config).map_err(ForgeError::Validation)?;

    if verbose {
        outln!(c, "{}", "🔄 Running inference...".cyan());
    }

    // Run query
    if let Some(target) = query_var {
        let var_result = if evidence_map.is_empty() {
            outln!(c, "{}", "📊 Query Result:".bold().green());
            engine.query(target).map_err(ForgeError::Eval)?
        } else {
            outln!(c, "{}", "📊 Query Result (with evidence):".bold().green());
            engine
                .query_with_evidence(target, &evidence_map)
                .map_err(ForgeError::Eval)?
        };
        print_bayesian_var_result(c, target, &var_result);
        if let Some(output_path) = output_file {
            let output_str = serialize_for_file(&var_result, &output_path)?;
            fs::write(&output_path, output_str).map_err(ForgeError::Io)?;
            outln!(
                c,
                "{}",
                format!("💾 Results written to {}", output_path.display())
                    .bold()
                    .green()
            );
        }
    } else {
        // Query all nodes
        let all_results = if evidence_map.is_empty() {
            engine.query_all().map_err(ForgeError::Eval)?
        } else {
            engine
                .query_all_with_evidence(&evidence_map)
                .map_err(ForgeError::Eval)?
        };

        outln!(c, "{}", "📊 All Node Probabilities:".bold().green());
        outln!(c);

        for (name, var_result) in &all_results.queries {
            print_bayesian_var_result(c, name, var_result);
        }

        if let Some(output_path) = output_file {
            let output_str = serialize_for_file(&all_results, &output_path)?;
            fs::write(&output_path, output_str).map_err(ForgeError::Io)?;
            outln!(
                c,
                "{}",
                format!("💾 Results written to {}", output_path.display())
                    .bold()
                    .green()
            );
        }
    }

    outln!(c, "{}", "✅ Bayesian inference complete".bold().green());
    Ok(())
}

/// Print Bayesian variable result with probability bars
fn print_bayesian_var_result(
    c: &mut Console<'_>,
    name: &str,
    var_result: &crate::bayesian::VariableResult,
) {
    outln!(c, "   {}:", name.bright_blue().bold());
    for (state, prob) in var_result
        .states
        .iter()
        .zip(var_result.probabilities.iter())
    {
        // A probability is 0 to 1, so 0 to 30 characters; never under 1
        let bar_width = (prob * 30.0).to_usize().unwrap_or(0);
        let bar = "█".repeat(bar_width.max(1));
        outln!(
            c,
            "      {:15} |{}| {:.2}%",
            state,
            bar.cyan(),
            prob * 100.0
        );
    }
    outln!(c);
    outln!(
        c,
        "   Most likely: {} ({:.1}%)",
        var_result.most_likely.bold().green(),
        var_result.max_probability * 100.0
    );
    outln!(c);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::console::capture;

    const DILIGENCE: &str = "examples/acquisition/diligence.yaml";

    fn model(text: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.yaml");
        fs::write(&path, text).unwrap();
        (dir, path)
    }

    fn evidence(items: &[&str]) -> Vec<String> {
        items.iter().map(ToString::to_string).collect()
    }

    fn message(result: ForgeResult<impl std::fmt::Debug>) -> String {
        result.unwrap_err().to_string()
    }

    #[test]
    fn core_refuses_a_model_that_is_not_a_network() {
        let (_dir, none) = model("a: 1\n");
        assert!(message(bayesian_core(&none, None, &[])).contains("No 'bayesian_network' section"));
        let (_dir, bad_yaml) = model("a: [\n");
        assert!(message(bayesian_core(&bad_yaml, None, &[])).contains("YAML parse error"));
        let (_dir, bad_config) = model("bayesian_network: 5\n");
        assert!(message(bayesian_core(&bad_config, None, &[]))
            .contains("bayesian_network config error"));
        let (_dir, bad_network) = model(
            "bayesian_network:\n  name: n\n  nodes:\n    a:\n      type: discrete\n      states: [x, y]\n      parents: [ghost]\n",
        );
        assert!(bayesian_core(&bad_network, None, &[]).is_err());
        assert!(bayesian_core(Path::new("no/such/model.yaml"), None, &[]).is_err());
    }

    #[test]
    fn core_answers_one_node_or_all_of_them() {
        let path = Path::new(DILIGENCE);
        let one = bayesian_core(path, Some("receivables"), &[]).unwrap();
        assert_eq!(one.queries.len(), 1);
        assert!(one.evidence.is_empty());
        let jumped = &one.queries["receivables"];
        // 0.08 * 0.80 + 0.92 * 0.15
        assert!((jumped.probabilities[0] - 0.202).abs() < 1e-12);

        let with = bayesian_core(path, Some("stuffing"), &evidence(&["q4_spike=yes"])).unwrap();
        assert_eq!(with.evidence["q4_spike"], "yes");

        let all = bayesian_core(path, None, &[]).unwrap();
        assert_eq!(all.queries.len(), 4);
        let all = bayesian_core(path, None, &evidence(&["q4_spike=yes", "junk", "a=b=c"])).unwrap();
        // The observed node is not asked about again
        assert_eq!(all.queries.len(), 3);
        assert!(!all.queries.contains_key("q4_spike"));
        assert_eq!(all.evidence.len(), 1);

        assert!(bayesian_core(path, Some("ghost"), &[]).is_err());
    }

    #[test]
    fn all_nodes_are_printed_with_bars_and_the_most_likely_state() {
        let (result, out) = capture(|c| bayesian(c, Path::new(DILIGENCE), None, &[], None, false));
        result.unwrap();
        for expected in [
            "Network: Quillon's last quarter",
            "Nodes: 4",
            "All Node Probabilities:",
            "stuffing:",
            "Most likely: no (92.0%)",
            "92.00%",
            "Bayesian inference complete",
        ] {
            assert!(out.out.contains(expected), "{expected}\n{}", out.out);
        }
        assert!(!out.out.contains("Running inference"), "{}", out.out);
    }

    #[test]
    fn a_query_with_evidence_prints_it_and_the_node_list_when_verbose() {
        let (result, out) = capture(|c| {
            bayesian(
                c,
                Path::new(DILIGENCE),
                Some("stuffing"),
                &evidence(&["q4_spike=yes", "junk", "a=b=c"]),
                None,
                true,
            )
        });
        result.unwrap();
        for expected in [
            "Evidence:",
            "q4_spike = yes",
            "Running inference...",
            "Query Result (with evidence):",
            "q4_spike (2 states, parents: stuffing, grid_contract)",
            "stuffing (2 states, parents: root)",
            "Most likely: no (76.7%)",
        ] {
            assert!(out.out.contains(expected), "{expected}\n{}", out.out);
        }
    }

    #[test]
    fn a_query_without_evidence_says_so() {
        let (result, out) =
            capture(|c| bayesian(c, Path::new(DILIGENCE), Some("stuffing"), &[], None, false));
        result.unwrap();
        assert!(out.out.contains("Query Result:"), "{}", out.out);
        assert!(!out.out.contains("Evidence:"), "{}", out.out);
    }

    #[test]
    fn all_nodes_with_evidence_are_answered_given_it() {
        let (result, out) = capture(|c| {
            bayesian(
                c,
                Path::new(DILIGENCE),
                None,
                &evidence(&["receivables=jumped"]),
                None,
                false,
            )
        });
        result.unwrap();
        assert!(out.out.contains("receivables = jumped"), "{}", out.out);
        assert!(out.out.contains("All Node Probabilities:"), "{}", out.out);
    }

    #[test]
    fn results_are_written_as_yaml_or_json_by_the_extension() {
        let dir = tempfile::tempdir().unwrap();
        for (name, query) in [
            ("one.yaml", Some("stuffing")),
            ("one.json", Some("stuffing")),
            ("all.yaml", None),
            ("all.json", None),
        ] {
            let out = dir.path().join(name);
            let (result, printed) = capture(|c| {
                bayesian(
                    c,
                    Path::new(DILIGENCE),
                    query,
                    &[],
                    Some(out.clone()),
                    false,
                )
            });
            result.unwrap();
            assert!(
                printed.out.contains("Results written to"),
                "{}",
                printed.out
            );
            let text = fs::read_to_string(&out).unwrap();
            let doc: yaml_serde::Value = yaml_serde::from_str(&text).unwrap();
            if query.is_some() {
                assert_eq!(doc["most_likely"].as_str(), Some("no"), "{name}");
            } else {
                assert!(doc["queries"]["stuffing"].is_mapping(), "{name}");
            }
        }
    }

    #[test]
    fn an_unwritable_output_is_an_io_error() {
        for query in [Some("stuffing"), None] {
            let out = PathBuf::from("no/such/dir/out.yaml");
            let (result, _) = capture(|c| {
                bayesian(
                    c,
                    Path::new(DILIGENCE),
                    query,
                    &[],
                    Some(out.clone()),
                    false,
                )
            });
            assert!(matches!(result, Err(ForgeError::Io(_))), "{query:?}");
        }
    }

    #[test]
    fn the_command_refuses_what_the_core_refuses() {
        let (_dir, none) = model("a: 1\n");
        let (_dir2, bad_yaml) = model("a: [\n");
        let (_dir3, bad_config) = model("bayesian_network: 5\n");
        let (_dir4, bad_network) = model(
            "bayesian_network:\n  name: n\n  nodes:\n    a:\n      type: discrete\n      states: [x, y]\n      parents: [ghost]\n",
        );
        for (path, expected) in [
            (&none, "No 'bayesian_network' section"),
            (&bad_yaml, "YAML parse error"),
            (&bad_config, "bayesian_network config error"),
        ] {
            let (result, _) = capture(|c| bayesian(c, path, None, &[], None, false));
            assert!(message(result).contains(expected), "{expected}");
        }
        let (result, _) = capture(|c| bayesian(c, &bad_network, None, &[], None, false));
        assert!(matches!(result, Err(ForgeError::Validation(_))));
        let (result, _) =
            capture(|c| bayesian(c, Path::new(DILIGENCE), Some("ghost"), &[], None, false));
        assert!(matches!(result, Err(ForgeError::Eval(_))));
        let (result, _) =
            capture(|c| bayesian(c, Path::new("no/such/model.yaml"), None, &[], None, false));
        assert!(result.is_err());
    }

    #[test]
    fn a_zero_probability_still_gets_a_one_character_bar() {
        let result = crate::bayesian::VariableResult {
            name: "x".to_string(),
            states: vec!["a".to_string(), "b".to_string()],
            probabilities: vec![0.0, 1.0],
            most_likely: "b".to_string(),
            max_probability: 1.0,
        };
        let ((), out) = capture(|c| print_bayesian_var_result(c, "x", &result));
        assert!(out.out.contains("a               |█| 0.00%"), "{}", out.out);
        assert!(
            out.out.contains(&format!("|{}| 100.00%", "█".repeat(30))),
            "{}",
            out.out
        );
    }

    mod acquisition_example {
        //! examples/acquisition/diligence.yaml: was Quillon's last quarter stuffed?
        //! Enumerating the joint distribution of stuffing x grid contract x spike x
        //! receivables: P(stuffing) = 0.08; P(stuffing | spike) = 0.23333333333333336;
        //! with receivables jumped too, 0.6187845303867404; and if the grid
        //! contract is real, 0.386277001270648 (the contract explains the spike
        //! away, but not the receivables).

        use crate::cli::bayesian_core;
        use std::path::Path;

        fn stuffing(evidence: &[&str]) -> f64 {
            let ev: Vec<String> = evidence.iter().map(ToString::to_string).collect();
            let file = Path::new("examples/acquisition/diligence.yaml");
            let r = bayesian_core(file, Some("stuffing"), &ev).unwrap();
            let q = &r.queries["stuffing"];
            q.probabilities[q.states.iter().position(|s| s == "yes").unwrap()]
        }

        #[test]
        fn the_receivables_make_stuffing_likelier_than_not() {
            for (evidence, want) in [
                (&[][..], 0.08),
                (&["q4_spike=yes"][..], 0.233_333_333_333_333_36),
                (
                    &["q4_spike=yes", "receivables=jumped"][..],
                    0.618_784_530_386_740_4,
                ),
                (
                    &["q4_spike=yes", "receivables=jumped", "grid_contract=yes"][..],
                    0.386_277_001_270_648,
                ),
            ] {
                let got = stuffing(evidence);
                assert!((got - want).abs() <= 1e-12, "{evidence:?}: {got}");
            }
        }
    }
}
