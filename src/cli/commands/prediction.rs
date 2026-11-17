//! Prediction Methods CLI Commands
//!
//! CLI handlers for advanced forecasting methods:
//! - scenarios: Run scenario analysis (Base/Bull/Bear)
//! - `decision_tree`: Analyze decision trees with backward induction
//! - `real_options`: Value managerial flexibility (defer/expand/abandon)
//! - tornado: Generate sensitivity tornado diagrams
//! - bootstrap: Non-parametric confidence intervals via resampling
//! - bayesian: Bayesian network inference and queries

use crate::bayesian::{BayesianConfig, BayesianEngine};
use crate::bootstrap::{BootstrapConfig, BootstrapEngine};
use crate::cli::console::outln;
use crate::cli::Console;
use crate::core::precision::money;
use crate::core::precision::Rounded;
use crate::decision_trees::{DecisionTreeConfig, DecisionTreeEngine};
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use crate::real_options::{RealOptionsConfig, RealOptionsEngine};
use crate::scenarios::{ScenarioConfig, ScenarioEngine};
use crate::tornado::{TornadoConfig, TornadoEngine};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Serialises an analytics result for `--output`: JSON for a `.json` path,
/// YAML otherwise, with numbers to 15 significant digits (`core::precision`).
/// (This used to write a Rust debug dump, which no tool can read.)
fn serialize_for_file<T: serde::Serialize>(value: &T, path: &Path) -> ForgeResult<String> {
    if path.extension().and_then(|e| e.to_str()) == Some("json") {
        serde_json::to_string_pretty(&Rounded(value))
            .map_err(|e| ForgeError::Validation(format!("JSON error: {e}")))
    } else {
        yaml_serde::to_string(&Rounded(value))
            .map_err(|e| ForgeError::Validation(format!("YAML error: {e}")))
    }
}

/// Run scenario analysis and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no scenarios section exists,
/// or the scenario engine fails.
pub fn scenarios_core(
    file: &Path,
    scenario_filter: Option<&str>,
) -> ForgeResult<crate::scenarios::ScenarioResults> {
    let yaml_content = parser::read_model_text(file)?;
    let model = parser::parse_model(file)?;

    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let config: ScenarioConfig = if let Some(scenarios_value) = value.get("scenarios") {
        let scenarios_map: indexmap::IndexMap<String, yaml_serde::Value> =
            yaml_serde::from_value(scenarios_value.clone())
                .map_err(|e| ForgeError::Validation(format!("scenarios config error: {e}")))?;

        let mut config = ScenarioConfig::default();
        for (name, def) in scenarios_map {
            // `forge compare` needs no probability; this command weighs by it
            if def
                .as_mapping()
                .is_some_and(|m| !m.contains_key("probability"))
            {
                return Err(ForgeError::Validation(format!(
                    "scenario '{name}' has no probability. forge scenarios weighs the scenarios \
                     by it, and the probabilities of all scenarios must sum to 1 (forge compare \
                     does not need it)"
                )));
            }
            let scenario_def = yaml_serde::from_value(def)
                .map_err(|e| ForgeError::Validation(format!("scenario '{name}' error: {e}")))?;
            config.scenarios.insert(name, scenario_def);
        }
        config
    } else {
        return Err(ForgeError::Validation(
            "No 'scenarios' section found in YAML".to_string(),
        ));
    };

    if let Some(name) = scenario_filter {
        if !config.scenarios.contains_key(name) {
            let mut available: Vec<_> = config.scenarios.keys().cloned().collect();
            available.sort();
            return Err(ForgeError::Validation(format!(
                "Scenario '{name}' not found. Available scenarios: {available:?}"
            )));
        }
    }

    // Report every calculated scalar (the ones with a formula) as an output,
    // so the probability of a positive result and the range are filled in.
    let outputs: Vec<String> = model
        .scalars
        .iter()
        .filter(|(_, v)| v.formula.is_some())
        .map(|(name, _)| name.clone())
        .collect();
    let engine = ScenarioEngine::new(config, model)
        .map_err(ForgeError::Validation)?
        .with_outputs(outputs);
    let mut results = engine.run().map_err(ForgeError::Eval)?;
    // The expected values and ranges still cover every scenario.
    if let Some(name) = scenario_filter {
        results.scenarios.retain(|s| s.name == name);
    }
    Ok(results)
}

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

/// Value real options and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no `real_options` section exists,
/// or the analysis fails.
pub fn real_options_core(file: &Path) -> ForgeResult<crate::real_options::OptionsResult> {
    let yaml_content = parser::read_model_text(file)?;

    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let config: RealOptionsConfig = if let Some(ro_value) = value.get("real_options") {
        yaml_serde::from_value(ro_value.clone())
            .map_err(|e| ForgeError::Validation(format!("real_options config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'real_options' section found in YAML".to_string(),
        ));
    };

    let engine = RealOptionsEngine::new(config).map_err(ForgeError::Validation)?;
    engine.analyze().map_err(ForgeError::Eval)
}

/// Generate tornado sensitivity analysis and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no tornado section exists,
/// or the analysis fails.
pub fn tornado_core(
    file: &Path,
    output_var: Option<&str>,
) -> ForgeResult<crate::tornado::TornadoResult> {
    let yaml_content = parser::read_model_text(file)?;
    let model = parser::parse_model(file)?;

    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let mut config: TornadoConfig = if let Some(tornado_value) = value.get("tornado") {
        yaml_serde::from_value(tornado_value.clone())
            .map_err(|e| ForgeError::Validation(format!("tornado config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'tornado' section found in YAML".to_string(),
        ));
    };

    if let Some(out_var) = output_var {
        config.output = out_var.to_string();
    }

    let engine = TornadoEngine::new(config, model).map_err(ForgeError::Validation)?;
    engine.analyze().map_err(ForgeError::Eval)
}

/// Run bootstrap resampling and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no bootstrap section exists,
/// or the analysis fails.
pub fn bootstrap_core(
    file: &Path,
    iterations_override: Option<usize>,
    seed_override: Option<u64>,
    confidence_override: Option<Vec<f64>>,
) -> ForgeResult<crate::bootstrap::BootstrapResult> {
    let yaml_content = parser::read_model_text(file)?;

    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let mut config: BootstrapConfig = if let Some(bs_value) = value.get("bootstrap") {
        yaml_serde::from_value(bs_value.clone())
            .map_err(|e| ForgeError::Validation(format!("bootstrap config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'bootstrap' section found in YAML".to_string(),
        ));
    };

    if let Some(n) = iterations_override {
        config.iterations = n;
    }
    if let Some(s) = seed_override {
        config.seed = Some(s);
    }
    if let Some(levels) = confidence_override {
        config.confidence_levels = levels;
    }

    let mut engine = BootstrapEngine::new(config).map_err(ForgeError::Validation)?;
    engine.analyze().map_err(ForgeError::Eval)
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

/// Execute the scenarios command - probability-weighted scenario analysis
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no scenarios section exists,
/// or the scenario engine fails.
pub fn scenarios(
    c: &mut Console<'_>,
    file: &Path,
    scenario_filter: Option<&str>,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "📊 Forge - Scenario Analysis".bold().green());
    outln!(c, "   File: {}", file.display());
    outln!(c);

    if verbose {
        match scenario_filter {
            Some(filter) => outln!(c, "{}", format!("🎯 Running scenario: {filter}").cyan()),
            None => outln!(c, "{}", "🔄 Running all scenarios...".cyan()),
        }
    }
    let results = scenarios_core(file, scenario_filter)?;
    // A number with a `format:` is printed as the format shows it (ADR-061)
    let formats = super::scalar_formats(&parser::parse_model(file)?);
    let shown = |var: &str, value: f64| {
        formats
            .get(var)
            .and_then(|f| super::number_shown_by(value, f))
            .unwrap_or_else(|| super::display_number(value))
    };

    // Display results: what each scenario produced (the calculated scalars)
    outln!(c, "{}", "📈 Scenario Results:".bold().green());
    outln!(c);
    for result in &results.scenarios {
        outln!(
            c,
            "   {} (p={}%): {}",
            result.name.bright_blue().bold(),
            super::format_number(result.probability * 100.0),
            result.description
        );
        for (var, value) in &result.outputs {
            outln!(c, "      {}: {}", var.cyan(), shown(var, *value));
        }
        outln!(c);
    }

    outln!(c, "   {}", "Expected Values (probability-weighted):".bold());
    for (var, ev) in &results.expected_values {
        if !results.ranges.contains_key(var) {
            continue; // an input, not a result
        }
        let (low, high) = results.ranges[var];
        let positive = results
            .probability_positive
            .get(var)
            .copied()
            .unwrap_or(0.0);
        outln!(
            c,
            "      {}: {}   (range {} to {}, P(>0) = {}%)",
            var.bright_blue(),
            shown(var, *ev),
            shown(var, low),
            shown(var, high),
            super::format_number(positive * 100.0)
        );
    }
    if scenario_filter.is_some() {
        outln!(c, "      (expected values cover every scenario)");
    }
    outln!(c);

    // Write output if specified
    if let Some(output_path) = output_file {
        let output_str = serialize_for_file(&results, &output_path)?;
        fs::write(&output_path, output_str).map_err(ForgeError::Io)?;
        outln!(
            c,
            "{}",
            format!("💾 Results written to {}", output_path.display())
                .bold()
                .green()
        );
    }

    outln!(c, "{}", "✅ Scenario analysis complete".bold().green());
    Ok(())
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

/// Execute the real-options command - value managerial flexibility
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no `real_options` section exists,
/// the option filter does not match, or the analysis fails.
pub fn real_options(
    c: &mut Console<'_>,
    file: &Path,
    option_filter: Option<&str>,
    compare_npv: bool,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "💎 Forge - Real Options Analysis".bold().green());
    outln!(c, "   File: {}", file.display());
    outln!(c);

    // Parse YAML
    let yaml_content = parser::read_model_text(file)?;

    // Parse real_options config
    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let config: RealOptionsConfig = if let Some(ro_value) = value.get("real_options") {
        yaml_serde::from_value(ro_value.clone())
            .map_err(|e| ForgeError::Validation(format!("real_options config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'real_options' section found in YAML".to_string(),
        ));
    };

    // Display config
    outln!(c, "   {}", format!("Analysis: {}", config.name).bold());
    outln!(
        c,
        "      Method: {}",
        format!("{:?}", config.method).bright_blue()
    );
    outln!(
        c,
        "      Underlying value: {}",
        money(config.underlying.current_value)
    );
    outln!(
        c,
        "      Volatility: {:.1}%",
        config.underlying.volatility * 100.0
    );
    outln!(
        c,
        "      Risk-free rate: {:.1}%",
        config.underlying.risk_free_rate * 100.0
    );
    outln!(
        c,
        "      Time horizon: {} {}",
        config.underlying.time_horizon,
        if (config.underlying.time_horizon - 1.0).abs() < f64::EPSILON {
            "year"
        } else {
            "years"
        }
    );
    outln!(c);

    // Create engine and value options
    let engine = RealOptionsEngine::new(config).map_err(ForgeError::Validation)?;

    if verbose {
        outln!(c, "{}", "🔄 Valuing options...".cyan());
    }

    let result = engine.analyze().map_err(ForgeError::Eval)?;

    // Filter if specific option requested
    if let Some(filter) = option_filter {
        if !result.options.contains_key(filter) {
            return Err(ForgeError::Validation(format!(
                "Option '{}' not found. Available: {:?}",
                filter,
                result.options.keys().collect::<Vec<_>>()
            )));
        }
    }

    // Display results, options in the order the file lists them
    let order: Vec<String> = engine
        .config()
        .options
        .iter()
        .map(|o| o.name.clone())
        .collect();
    print_real_options_results(c, &result, &order, option_filter, compare_npv);

    // Write output if specified
    if let Some(output_path) = output_file {
        let output_str = result.to_yaml();
        fs::write(&output_path, output_str).map_err(ForgeError::Io)?;
        outln!(
            c,
            "{}",
            format!("💾 Results written to {}", output_path.display())
                .bold()
                .green()
        );
    }

    outln!(c, "{}", "✅ Real options analysis complete".bold().green());
    Ok(())
}

/// Execute the tornado command - sensitivity tornado diagram
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no tornado section exists,
/// or the analysis fails.
pub fn tornado(
    c: &mut Console<'_>,
    file: &Path,
    output_var: Option<&str>,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "🌪️ Forge - Tornado Diagram".bold().green());
    outln!(c, "   File: {}", file.display());
    outln!(c);

    // Parse YAML and model
    let yaml_content = parser::read_model_text(file)?;
    let model = parser::parse_model(file)?;

    // Parse tornado config
    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let mut config: TornadoConfig = if let Some(tornado_value) = value.get("tornado") {
        yaml_serde::from_value(tornado_value.clone())
            .map_err(|e| ForgeError::Validation(format!("tornado config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'tornado' section found in YAML".to_string(),
        ));
    };

    // Override output variable if specified
    if let Some(out_var) = output_var {
        config.output = out_var.to_string();
    }

    // Display config
    outln!(c, "   Output variable: {}", config.output.bright_blue());
    outln!(c, "   Inputs to vary: {}", config.inputs.len());
    if verbose {
        for input in &config.inputs {
            outln!(
                c,
                "      {} [{:.2} - {:.2}]",
                input.name.cyan(),
                input.low,
                input.high
            );
        }
    }
    outln!(c);

    // Create engine and analyze
    let engine = TornadoEngine::new(config, model).map_err(ForgeError::Validation)?;

    if verbose {
        outln!(c, "{}", "🔄 Calculating sensitivities...".cyan());
    }

    let result = engine.analyze().map_err(ForgeError::Eval)?;

    // Display results
    outln!(c, "{}", "📊 Tornado Diagram:".bold().green());
    outln!(
        c,
        "   Base value: {}",
        super::display_number(result.base_value).bold()
    );
    outln!(c);

    // Tornado bars (already sorted by impact)
    let max_impact = result
        .bars
        .iter()
        .map(|b| b.swing.abs())
        .fold(0.0f64, f64::max);

    let name_width = result
        .bars
        .iter()
        .map(|b| b.input_name.chars().count())
        .max()
        .unwrap_or(0);

    for bar in &result.bars {
        // Truncation/sign-loss impossible: value is always in 0.0..=30.0
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let bar_width = if max_impact > 0.0 {
            ((bar.swing.abs() / max_impact) * 30.0) as usize
        } else {
            0
        };
        let bar_str = "█".repeat(bar_width.max(1));

        // `swing` is the output at the high input minus the output at the low input.
        outln!(
            c,
            "   {:name_width$} |{:30}| swing {}  (output {} at low, {} at high)",
            bar.input_name.bright_blue(),
            bar_str.cyan(),
            super::display_number(bar.swing.abs()),
            super::display_number(bar.output_at_low),
            super::display_number(bar.output_at_high)
        );
        if let Some(note) = &bar.note {
            outln!(c, "   {:name_width$}  {}", "", note.yellow());
        }
    }
    outln!(c);

    // Write output if specified
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

    outln!(c, "{}", "✅ Tornado diagram complete".bold().green());
    Ok(())
}

/// Execute the bootstrap command - resampling confidence intervals
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no bootstrap section exists,
/// or the analysis fails.
pub fn bootstrap(
    c: &mut Console<'_>,
    file: &Path,
    iterations_override: Option<usize>,
    seed_override: Option<u64>,
    confidence_override: Option<Vec<f64>>,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "🔄 Forge - Bootstrap Resampling".bold().green());
    outln!(c, "   File: {}", file.display());
    outln!(c);

    // Parse YAML
    let yaml_content = parser::read_model_text(file)?;

    // Parse bootstrap config
    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let mut config: BootstrapConfig = if let Some(bs_value) = value.get("bootstrap") {
        yaml_serde::from_value(bs_value.clone())
            .map_err(|e| ForgeError::Validation(format!("bootstrap config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'bootstrap' section found in YAML".to_string(),
        ));
    };

    // Apply overrides
    if let Some(n) = iterations_override {
        config.iterations = n;
    }
    if let Some(s) = seed_override {
        config.seed = Some(s);
    }
    if let Some(levels) = confidence_override {
        config.confidence_levels = levels;
    }

    // Display config
    outln!(c, "   {}", "Configuration:".bold());
    outln!(
        c,
        "      Iterations: {}",
        config.iterations.to_string().bright_blue()
    );
    outln!(
        c,
        "      Statistic: {}",
        format!("{:?}", config.statistic).bright_blue()
    );
    outln!(c, "      Data points: {}", config.data.len());
    outln!(c, "      Confidence levels: {:?}", config.confidence_levels);
    outln!(c, "      Interval method: {:?}", config.method);
    if let Some(seed) = config.seed {
        outln!(c, "      Seed: {seed}");
    }
    outln!(c);

    // Create engine and analyze
    let mut engine = BootstrapEngine::new(config).map_err(ForgeError::Validation)?;

    if verbose {
        outln!(c, "{}", "🔄 Resampling...".cyan());
    }

    let result = engine.analyze().map_err(ForgeError::Eval)?;

    // Display results
    outln!(c, "{}", "📊 Bootstrap Results:".bold().green());
    outln!(c);

    outln!(
        c,
        "   Original statistic: {}",
        format!("{:.4}", result.original_estimate).bold()
    );
    outln!(c, "   Bootstrap mean: {:.4}", result.bootstrap_mean);
    outln!(
        c,
        "   Bootstrap std error: {:.4}",
        result.bootstrap_std_error
    );
    outln!(c, "   Bias: {:.4}", result.bias);
    outln!(c);

    // Confidence intervals
    outln!(c, "   {}", "Confidence Intervals:".bold());
    for ci in &result.confidence_intervals {
        outln!(
            c,
            "      {:.0}% CI: [{:.4}, {:.4}]",
            ci.level * 100.0,
            ci.lower,
            ci.upper
        );
    }
    outln!(c);

    // Write output if specified
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

    outln!(c, "{}", "✅ Bootstrap analysis complete".bold().green());
    Ok(())
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

/// Print real options analysis results
fn print_real_options_results(
    c: &mut Console<'_>,
    result: &crate::real_options::OptionsResult,
    order: &[String],
    option_filter: Option<&str>,
    compare_npv: bool,
) {
    outln!(c, "{}", "📊 Real Options Results:".bold().green());
    outln!(c);

    match result.traditional_npv {
        Some(npv) => outln!(
            c,
            "   Traditional NPV (invest now, no flexibility): {}",
            money(npv).yellow()
        ),
        None if compare_npv => outln!(
            c,
            "   Traditional NPV: {}",
            "not given (set traditional_npv in real_options)".yellow()
        ),
        None => {},
    }
    outln!(c);

    outln!(c, "   {}", "Options (each valued on its own):".bold());
    for name in order {
        if option_filter.is_some_and(|filter| filter != name) {
            continue;
        }
        let Some(opt_result) = result.options.get(name) else {
            continue;
        };
        outln!(
            c,
            "      {} ({})",
            opt_result.name.bright_blue(),
            format!("{:?}", opt_result.option_type).dimmed(),
        );
        match opt_result.value_added {
            Some(v) => outln!(
                c,
                "         Adds over the traditional NPV: {}",
                money(v).bold().green()
            ),
            None => outln!(
                c,
                "         Adds over the traditional NPV: needs traditional_npv"
            ),
        }
        match opt_result.project_value {
            Some(pv) => outln!(c, "         Project value with this option: {}", money(pv)),
            None => outln!(
                c,
                "         Project value with this option: needs traditional_npv"
            ),
        }
        if let Some(ref trigger) = opt_result.optimal_trigger {
            outln!(c, "         Rule of thumb: exercise when {trigger}");
        }
        if let Some(prob) = opt_result.probability_exercise {
            outln!(c, "         P(exercise): {:.1}%", prob * 100.0);
        }
    }
    outln!(c);

    if let (Some(best), Some(pv)) = (&result.best_option, result.project_value_with_options) {
        outln!(
            c,
            "   Project value with the best option ({}): {}",
            best.bright_blue(),
            money(pv).bold().green()
        );
    }
    if result.options.len() > 1 {
        outln!(
            c,
            "   {}",
            "Option values overlap, so they are compared, not added.".dimmed()
        );
    }

    outln!(c);
    outln!(
        c,
        "   {}: {}",
        "Decision".bold(),
        result.decision.bright_yellow()
    );
    outln!(
        c,
        "   {}: {}",
        "Recommendation".bold(),
        result.recommendation
    );
    outln!(c);
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
        // Truncation/sign-loss impossible: prob is in 0.0..=1.0, result in 0.0..=30.0
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let bar_width = (prob * 30.0) as usize;
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
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_scenarios_config_parsing() {
        let yaml = r#"
_forge_version: "9.0.0"

scenarios:
  base_case:
    probability: 0.50
    description: "Base case"
    scalars:
      revenue_growth: 0.05
  bull_case:
    probability: 0.50
    scalars:
      revenue_growth: 0.15

scalars:
  revenue_growth:
    value: 0.05
"#;
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{yaml}").unwrap();

        // Test config parsing
        let value: yaml_serde::Value = yaml_serde::from_str(yaml).unwrap();
        assert!(value.get("scenarios").is_some());
    }

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
    fn test_bootstrap_config_parsing() {
        let yaml = r#"
_forge_version: "9.0.0"

bootstrap:
  iterations: 1000
  confidence_levels: [0.90, 0.95]
  data: [1.0, 2.0, 3.0, 4.0, 5.0]
  statistic: mean
"#;
        let value: yaml_serde::Value = yaml_serde::from_str(yaml).unwrap();
        let bs_value = value.get("bootstrap").unwrap();
        let config: BootstrapConfig = yaml_serde::from_value(bs_value.clone()).unwrap();
        assert_eq!(config.iterations, 1000);
        assert_eq!(config.data.len(), 5);
    }

    /// `--output` must write a file other tools can read (YAML, or JSON for
    /// a `.json` path), not a Rust debug dump.
    #[test]
    fn test_output_files_are_machine_readable() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("m.yaml");
        std::fs::write(
            &model,
            "_forge_version: \"5.0.0\"\n\nbootstrap:\n  iterations: 200\n  seed: 1\n  confidence_levels: [0.95]\n  data: [1.0, 2.0, 3.0, 4.0, 5.0]\n  statistic: mean\n",
        )
        .unwrap();

        let yaml_out = dir.path().join("out.yaml");
        capture(|c| bootstrap(c, &model, None, None, None, Some(yaml_out.clone()), false))
            .0
            .unwrap();
        let doc: yaml_serde::Value =
            yaml_serde::from_str(&std::fs::read_to_string(&yaml_out).unwrap()).unwrap();
        assert_eq!(doc["original_estimate"].as_f64(), Some(3.0));
        assert!(doc["confidence_intervals"].is_sequence());

        let json_out = dir.path().join("out.json");
        capture(|c| bootstrap(c, &model, None, None, None, Some(json_out.clone()), false))
            .0
            .unwrap();
        let doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&json_out).unwrap()).unwrap();
        assert_eq!(doc["original_estimate"].as_f64(), Some(3.0));
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
}
