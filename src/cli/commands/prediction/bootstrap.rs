//! `forge bootstrap`: confidence intervals by resampling the data.

use super::write_result;
use crate::bootstrap::{BootstrapConfig, BootstrapEngine};
use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use colored::Colorize;
use std::path::{Path, PathBuf};

/// The `bootstrap` section of a model file, with the command line's overrides.
fn read_config(
    file: &Path,
    iterations_override: Option<usize>,
    seed_override: Option<u64>,
    confidence_override: Option<Vec<f64>>,
) -> ForgeResult<BootstrapConfig> {
    let yaml_content = parser::read_model_text(file)?;
    let value: yaml_serde::Value = parser::read_yaml(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;
    let bs_value = value.get("bootstrap").ok_or_else(|| {
        ForgeError::Validation("No 'bootstrap' section found in YAML".to_string())
    })?;
    let mut config: BootstrapConfig = yaml_serde::from_value(bs_value.clone())
        .map_err(|e| ForgeError::Validation(format!("bootstrap config error: {e}")))?;
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

/// Resample the data of a configuration.
fn analyze(config: BootstrapConfig) -> ForgeResult<crate::bootstrap::BootstrapResult> {
    let mut engine = BootstrapEngine::new(config).map_err(ForgeError::Validation)?;
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
    analyze(read_config(
        file,
        iterations_override,
        seed_override,
        confidence_override,
    )?)
}

/// Execute the bootstrap command - resampling confidence intervals
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no bootstrap section exists,
/// the analysis fails or the `--output` file cannot be written.
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

    let config = read_config(
        file,
        iterations_override,
        seed_override,
        confidence_override,
    )?;

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

    if verbose {
        outln!(c, "{}", "🔄 Resampling...".cyan());
    }

    let result = analyze(config)?;

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

    if let Some(output_path) = output_file {
        write_result(c, &result, &output_path)?;
    }

    outln!(c, "{}", "✅ Bootstrap analysis complete".bold().green());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `forge bootstrap` prints (the console is not a terminal: no colours) and how it ends
    fn run(
        file: &Path,
        overrides: (Option<usize>, Option<u64>, Option<Vec<f64>>),
        output: Option<PathBuf>,
        verbose: bool,
    ) -> (String, Result<(), String>) {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut input = std::io::empty();
        let mut console = Console::new(&mut out, &mut err, &mut input);
        let (iterations, seed, levels) = overrides;
        let result = bootstrap(
            &mut console,
            file,
            iterations,
            seed,
            levels,
            output,
            verbose,
        );
        (
            String::from_utf8(out).unwrap(),
            result.map_err(|e| e.to_string()),
        )
    }

    /// A model file of one `bootstrap` section
    fn model(dir: &Path, name: &str, section: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(
            &path,
            format!("_forge_version: \"5.0.0\"\n\nbootstrap:\n{section}"),
        )
        .unwrap();
        path
    }

    /// Data of one value: every resample is that value, whatever the draws,
    /// so the estimate, the mean and both ends of each interval are 2 and the
    /// error and the bias 0
    const FLAT: &str = "  iterations: 50\n  confidence_levels: [0.90, 0.95]\n  data: [2.0, 2.0, 2.0]\n  statistic: mean\n";

    #[test]
    fn the_configuration_the_steps_and_the_results_are_printed() {
        let dir = tempfile::tempdir().unwrap();
        let path = model(dir.path(), "flat.yaml", FLAT);
        let (text, result) = run(&path, (None, Some(7), None), None, true);
        assert_eq!(
            (text, result),
            (
                format!(
                    "🔄 Forge - Bootstrap Resampling\n   File: {}\n\n   Configuration:\n      Iterations: 50\n      Statistic: Mean\n      Data points: 3\n      Confidence levels: [0.9, 0.95]\n      Interval method: Percentile\n      Seed: 7\n\n🔄 Resampling...\n📊 Bootstrap Results:\n\n   Original statistic: 2.0000\n   Bootstrap mean: 2.0000\n   Bootstrap std error: 0.0000\n   Bias: 0.0000\n\n   Confidence Intervals:\n      90% CI: [2.0000, 2.0000]\n      95% CI: [2.0000, 2.0000]\n\n✅ Bootstrap analysis complete\n",
                    path.display()
                ),
                Ok(())
            )
        );
    }

    /// Without a seed there is no seed line and without `--verbose` no step
    #[test]
    fn without_a_seed_or_verbose_neither_is_printed() {
        let dir = tempfile::tempdir().unwrap();
        let path = model(dir.path(), "flat.yaml", FLAT);
        let (text, result) = run(&path, (None, None, Some(vec![0.5])), None, false);
        assert_eq!(
            (
                text.contains("Seed:"),
                text.contains("Resampling..."),
                result
            ),
            (false, false, Ok(()))
        );
        assert!(
            text.contains("      Confidence levels: [0.5]\n")
                && text.contains("      50% CI: [2.0000, 2.0000]\n"),
            "{text}"
        );
    }

    /// Each override replaces the file's value: one the configuration refuses
    /// shows it was used, and the same seed draws the same resamples
    #[test]
    fn the_command_lines_values_override_the_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = model(
            dir.path(),
            "m.yaml",
            "  iterations: 200\n  seed: 1\n  confidence_levels: [0.95]\n  data: [1.0, 2.0, 3.0, 4.0, 5.0]\n  statistic: mean\n",
        );
        assert_eq!(
            bootstrap_core(&path, Some(0), None, None)
                .unwrap_err()
                .to_string(),
            "Validation error: Iterations must be positive"
        );
        assert_eq!(
            bootstrap_core(&path, None, None, Some(vec![1.5]))
                .unwrap_err()
                .to_string(),
            "Validation error: Confidence level 1.5 must be between 0 and 1"
        );
        let a = bootstrap_core(&path, Some(30), Some(99), None).unwrap();
        let b = bootstrap_core(&path, Some(30), Some(99), None).unwrap();
        assert_eq!(
            (
                a.bootstrap_mean,
                a.bootstrap_std_error,
                a.distribution.len()
            ),
            (b.bootstrap_mean, b.bootstrap_std_error, 30)
        );
    }

    /// Each way a model file cannot be resampled, with the message, the
    /// same from `bootstrap_core`
    #[test]
    fn a_file_that_cannot_be_resampled_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        for (name, text, want) in [
            (
                "plain.yaml",
                "_forge_version: \"5.1.0\"\nscalars: {}\n",
                "Validation error: No 'bootstrap' section found in YAML",
            ),
            ("bad.yaml", "a: [1\n", "Validation error: YAML parse error: "),
            (
                "config.yaml",
                "bootstrap: 3\n",
                "Validation error: bootstrap config error: ",
            ),
            (
                "short.yaml",
                "bootstrap:\n  data: [1.0]\n",
                "Validation error: Data must have at least 2 observations",
            ),
            (
                "bca.yaml",
                "bootstrap:\n  iterations: 20\n  data: [2.0, 2.0]\n  method: bca\n",
                "Formula evaluation error: BCa interval undefined: every bootstrap value is on the same side of the estimate; use method percentile or basic",
            ),
        ] {
            let path = dir.path().join(name);
            std::fs::write(&path, text).unwrap();
            let err = run(&path, (None, None, None), None, false).1.unwrap_err();
            assert!(err.starts_with(want), "{name}: {err}");
            let core = bootstrap_core(&path, None, None, None)
                .unwrap_err()
                .to_string();
            assert_eq!(core, err, "{name}: bootstrap_core says the same");
        }
        let missing = dir.path().join("missing.yaml");
        assert!(run(&missing, (None, None, None), None, false).1.is_err());
    }

    /// `--output` must write a file other tools can read (YAML, or JSON for
    /// a `.json` path), not a Rust debug dump, and say where
    #[test]
    fn the_result_is_written_where_asked() {
        let dir = tempfile::tempdir().unwrap();
        let path = model(dir.path(), "flat.yaml", FLAT);
        let yaml_out = dir.path().join("out.yaml");
        let json_out = dir.path().join("out.json");
        for out in [&yaml_out, &json_out] {
            let (text, result) = run(&path, (None, None, None), Some(out.clone()), false);
            assert!(
                result.is_ok()
                    && text.contains(&format!("💾 Results written to {}\n", out.display())),
                "{text}"
            );
        }
        let doc: yaml_serde::Value =
            yaml_serde::from_str(&std::fs::read_to_string(&yaml_out).unwrap()).unwrap();
        assert_eq!(
            (
                doc["original_estimate"].as_f64(),
                doc["confidence_intervals"][1]["upper"].as_f64()
            ),
            (Some(2.0), Some(2.0))
        );
        let doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&json_out).unwrap()).unwrap();
        assert_eq!(doc["bias"].as_f64(), Some(0.0));
        let (_, result) = run(
            &path,
            (None, None, None),
            Some(dir.path().join("no-such-folder").join("r.json")),
            false,
        );
        assert!(result.is_err(), "{result:?}");
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
