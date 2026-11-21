//! `forge bayesian`: the probabilities of a Bayesian network, with evidence.

use super::write_result;
use crate::bayesian::{BayesianConfig, BayesianEngine};
use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use colored::Colorize;
use num_traits::ToPrimitive;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The network of a model file.
fn read_network(file: &Path) -> ForgeResult<BayesianConfig> {
    let yaml_content = parser::read_model_text(file)?;
    let value: yaml_serde::Value = parser::read_yaml(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;
    let bn_value = value.get("bayesian_network").ok_or_else(|| {
        ForgeError::Validation("No 'bayesian_network' section found in YAML".to_string())
    })?;
    yaml_serde::from_value(bn_value.clone())
        .map_err(|e| ForgeError::Validation(format!("bayesian_network config error: {e}")))
}

/// The `name=state` pairs; an entry without exactly one `=` is left out.
fn parse_evidence(evidence: &[String]) -> HashMap<String, &str> {
    evidence
        .iter()
        .filter_map(|ev| match ev.split('=').collect::<Vec<_>>()[..] {
            [name, state] => Some((name.to_string(), state)),
            _ => None,
        })
        .collect()
}

/// The probabilities of `query_var`, or of every node, given the evidence.
fn infer(
    config: BayesianConfig,
    query_var: Option<&str>,
    evidence_map: &HashMap<String, &str>,
) -> ForgeResult<crate::bayesian::BayesianResult> {
    let network_name = config.name.clone();
    let engine = BayesianEngine::new(config).map_err(ForgeError::Validation)?;
    let Some(target) = query_var else {
        return if evidence_map.is_empty() {
            engine.query_all()
        } else {
            engine.query_all_with_evidence(evidence_map)
        }
        .map_err(ForgeError::Eval);
    };
    let var_result = if evidence_map.is_empty() {
        engine.query(target)
    } else {
        engine.query_with_evidence(target, evidence_map)
    }
    .map_err(ForgeError::Eval)?;
    Ok(crate::bayesian::BayesianResult {
        name: network_name,
        queries: std::iter::once((target.to_string(), var_result)).collect(),
        evidence: evidence_map
            .iter()
            .map(|(k, v)| (k.clone(), (*v).to_string()))
            .collect(),
    })
}

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
    infer(read_network(file)?, query_var, &parse_evidence(evidence))
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

    let config = read_network(file)?;

    print_bayesian_network(c, &config, verbose);

    let evidence_map = parse_evidence(evidence);

    if !evidence_map.is_empty() {
        outln!(c, "   {}", "Evidence:".bold());
        // In name order: the map's own order changes from run to run
        let mut shown: Vec<_> = evidence_map.iter().collect();
        shown.sort_unstable();
        for (var, val) in shown {
            outln!(c, "      {} = {}", var.bright_blue(), val.cyan());
        }
        outln!(c);
    }

    if verbose {
        outln!(c, "{}", "🔄 Running inference...".cyan());
    }

    let result = infer(config, query_var, &evidence_map)?;
    if let Some(target) = query_var {
        let heading = if evidence_map.is_empty() {
            "📊 Query Result:"
        } else {
            "📊 Query Result (with evidence):"
        };
        outln!(c, "{}", heading.bold().green());
        let var_result = &result.queries[target];
        print_bayesian_var_result(c, target, var_result);
        if let Some(output_path) = output_file {
            write_result(c, var_result, &output_path)?;
        }
    } else {
        outln!(c, "{}", "📊 All Node Probabilities:".bold().green());
        outln!(c);
        for (name, var_result) in &result.queries {
            print_bayesian_var_result(c, name, var_result);
        }
        if let Some(output_path) = output_file {
            write_result(c, &result, &output_path)?;
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
    use crate::cli::Console;

    const SLOW_WEEK: &str = "examples/bagel/slow_week.yaml";

    /// What `forge bayesian` prints (the console is not a terminal: no colours) and how it ends
    fn run(
        file: &Path,
        target: Option<&str>,
        evidence: &[&str],
        output: Option<PathBuf>,
        verbose: bool,
    ) -> (String, Result<(), String>) {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut input = std::io::empty();
        let mut console = Console::new(&mut out, &mut err, &mut input);
        let evidence: Vec<String> = evidence.iter().map(ToString::to_string).collect();
        let result = bayesian(&mut console, file, target, &evidence, output, verbose);
        let text = String::from_utf8(out).unwrap();
        (text, result.map_err(|e| e.to_string()))
    }

    const HEAD: &str = "🔮 Forge - Bayesian Network Inference\n   File: examples/bagel/slow_week.yaml\n\n   Network: Why was the week slow?\n   Nodes: 3\n";

    /// Worked by hand: P(roadwork | rainy, slow) = 0.2 * 0.9 / (0.2 * 0.9 + 0.8 * 0.6) = 3/11
    #[test]
    fn a_query_with_evidence_prints_the_node_its_evidence_and_its_steps() {
        let (text, result) = run(
            Path::new(SLOW_WEEK),
            Some("roadwork"),
            &["sales=slow", "weather=rainy"],
            None,
            true,
        );
        assert_eq!(
            (text.as_str(), result),
            (
                format!(
                    "{HEAD}      roadwork (2 states, parents: root)\n      sales (2 states, parents: weather, roadwork)\n      weather (2 states, parents: root)\n\n   Evidence:\n      sales = slow\n      weather = rainy\n\n🔄 Running inference...\n📊 Query Result (with evidence):\n   roadwork:\n      yes             |████████| 27.27%\n      no              |█████████████████████| 72.73%\n\n   Most likely: no (72.7%)\n\n✅ Bayesian inference complete\n"
                )
                .as_str(),
                Ok(())
            )
        );
    }

    /// With only sales = slow: P(slow) = 0.38 (0.054 + 0.144 + 0.098 + 0.084),
    /// P(roadwork) = 0.152 / 0.38 = 0.4, P(rainy) = 0.198 / 0.38 = 0.5211
    #[test]
    fn without_a_node_every_node_is_printed_and_an_entry_without_one_equals_is_left_out() {
        let (text, result) = run(
            Path::new(SLOW_WEEK),
            None,
            &["sales=slow", "bad", "a=b=c"],
            None,
            false,
        );
        assert_eq!(
            (text.as_str(), result),
            (
                format!(
                    "{HEAD}\n   Evidence:\n      sales = slow\n\n📊 All Node Probabilities:\n\n   roadwork:\n      yes             |████████████| 40.00%\n      no              |██████████████████| 60.00%\n\n   Most likely: no (60.0%)\n\n   weather:\n      rainy           |███████████████| 52.11%\n      dry             |██████████████| 47.89%\n\n   Most likely: rainy (52.1%)\n\n✅ Bayesian inference complete\n"
                )
                .as_str(),
                Ok(())
            )
        );
    }

    #[test]
    fn a_query_without_evidence_says_so_in_its_heading() {
        let (text, result) = run(Path::new(SLOW_WEEK), Some("weather"), &[], None, false);
        assert_eq!(
            (text.as_str(), result),
            (
                format!(
                    "{HEAD}\n📊 Query Result:\n   weather:\n      rainy           |█████████| 30.00%\n      dry             |█████████████████████| 70.00%\n\n   Most likely: dry (70.0%)\n\n✅ Bayesian inference complete\n"
                )
                .as_str(),
                Ok(())
            )
        );
    }

    #[test]
    fn what_cannot_be_asked_is_refused_by_name() {
        for (target, evidence, want) in [
            (Some("nope"), vec![], "Variable 'nope' not found in network"),
            (
                Some("roadwork"),
                vec!["sales=zzz"],
                "State 'zzz' not found for variable 'sales'",
            ),
            (None, vec!["zzz=slow"], "Evidence variable 'zzz' not found"),
        ] {
            let (_, result) = run(Path::new(SLOW_WEEK), target, &evidence, None, false);
            assert!(
                result.as_ref().is_err_and(|e| e.ends_with(want)),
                "{target:?} {evidence:?}: {result:?}"
            );
        }
    }

    /// `-o` writes the node asked for, or every node, as JSON or YAML by the
    /// path's extension, and says where
    #[test]
    fn the_result_is_written_where_asked() {
        let dir = tempfile::tempdir().unwrap();
        let one = dir.path().join("one.json");
        let all = dir.path().join("all.yaml");
        let (text, result) = run(
            Path::new(SLOW_WEEK),
            Some("weather"),
            &[],
            Some(one.clone()),
            false,
        );
        assert!(
            result.is_ok() && text.contains(&format!("💾 Results written to {}\n", one.display())),
            "{text}"
        );
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&one).unwrap()).unwrap();
        assert_eq!(
            (&json["states"], &json["probabilities"]),
            (
                &serde_json::json!(["rainy", "dry"]),
                &serde_json::json!([0.3, 0.7])
            )
        );
        let (text, result) = run(Path::new(SLOW_WEEK), None, &[], Some(all.clone()), false);
        assert!(
            result.is_ok() && text.contains(&format!("💾 Results written to {}\n", all.display())),
            "{text}"
        );
        let yaml = std::fs::read_to_string(&all).unwrap();
        assert!(
            yaml.starts_with("name: Why was the week slow?\nqueries:\n"),
            "{yaml}"
        );
    }

    #[test]
    fn a_result_that_cannot_be_written_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        for target in [Some("weather"), None] {
            let (_, result) = run(
                Path::new(SLOW_WEEK),
                target,
                &[],
                Some(dir.path().join("no-such-folder").join("r.json")),
                false,
            );
            assert!(result.is_err(), "{target:?}: {result:?}");
        }
    }

    /// Each way a model file is not a network, with the message
    #[test]
    fn a_file_that_is_not_a_network_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        for (name, text, want) in [
            (
                "plain.yaml",
                "_forge_version: \"5.1.0\"\nscalars: {}\n",
                "Validation error: No 'bayesian_network' section found in YAML".to_string(),
            ),
            (
                "bad.yaml",
                "a: [1\n",
                "Validation error: YAML parse error: ".to_string(),
            ),
            (
                "config.yaml",
                "bayesian_network: 3\n",
                "Validation error: bayesian_network config error: ".to_string(),
            ),
            (
                "empty.yaml",
                "bayesian_network:\n  name: x\n  nodes:\n    a:\n      type: discrete\n      states: [\"y\", \"n\"]\n      prior: [0.5]\n",
                "Validation error: ".to_string(),
            ),
        ] {
            let path = dir.path().join(name);
            std::fs::write(&path, text).unwrap();
            let (_, result) = run(&path, None, &[], None, false);
            let err = result.unwrap_err();
            assert!(err.starts_with(&want), "{name}: {err}");
            let core = bayesian_core(&path, None, &[]).unwrap_err().to_string();
            assert_eq!(core, err, "{name}: bayesian_core says the same");
        }
        let missing = dir.path().join("missing.yaml");
        assert!(run(&missing, None, &[], None, false).1.is_err());
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
