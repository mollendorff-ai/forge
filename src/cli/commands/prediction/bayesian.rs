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

    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
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
    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
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
