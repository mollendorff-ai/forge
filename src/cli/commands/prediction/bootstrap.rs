//! `forge bootstrap`: confidence intervals by resampling the data.

use super::serialize_for_file;
use crate::bootstrap::{BootstrapConfig, BootstrapEngine};
use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::console::capture;

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
}
