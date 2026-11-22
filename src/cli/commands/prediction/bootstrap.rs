//! `forge bootstrap`: confidence intervals by resampling the data.

use super::write_result;
use crate::bootstrap::{BootstrapConfig, BootstrapEngine};
use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use colored::Colorize;
use std::path::{Path, PathBuf};

/// The `bootstrap:` section of a model, with the overrides of the command
/// line applied.
fn load_config(
    file: &Path,
    iterations_override: Option<usize>,
    seed_override: Option<u64>,
    confidence_override: Option<Vec<f64>>,
) -> ForgeResult<BootstrapConfig> {
    let yaml_content = parser::read_model_text(file)?;

    let value: yaml_serde::Value = parser::read_yaml(&yaml_content)
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
    Ok(config)
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
    let config = load_config(
        file,
        iterations_override,
        seed_override,
        confidence_override,
    )?;
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

    let config = load_config(
        file,
        iterations_override,
        seed_override,
        confidence_override,
    )?;

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
        write_result(c, &result, &output_path)?;
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

    fn model(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    const SMALL: &str = "_forge_version: \"5.0.0\"\n\nbootstrap:\n  iterations: 50\n  seed: 1\n  confidence_levels: [0.95]\n  data: [1.0, 2.0, 3.0]\n  statistic: mean\n";

    /// Each way of failing, the same from the library and from the command.
    #[test]
    fn what_cannot_be_read_or_resampled_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let bca = SMALL
            .replace("[1.0, 2.0, 3.0]", "[1.0, 1.0, 1.0]")
            .replace("mean\n", "mean\n  method: bca\n");
        for (text, iterations, error) in [
            (
                "a: [1\n",
                None,
                "Validation error: YAML parse error: did not find expected ',' or ']' at line 2 column 1, while parsing a flow sequence at line 1 column 4",
            ),
            (
                "_forge_version: \"5.0.0\"\nx: 1\n",
                None,
                "Validation error: No 'bootstrap' section found in YAML",
            ),
            (
                "_forge_version: \"5.0.0\"\nbootstrap:\n  iterations: many\n",
                None,
                "Validation error: bootstrap config error: invalid type: string \"many\", expected usize",
            ),
            (SMALL, Some(0), "Validation error: Iterations must be positive"),
            (
                &bca,
                None,
                "Formula evaluation error: BCa interval undefined: every bootstrap value is on the same side of the estimate; use method percentile or basic",
            ),
        ] {
            let file = model(dir.path(), "m.yaml", text);
            let core = bootstrap_core(&file, iterations, None, None).unwrap_err();
            assert_eq!(core.to_string(), error);
            let command = capture(|c| bootstrap(c, &file, iterations, None, None, None, false)).0;
            assert_eq!(command.unwrap_err().to_string(), error);
        }
        let missing = dir.path().join("none.yaml");
        let gone = "IO error: No such file or directory (os error 2)";
        assert_eq!(
            bootstrap_core(&missing, None, None, None)
                .unwrap_err()
                .to_string(),
            gone
        );
        let ok = model(dir.path(), "ok.yaml", SMALL);
        let nowhere = dir.path().join("nodir").join("out.yaml");
        let result = capture(|c| bootstrap(c, &ok, None, None, None, Some(nowhere), false)).0;
        assert_eq!(result.unwrap_err().to_string(), gone);
    }

    /// The command line's iterations, seed and confidence levels replace the
    /// model's, in the library and in what the command prints.
    #[test]
    fn the_overrides_replace_the_models_settings() {
        let dir = tempfile::tempdir().unwrap();
        let ok = model(dir.path(), "ok.yaml", SMALL);
        let result = bootstrap_core(&ok, Some(10), Some(9), Some(vec![0.5])).unwrap();
        assert_eq!(result.confidence_intervals.len(), 1);
        assert!((result.confidence_intervals[0].level - 0.5).abs() <= 1e-12);
        let (done, printed) =
            capture(|c| bootstrap(c, &ok, Some(10), Some(9), Some(vec![0.5]), None, true));
        done.unwrap();
        for line in [
            "      Iterations: 10\n",
            "      Confidence levels: [0.5]\n",
            "      Seed: 9\n",
            "🔄 Resampling...\n",
            "      50% CI: [",
            "✅ Bootstrap analysis complete\n",
        ] {
            assert!(printed.out.contains(line), "{line}\n{printed:?}");
        }
        let unseeded = model(dir.path(), "u.yaml", &SMALL.replace("  seed: 1\n", ""));
        let (done, printed) = capture(|c| bootstrap(c, &unseeded, None, None, None, None, false));
        done.unwrap();
        assert!(!printed.out.contains("Seed:"), "{printed:?}");
        assert!(!printed.out.contains("🔄 Resampling..."), "{printed:?}");
        assert!(
            printed.out.contains("      Iterations: 50\n"),
            "{printed:?}"
        );
    }

    mod acquisition_example {
        //! examples/acquisition/premiums.yaml: the median premium of 24 comparable
        //! takeovers is 0.335, and 8 of the 24 paid less than 30%. numpy, 200,000
        //! resamples: standard error 0.0225, 90% interval [0.30, 0.375], 95%
        //! [0.29, 0.38]. Forge's own resampler (seed 11) must agree within
        //! resampling noise: a 30% offer sits at the bottom of the 90% interval.

        use super::super::bootstrap_core;
        use std::path::Path;

        #[test]
        fn a_30_percent_premium_is_at_the_bottom_of_the_range() {
            let r = bootstrap_core(
                Path::new("examples/acquisition/premiums.yaml"),
                None,
                None,
                None,
            )
            .unwrap();
            assert!((r.original_estimate - 0.335).abs() <= 1e-12);
            assert!((r.bootstrap_std_error - 0.0225).abs() <= 0.002);
            for (level, lower, upper) in [(0.90, 0.30, 0.375), (0.95, 0.29, 0.38)] {
                let ci = r
                    .confidence_intervals
                    .iter()
                    .find(|c| (c.level - level).abs() <= 1e-12)
                    .unwrap();
                assert!((ci.lower - lower).abs() <= 0.006, "{level}: {}", ci.lower);
                assert!((ci.upper - upper).abs() <= 0.006, "{level}: {}", ci.upper);
            }
        }
    }
}
