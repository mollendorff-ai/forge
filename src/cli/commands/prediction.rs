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
/// YAML otherwise. (This used to write a Rust debug dump, which no tool can read.)
fn serialize_for_file<T: serde::Serialize>(value: &T, path: &Path) -> ForgeResult<String> {
    if path.extension().and_then(|e| e.to_str()) == Some("json") {
        serde_json::to_string_pretty(value)
            .map_err(|e| ForgeError::Validation(format!("JSON error: {e}")))
    } else {
        yaml_serde::to_string(value).map_err(|e| ForgeError::Validation(format!("YAML error: {e}")))
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
    file: &Path,
    scenario_filter: Option<&str>,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "📊 Forge - Scenario Analysis".bold().green());
    println!("   File: {}", file.display());
    println!();

    if verbose {
        match scenario_filter {
            Some(filter) => println!("{}", format!("🎯 Running scenario: {filter}").cyan()),
            None => println!("{}", "🔄 Running all scenarios...".cyan()),
        }
    }
    let results = scenarios_core(file, scenario_filter)?;

    // Display results: what each scenario produced (the calculated scalars)
    println!("{}", "📈 Scenario Results:".bold().green());
    println!();
    for result in &results.scenarios {
        println!(
            "   {} (p={}%): {}",
            result.name.bright_blue().bold(),
            super::format_number(result.probability * 100.0),
            result.description
        );
        for (var, value) in &result.outputs {
            println!("      {}: {}", var.cyan(), super::format_number(*value));
        }
        println!();
    }

    println!("   {}", "Expected Values (probability-weighted):".bold());
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
        println!(
            "      {}: {}   (range {} to {}, P(>0) = {}%)",
            var.bright_blue(),
            super::format_number(*ev),
            super::format_number(low),
            super::format_number(high),
            super::format_number(positive * 100.0)
        );
    }
    if scenario_filter.is_some() {
        println!("      (expected values cover every scenario)");
    }
    println!();

    // Write output if specified
    if let Some(output_path) = output_file {
        let output_str = serialize_for_file(&results, &output_path)?;
        fs::write(&output_path, output_str).map_err(ForgeError::Io)?;
        println!(
            "{}",
            format!("💾 Results written to {}", output_path.display())
                .bold()
                .green()
        );
    }

    println!("{}", "✅ Scenario analysis complete".bold().green());
    Ok(())
}

/// Execute the decision-tree command - backward induction analysis
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no `decision_tree` section exists,
/// or the analysis fails.
pub fn decision_tree(
    file: &Path,
    export_dot: bool,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🌳 Forge - Decision Tree Analysis".bold().green());
    println!("   File: {}", file.display());
    println!();

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
    println!("   {}", format!("Tree: {}", config.name).bold());
    if verbose {
        println!("      Root: {:?}", config.root.as_ref().map(|r| &r.name));
        println!("      Nodes: {}", config.nodes.len());
    }
    println!();

    // Create engine and analyze
    let engine = DecisionTreeEngine::new(config).map_err(ForgeError::Validation)?;

    if verbose {
        println!("{}", "🔄 Running backward induction...".cyan());
    }

    let result = engine.analyze().map_err(ForgeError::Eval)?;

    // Display results
    println!("{}", "📊 Decision Tree Results:".bold().green());
    println!();

    // Optimal path
    println!("   {}", "Optimal Path:".bold());
    for step in &result.optimal_path {
        println!("      → {}", step.cyan());
    }
    println!();

    // Expected value at root
    println!(
        "   Expected Value: {}",
        format!("${:.2}", result.root_expected_value).bold().green()
    );

    // Decision policy
    println!();
    println!("   {}", "Decision Policy:".bold());
    for (node, choice) in &result.decision_policy {
        println!("      At \"{}\": choose \"{}\"", node.bright_blue(), choice);
        if let Some(tied) = result
            .node_results
            .get(node)
            .filter(|n| !n.tied_with.is_empty())
        {
            println!(
                "         {} equally good: {} (the expected values are the same; the choice is not made by the numbers)",
                "⚠️".yellow(),
                tied.tied_with.join(", ")
            );
        }
    }

    // Risk profile
    println!();
    println!("   {}", "Risk Profile:".bold());
    println!("      Best case:  ${:.2}", result.risk_profile.best_case);
    println!("      Worst case: ${:.2}", result.risk_profile.worst_case);
    println!(
        "      P(value > 0): {:.1}%",
        result.risk_profile.probability_positive * 100.0
    );
    println!("      Outcomes under the optimal policy:");
    for outcome in &result.risk_profile.outcomes {
        println!(
            "         ${:.2} with probability {:.1}%: {}",
            outcome.value,
            outcome.probability * 100.0,
            outcome.path.join(" → ")
        );
    }
    println!();

    // Export DOT if requested (feature not yet implemented, just note it)
    if export_dot {
        println!(
            "{}",
            "⚠️  DOT export not yet implemented. Use --output for YAML export.".yellow()
        );
    }

    if let Some(output_path) = output_file {
        let output_str = serialize_for_file(&result, &output_path)?;
        fs::write(&output_path, output_str).map_err(ForgeError::Io)?;
        println!(
            "{}",
            format!("💾 Results written to {}", output_path.display())
                .bold()
                .green()
        );
    }

    println!("{}", "✅ Decision tree analysis complete".bold().green());
    Ok(())
}

/// Execute the real-options command - value managerial flexibility
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no `real_options` section exists,
/// the option filter does not match, or the analysis fails.
pub fn real_options(
    file: &Path,
    option_filter: Option<&str>,
    compare_npv: bool,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "💎 Forge - Real Options Analysis".bold().green());
    println!("   File: {}", file.display());
    println!();

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
    println!("   {}", format!("Analysis: {}", config.name).bold());
    println!(
        "      Method: {}",
        format!("{:?}", config.method).bright_blue()
    );
    println!(
        "      Underlying value: ${:.2}",
        config.underlying.current_value
    );
    println!(
        "      Volatility: {:.1}%",
        config.underlying.volatility * 100.0
    );
    println!(
        "      Risk-free rate: {:.1}%",
        config.underlying.risk_free_rate * 100.0
    );
    println!(
        "      Time horizon: {} years",
        config.underlying.time_horizon
    );
    println!();

    // Create engine and value options
    let engine = RealOptionsEngine::new(config).map_err(ForgeError::Validation)?;

    if verbose {
        println!("{}", "🔄 Valuing options...".cyan());
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
    print_real_options_results(&result, &order, option_filter, compare_npv);

    // Write output if specified
    if let Some(output_path) = output_file {
        let output_str = result.to_yaml();
        fs::write(&output_path, output_str).map_err(ForgeError::Io)?;
        println!(
            "{}",
            format!("💾 Results written to {}", output_path.display())
                .bold()
                .green()
        );
    }

    println!("{}", "✅ Real options analysis complete".bold().green());
    Ok(())
}

/// Execute the tornado command - sensitivity tornado diagram
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no tornado section exists,
/// or the analysis fails.
pub fn tornado(
    file: &Path,
    output_var: Option<&str>,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🌪️ Forge - Tornado Diagram".bold().green());
    println!("   File: {}", file.display());
    println!();

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
    println!("   Output variable: {}", config.output.bright_blue());
    println!("   Inputs to vary: {}", config.inputs.len());
    if verbose {
        for input in &config.inputs {
            println!(
                "      {} [{:.2} - {:.2}]",
                input.name.cyan(),
                input.low,
                input.high
            );
        }
    }
    println!();

    // Create engine and analyze
    let engine = TornadoEngine::new(config, model).map_err(ForgeError::Validation)?;

    if verbose {
        println!("{}", "🔄 Calculating sensitivities...".cyan());
    }

    let result = engine.analyze().map_err(ForgeError::Eval)?;

    // Display results
    println!("{}", "📊 Tornado Diagram:".bold().green());
    println!(
        "   Base value: {}",
        format!("{:.2}", result.base_value).bold()
    );
    println!();

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
        println!(
            "   {:name_width$} |{:30}| swing {:.2}  (output {:.2} at low, {:.2} at high)",
            bar.input_name.bright_blue(),
            bar_str.cyan(),
            bar.swing.abs(),
            bar.output_at_low,
            bar.output_at_high
        );
    }
    println!();

    // Write output if specified
    if let Some(output_path) = output_file {
        let output_str = serialize_for_file(&result, &output_path)?;
        fs::write(&output_path, output_str).map_err(ForgeError::Io)?;
        println!(
            "{}",
            format!("💾 Results written to {}", output_path.display())
                .bold()
                .green()
        );
    }

    println!("{}", "✅ Tornado diagram complete".bold().green());
    Ok(())
}

/// Execute the bootstrap command - resampling confidence intervals
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no bootstrap section exists,
/// or the analysis fails.
pub fn bootstrap(
    file: &Path,
    iterations_override: Option<usize>,
    seed_override: Option<u64>,
    confidence_override: Option<Vec<f64>>,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🔄 Forge - Bootstrap Resampling".bold().green());
    println!("   File: {}", file.display());
    println!();

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
    println!("   {}", "Configuration:".bold());
    println!(
        "      Iterations: {}",
        config.iterations.to_string().bright_blue()
    );
    println!(
        "      Statistic: {}",
        format!("{:?}", config.statistic).bright_blue()
    );
    println!("      Data points: {}", config.data.len());
    println!("      Confidence levels: {:?}", config.confidence_levels);
    println!("      Interval method: {:?}", config.method);
    if let Some(seed) = config.seed {
        println!("      Seed: {seed}");
    }
    println!();

    // Create engine and analyze
    let mut engine = BootstrapEngine::new(config).map_err(ForgeError::Validation)?;

    if verbose {
        println!("{}", "🔄 Resampling...".cyan());
    }

    let result = engine.analyze().map_err(ForgeError::Eval)?;

    // Display results
    println!("{}", "📊 Bootstrap Results:".bold().green());
    println!();

    println!(
        "   Original statistic: {}",
        format!("{:.4}", result.original_estimate).bold()
    );
    println!("   Bootstrap mean: {:.4}", result.bootstrap_mean);
    println!("   Bootstrap std error: {:.4}", result.bootstrap_std_error);
    println!("   Bias: {:.4}", result.bias);
    println!();

    // Confidence intervals
    println!("   {}", "Confidence Intervals:".bold());
    for ci in &result.confidence_intervals {
        println!(
            "      {:.0}% CI: [{:.4}, {:.4}]",
            ci.level * 100.0,
            ci.lower,
            ci.upper
        );
    }
    println!();

    // Write output if specified
    if let Some(output_path) = output_file {
        let output_str = serialize_for_file(&result, &output_path)?;
        fs::write(&output_path, output_str).map_err(ForgeError::Io)?;
        println!(
            "{}",
            format!("💾 Results written to {}", output_path.display())
                .bold()
                .green()
        );
    }

    println!("{}", "✅ Bootstrap analysis complete".bold().green());
    Ok(())
}

/// Execute the bayesian command - Bayesian network inference
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no `bayesian_network` section exists,
/// or inference fails.
pub fn bayesian(
    file: &Path,
    query_var: Option<&str>,
    evidence: &[String],
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🔮 Forge - Bayesian Network Inference".bold().green());
    println!("   File: {}", file.display());
    println!();

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

    // Display config
    println!("   {}", format!("Network: {}", config.name).bold());
    println!("   Nodes: {}", config.nodes.len());
    if verbose {
        for (name, node) in &config.nodes {
            let parents = if node.parents.is_empty() {
                "root".to_string()
            } else {
                node.parents.join(", ")
            };
            println!(
                "      {} ({} states, parents: {})",
                name.bright_blue(),
                node.states.len(),
                parents
            );
        }
    }
    println!();

    // Parse evidence
    let mut evidence_map: HashMap<String, &str> = HashMap::new();
    for ev in evidence {
        let parts: Vec<&str> = ev.split('=').collect();
        if parts.len() == 2 {
            evidence_map.insert(parts[0].to_string(), parts[1]);
        }
    }

    if !evidence_map.is_empty() {
        println!("   {}", "Evidence:".bold());
        for (var, val) in &evidence_map {
            println!("      {} = {}", var.bright_blue(), val.cyan());
        }
        println!();
    }

    // Create engine
    let engine = BayesianEngine::new(config).map_err(ForgeError::Validation)?;

    if verbose {
        println!("{}", "🔄 Running inference...".cyan());
    }

    // Run query
    if let Some(target) = query_var {
        let var_result = if evidence_map.is_empty() {
            println!("{}", "📊 Query Result:".bold().green());
            engine.query(target).map_err(ForgeError::Eval)?
        } else {
            println!("{}", "📊 Query Result (with evidence):".bold().green());
            engine
                .query_with_evidence(target, &evidence_map)
                .map_err(ForgeError::Eval)?
        };
        print_bayesian_var_result(target, &var_result);
        if let Some(output_path) = output_file {
            let output_str = serialize_for_file(&var_result, &output_path)?;
            fs::write(&output_path, output_str).map_err(ForgeError::Io)?;
            println!(
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

        println!("{}", "📊 All Node Probabilities:".bold().green());
        println!();

        for (name, var_result) in &all_results.queries {
            print_bayesian_var_result(name, var_result);
        }

        if let Some(output_path) = output_file {
            let output_str = serialize_for_file(&all_results, &output_path)?;
            fs::write(&output_path, output_str).map_err(ForgeError::Io)?;
            println!(
                "{}",
                format!("💾 Results written to {}", output_path.display())
                    .bold()
                    .green()
            );
        }
    }

    println!("{}", "✅ Bayesian inference complete".bold().green());
    Ok(())
}

/// Print real options analysis results
fn print_real_options_results(
    result: &crate::real_options::OptionsResult,
    order: &[String],
    option_filter: Option<&str>,
    compare_npv: bool,
) {
    println!("{}", "📊 Real Options Results:".bold().green());
    println!();

    match result.traditional_npv {
        Some(npv) => println!(
            "   Traditional NPV (invest now, no flexibility): {}",
            format!("${npv:.2}").yellow()
        ),
        None if compare_npv => println!(
            "   Traditional NPV: {}",
            "not given (set traditional_npv in real_options)".yellow()
        ),
        None => {},
    }
    println!();

    println!("   {}", "Options (each valued on its own):".bold());
    for name in order {
        if option_filter.is_some_and(|filter| filter != name) {
            continue;
        }
        let Some(opt_result) = result.options.get(name) else {
            continue;
        };
        println!(
            "      {} ({}): option value {}",
            opt_result.name.bright_blue(),
            format!("{:?}", opt_result.option_type).dimmed(),
            format!("${:.2}", opt_result.value).bold().green()
        );
        match opt_result.project_value {
            Some(pv) => println!("         Project value with this option: ${pv:.2}"),
            None => println!("         Project value with this option: needs traditional_npv"),
        }
        if let Some(ref trigger) = opt_result.optimal_trigger {
            println!("         Rule of thumb: exercise when {trigger}");
        }
        if let Some(prob) = opt_result.probability_exercise {
            println!("         P(exercise): {:.1}%", prob * 100.0);
        }
    }
    println!();

    if let (Some(best), Some(pv)) = (&result.best_option, result.project_value_with_options) {
        println!(
            "   Project value with the best option ({}): {}",
            best.bright_blue(),
            format!("${pv:.2}").bold().green()
        );
    }
    if result.options.len() > 1 {
        println!(
            "   {}",
            "Option values overlap, so they are compared, not added.".dimmed()
        );
    }

    println!();
    println!(
        "   {}: {}",
        "Decision".bold(),
        result.decision.bright_yellow()
    );
    println!("   {}: {}", "Recommendation".bold(), result.recommendation);
    println!();
}

/// Print Bayesian variable result with probability bars
fn print_bayesian_var_result(name: &str, var_result: &crate::bayesian::VariableResult) {
    println!("   {}:", name.bright_blue().bold());
    for (state, prob) in var_result
        .states
        .iter()
        .zip(var_result.probabilities.iter())
    {
        // Truncation/sign-loss impossible: prob is in 0.0..=1.0, result in 0.0..=30.0
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let bar_width = (prob * 30.0) as usize;
        let bar = "█".repeat(bar_width.max(1));
        println!("      {:15} |{}| {:.2}%", state, bar.cyan(), prob * 100.0);
    }
    println!();
    println!(
        "   Most likely: {} ({:.1}%)",
        var_result.most_likely.bold().green(),
        var_result.max_probability * 100.0
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
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
        bootstrap(&model, None, None, None, Some(yaml_out.clone()), false).unwrap();
        let doc: yaml_serde::Value =
            yaml_serde::from_str(&std::fs::read_to_string(&yaml_out).unwrap()).unwrap();
        assert_eq!(doc["original_estimate"].as_f64(), Some(3.0));
        assert!(doc["confidence_intervals"].is_sequence());

        let json_out = dir.path().join("out.json");
        bootstrap(&model, None, None, None, Some(json_out.clone()), false).unwrap();
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
        decision_tree(&model, false, Some(out.clone()), false).unwrap();
        let doc: yaml_serde::Value =
            yaml_serde::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert!(doc["root_expected_value"].as_f64().is_some());
    }
}
