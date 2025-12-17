//! Prediction Methods CLI Commands (Enterprise Only)
//!
//! CLI handlers for advanced forecasting methods:
//! - scenarios: Run scenario analysis (Base/Bull/Bear)
//! - decision_tree: Analyze decision trees with backward induction
//! - real_options: Value managerial flexibility (defer/expand/abandon)
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
use std::path::PathBuf;

/// Execute the scenarios command - probability-weighted scenario analysis
pub fn scenarios(
    file: PathBuf,
    scenario_filter: Option<String>,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "📊 Forge - Scenario Analysis".bold().green());
    println!("   File: {}", file.display());
    println!();

    // Parse YAML
    let yaml_content = fs::read_to_string(&file).map_err(ForgeError::Io)?;
    let model = parser::parse_model(&file)?;

    // Parse scenarios config
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let config: ScenarioConfig = if let Some(scenarios_value) = value.get("scenarios") {
        let scenarios_map: HashMap<String, serde_yaml_ng::Value> =
            serde_yaml_ng::from_value(scenarios_value.clone())
                .map_err(|e| ForgeError::Validation(format!("scenarios config error: {e}")))?;

        // Convert to ScenarioConfig
        let mut config = ScenarioConfig::default();
        for (name, def) in scenarios_map {
            let scenario_def = serde_yaml_ng::from_value(def)
                .map_err(|e| ForgeError::Validation(format!("scenario '{name}' error: {e}")))?;
            config.scenarios.insert(name, scenario_def);
        }
        config
    } else {
        return Err(ForgeError::Validation(
            "No 'scenarios' section found in YAML".to_string(),
        ));
    };

    // Display config
    println!("   {}", "Scenarios:".bold());
    for (name, def) in &config.scenarios {
        println!(
            "      {} (p={:.0}%): {}",
            name.bright_blue(),
            def.probability * 100.0,
            &def.description
        );
    }
    println!();

    // Create engine and run
    let engine = ScenarioEngine::new(config, model).map_err(ForgeError::Validation)?;

    let results = if let Some(ref filter) = scenario_filter {
        // Run single scenario
        if verbose {
            println!("{}", format!("🎯 Running scenario: {filter}").cyan());
        }
        engine.run().map_err(ForgeError::Eval)?
    } else {
        // Run all scenarios
        if verbose {
            println!("{}", "🔄 Running all scenarios...".cyan());
        }
        engine.run().map_err(ForgeError::Eval)?
    };

    // Display results
    println!("{}", "📈 Scenario Results:".bold().green());
    println!();

    for result in &results.scenarios {
        println!("   {}:", result.name.bright_blue().bold());
        println!("      Probability: {:.1}%", result.probability * 100.0);
        println!("      Key Scalars:");
        for (var, value) in result.scalars.iter().take(5) {
            println!("         {}: {:.2}", var.cyan(), value);
        }
        println!();
    }

    // Expected value
    println!("   {}", "Expected Values (probability-weighted):".bold());
    for (var, ev) in &results.expected_values {
        println!("      {}: {:.2}", var.bright_blue(), ev);
    }
    println!();

    // Write output if specified
    if let Some(output_path) = output_file {
        let output_str = format!("{results:#?}");
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
pub fn decision_tree(
    file: PathBuf,
    export_dot: bool,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🌳 Forge - Decision Tree Analysis".bold().green());
    println!("   File: {}", file.display());
    println!();

    // Parse YAML
    let yaml_content = fs::read_to_string(&file).map_err(ForgeError::Io)?;

    // Parse decision_tree config
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let config: DecisionTreeConfig = if let Some(dt_value) = value.get("decision_tree") {
        serde_yaml_ng::from_value(dt_value.clone())
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
    println!();

    // Export DOT if requested (feature not yet implemented, just note it)
    if export_dot {
        println!(
            "{}",
            "⚠️  DOT export not yet implemented. Use --output for YAML export.".yellow()
        );
    }

    if let Some(output_path) = output_file {
        let output_str = format!("{result:#?}");
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
pub fn real_options(
    file: PathBuf,
    option_filter: Option<String>,
    compare_npv: bool,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "💎 Forge - Real Options Analysis".bold().green());
    println!("   File: {}", file.display());
    println!();

    // Parse YAML
    let yaml_content = fs::read_to_string(&file).map_err(ForgeError::Io)?;

    // Parse real_options config
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let config: RealOptionsConfig = if let Some(ro_value) = value.get("real_options") {
        serde_yaml_ng::from_value(ro_value.clone())
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
    let engine = RealOptionsEngine::new(config.clone()).map_err(ForgeError::Validation)?;

    if verbose {
        println!("{}", "🔄 Valuing options...".cyan());
    }

    let result = engine.analyze().map_err(ForgeError::Eval)?;

    // Filter if specific option requested
    if let Some(ref filter) = option_filter {
        if !result.options.contains_key(filter) {
            return Err(ForgeError::Validation(format!(
                "Option '{}' not found. Available: {:?}",
                filter,
                result.options.keys().collect::<Vec<_>>()
            )));
        }
    }

    // Display results
    println!("{}", "📊 Real Options Results:".bold().green());
    println!();

    // Traditional NPV
    if compare_npv {
        println!(
            "   Traditional NPV: {}",
            format!("${:.2}", result.traditional_npv).yellow()
        );
    }

    // Option values
    println!("   {}", "Option Values:".bold());
    for (name, opt_result) in &result.options {
        // If filter specified, only show that option
        if let Some(ref filter) = option_filter {
            if name != filter {
                continue;
            }
        }
        println!(
            "      {} ({}): {}",
            opt_result.name.bright_blue(),
            format!("{:?}", opt_result.option_type).dimmed(),
            format!("${:.2}", opt_result.value).bold().green()
        );
        if let Some(ref trigger) = opt_result.optimal_trigger {
            println!("         Trigger: {trigger}");
        }
        if let Some(prob) = opt_result.probability_exercise {
            println!("         P(exercise): {:.1}%", prob * 100.0);
        }
    }
    println!();

    // Total option value
    println!(
        "   Total Option Value: {}",
        format!("${:.2}", result.total_option_value).bold().green()
    );

    // Project value with options
    println!(
        "   Project Value (with options): {}",
        format!("${:.2}", result.project_value_with_options)
            .bold()
            .green()
    );

    // Decision
    println!();
    println!(
        "   {}: {}",
        "Decision".bold(),
        result.decision.bright_yellow()
    );
    println!("   {}: {}", "Recommendation".bold(), result.recommendation);
    println!();

    // Write output if specified
    if let Some(output_path) = output_file {
        let output_str = format!("{result:#?}");
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
pub fn tornado(
    file: PathBuf,
    output_var: Option<String>,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🌪️ Forge - Tornado Diagram".bold().green());
    println!("   File: {}", file.display());
    println!();

    // Parse YAML and model
    let yaml_content = fs::read_to_string(&file).map_err(ForgeError::Io)?;
    let model = parser::parse_model(&file)?;

    // Parse tornado config
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let mut config: TornadoConfig = if let Some(tornado_value) = value.get("tornado") {
        serde_yaml_ng::from_value(tornado_value.clone())
            .map_err(|e| ForgeError::Validation(format!("tornado config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'tornado' section found in YAML".to_string(),
        ));
    };

    // Override output variable if specified
    if let Some(ref out_var) = output_var {
        config.output = out_var.clone();
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

    for bar in &result.bars {
        let bar_width = if max_impact > 0.0 {
            ((bar.swing.abs() / max_impact) * 30.0) as usize
        } else {
            0
        };
        let bar_str = "█".repeat(bar_width.max(1));

        println!(
            "   {:20} |{}| ±{:.2}",
            bar.input_name.bright_blue(),
            bar_str.cyan(),
            bar.swing.abs()
        );
        if verbose {
            println!(
                "                        Low: {:.2}  High: {:.2}",
                bar.output_at_low, bar.output_at_high
            );
        }
    }
    println!();

    // Write output if specified
    if let Some(output_path) = output_file {
        let output_str = format!("{result:#?}");
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
pub fn bootstrap(
    file: PathBuf,
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
    let yaml_content = fs::read_to_string(&file).map_err(ForgeError::Io)?;

    // Parse bootstrap config
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let mut config: BootstrapConfig = if let Some(bs_value) = value.get("bootstrap") {
        serde_yaml_ng::from_value(bs_value.clone())
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
        let output_str = format!("{result:#?}");
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
pub fn bayesian(
    file: PathBuf,
    query_var: Option<String>,
    evidence: Vec<String>,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🔮 Forge - Bayesian Network Inference".bold().green());
    println!("   File: {}", file.display());
    println!();

    // Parse YAML
    let yaml_content = fs::read_to_string(&file).map_err(ForgeError::Io)?;

    // Parse bayesian_network config
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let config: BayesianConfig = if let Some(bn_value) = value.get("bayesian_network") {
        serde_yaml_ng::from_value(bn_value.clone())
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
    for ev in &evidence {
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
    if let Some(ref target) = query_var {
        if evidence_map.is_empty() {
            let var_result = engine.query(target).map_err(ForgeError::Eval)?;
            println!("{}", "📊 Query Result:".bold().green());
            println!("   {}:", target.bright_blue().bold());
            for (state, prob) in var_result
                .states
                .iter()
                .zip(var_result.probabilities.iter())
            {
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

            // Write output if specified
            if let Some(output_path) = output_file {
                let output_str = format!("{var_result:#?}");
                fs::write(&output_path, output_str).map_err(ForgeError::Io)?;
                println!(
                    "{}",
                    format!("💾 Results written to {}", output_path.display())
                        .bold()
                        .green()
                );
            }
        } else {
            let var_result = engine
                .query_with_evidence(target, &evidence_map)
                .map_err(ForgeError::Eval)?;
            println!("{}", "📊 Query Result (with evidence):".bold().green());
            println!("   {}:", target.bright_blue().bold());
            for (state, prob) in var_result
                .states
                .iter()
                .zip(var_result.probabilities.iter())
            {
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

            // Write output if specified
            if let Some(output_path) = output_file {
                let output_str = format!("{var_result:#?}");
                fs::write(&output_path, output_str).map_err(ForgeError::Io)?;
                println!(
                    "{}",
                    format!("💾 Results written to {}", output_path.display())
                        .bold()
                        .green()
                );
            }
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
            println!("   {}:", name.bright_blue().bold());
            for (state, prob) in var_result
                .states
                .iter()
                .zip(var_result.probabilities.iter())
            {
                let bar_width = (prob * 30.0) as usize;
                let bar = "█".repeat(bar_width.max(1));
                println!("      {:15} |{}| {:.2}%", state, bar.cyan(), prob * 100.0);
            }
            println!();
        }

        // Write output if specified
        if let Some(output_path) = output_file {
            let output_str = format!("{all_results:#?}");
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
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
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
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
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
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
        let bs_value = value.get("bootstrap").unwrap();
        let config: BootstrapConfig = serde_yaml_ng::from_value(bs_value.clone()).unwrap();
        assert_eq!(config.iterations, 1000);
        assert_eq!(config.data.len(), 5);
    }
}
