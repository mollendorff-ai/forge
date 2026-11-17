//! `forge scenarios`: the model under each scenario, weighted by its
//! probability.

use super::serialize_for_file;
use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use crate::scenarios::{ScenarioConfig, ScenarioEngine};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

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
    let formats = crate::cli::commands::scalar_formats(&parser::parse_model(file)?);
    let shown = |var: &str, value: f64| {
        formats
            .get(var)
            .and_then(|f| crate::cli::commands::number_shown_by(value, f))
            .unwrap_or_else(|| crate::cli::commands::display_number(value))
    };

    // Display results: what each scenario produced (the calculated scalars)
    outln!(c, "{}", "📈 Scenario Results:".bold().green());
    outln!(c);
    for result in &results.scenarios {
        outln!(
            c,
            "   {} (p={}%): {}",
            result.name.bright_blue().bold(),
            crate::cli::commands::format_number(result.probability * 100.0),
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
            crate::cli::commands::format_number(positive * 100.0)
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

#[cfg(test)]
mod tests {
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
}
