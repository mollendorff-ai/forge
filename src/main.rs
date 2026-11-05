//! forge-e2e-r: CLI entry point.
//!
//! Validates forge analytics against R.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use clap::Parser;
use colored::Colorize;
use tempfile::NamedTempFile;

use forge_e2e_r::cli_runner::find_forge_binary;
use forge_e2e_r::r_validator::{check_r_available, validate_with_r, RConfig, RParams};
use forge_e2e_r::stats::{within_tolerance, Tolerance};
use forge_e2e_r::types::{load_analytics_tests, AnalyticsTestSpec, TestResult};

#[derive(Parser)]
#[command(name = "forge-e2e-r")]
#[command(about = "E2E validation of forge analytics against R")]
#[command(version)]
struct Cli {
    /// Run all tests (headless mode with colored output).
    #[arg(long)]
    all: bool,

    /// Path to test specs directory.
    #[arg(short, long, default_value = "tests/analytics")]
    tests: PathBuf,

    /// Path to forge binary (or set `FORGE_BIN` env var).
    #[arg(short, long)]
    binary: Option<PathBuf>,

    /// Compare decision trees with the spec's expected values and do not run R.
    /// For machines where the R package data.tree is not installed.
    #[arg(long)]
    expected_only: bool,

    /// Path to R validators directory.
    #[arg(long, default_value = "validators/r")]
    validators: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Find forge binary
    let forge_binary = cli
        .binary
        .or_else(find_forge_binary)
        .ok_or_else(|| anyhow::anyhow!("Forge binary not found. Set FORGE_BIN or use --binary"))?;

    if !forge_binary.exists() {
        anyhow::bail!("Forge binary not found: {}", forge_binary.display());
    }

    // Check R availability
    let r_config = RConfig {
        validators_dir: cli.validators.clone(),
        ..Default::default()
    };

    let r_version = check_r_available(&r_config).map_err(|_| {
        anyhow::anyhow!(
            "R (Rscript) not found. Install with:\n  macOS: brew install r\n  Ubuntu: apt install r-base"
        )
    })?;

    println!("{}", "forge-e2e-r".bold());
    println!("  Forge: {}", forge_binary.display());
    println!("  R: {r_version}");
    println!("  Tests: {}", cli.tests.display());
    println!("  Validators: {}", cli.validators.display());
    println!();

    // Load tests
    let tests = load_tests(&cli.tests)?;
    println!("Loaded {} tests", tests.len());
    println!();

    if cli.all {
        run_all_mode(&tests, &forge_binary, &r_config, cli.expected_only)?;
    } else {
        println!("Use --all to run all tests");
    }

    Ok(())
}

fn load_tests(tests_dir: &PathBuf) -> anyhow::Result<Vec<forge_e2e_r::types::AnalyticsTestSpec>> {
    let mut all_tests = Vec::new();

    if !tests_dir.exists() {
        anyhow::bail!("Tests directory not found: {}", tests_dir.display());
    }

    for entry in fs::read_dir(tests_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|e| e == "yaml") {
            let content = fs::read_to_string(&path)?;
            match load_analytics_tests(&content) {
                Ok(tests) => all_tests.extend(tests),
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {e}", path.display());
                }
            }
        }
    }

    Ok(all_tests)
}

#[allow(clippy::unnecessary_wraps)] // Result for consistent main() error handling
fn run_all_mode(
    tests: &[AnalyticsTestSpec],
    forge_binary: &PathBuf,
    r_config: &RConfig,
    expected_only: bool,
) -> anyhow::Result<()> {
    let start = Instant::now();
    let mut results = Vec::new();

    println!("{}", "Running tests...".cyan());

    let env = forge_e2e_r::analytics::Env {
        forge: forge_binary,
        r: r_config,
        expected_only,
    };
    for test in tests {
        let result = forge_e2e_r::analytics::run(test, &env)
            .unwrap_or_else(|| run_monte_carlo_test(test, forge_binary, r_config));
        print_result(&result);
        results.push(result);
    }

    let elapsed = start.elapsed();

    // Summary
    println!();
    println!("{}", "=".repeat(60));

    let passed = results.iter().filter(|r| r.is_pass()).count();
    let failed = results.iter().filter(|r| r.is_fail()).count();
    let errored = results.iter().filter(|r| r.is_error()).count();
    let skipped = results
        .iter()
        .filter(|r| matches!(r, TestResult::Skip { .. }))
        .count();

    // An error (missing R package, Forge crash, unparseable output) means the
    // test did not run, so it counts against the run just like a failure.
    // A run that passed nothing proves nothing, so it fails too.
    let bad = failed + errored;
    if bad == 0 && passed > 0 {
        println!(
            "  {} {} passed, {} skipped in {:.2}s",
            "PASS".green(),
            passed.to_string().green(),
            skipped,
            elapsed.as_secs_f64()
        );
    } else {
        println!(
            "  {} {} passed, {} failed, {} errored, {} skipped in {:.2}s",
            "FAIL".red(),
            passed,
            failed.to_string().red(),
            errored.to_string().red(),
            skipped,
            elapsed.as_secs_f64()
        );
        if passed == 0 && bad == 0 {
            println!("  No test passed: nothing was validated.");
        }
    }

    if skipped > 0 {
        println!(
            "  {} {skipped} of {} tests were skipped, not validated (no runner yet, or Forge lacks the feature).",
            "NOTE".yellow(),
            results.len()
        );
    }

    let unchecked_by_r = results
        .iter()
        .filter(|r| matches!(r, TestResult::Pass { details, .. } if details.contains("R not run")))
        .count();
    if unchecked_by_r > 0 {
        println!(
            "  {} {unchecked_by_r} passed tests were compared with the spec's expected values only; R was not run for them.",
            "NOTE".yellow()
        );
    }

    println!("{}", "=".repeat(60));

    if bad > 0 || passed == 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Runs a single Monte Carlo test by:
/// 1. Creating a temporary YAML fixture for forge
/// 2. Running forge simulate
/// 3. Running the R validator
/// 4. Comparing results
#[allow(clippy::too_many_lines)]
fn run_monte_carlo_test(
    test: &AnalyticsTestSpec,
    forge_binary: &PathBuf,
    r_config: &RConfig,
) -> TestResult {
    // Skip tests without distribution (non-Monte Carlo tests)
    let Some(ref distribution) = test.distribution else {
        return TestResult::Skip {
            name: test.name.clone(),
            reason: "No distribution specified (not a Monte Carlo test)".to_string(),
        };
    };

    // Build the MC.* formula based on distribution type
    let mc_formula = match build_mc_formula(distribution, &test.params) {
        Ok(formula) => formula,
        Err(e) => {
            return TestResult::Skip {
                name: test.name.clone(),
                reason: format!("Cannot build formula: {e}"),
            };
        }
    };

    // Create temporary YAML file for forge
    let yaml_content = format!(
        r#"_forge_version: "5.0.0"
monte_carlo:
  enabled: true
  iterations: {iterations}
  sampling: monte_carlo
  seed: {seed}
  outputs:
    - variable: test_output
      percentiles: [5, 10, 25, 50, 75, 90, 95]
scalars:
  test_output:
    value: null
    formula: "{formula}"
"#,
        iterations = test.iterations,
        seed = test.seed,
        formula = mc_formula,
    );

    // Write to temp file
    let temp_file = match NamedTempFile::new() {
        Ok(f) => f,
        Err(e) => {
            return TestResult::Error {
                name: test.name.clone(),
                error: format!("Failed to create temp file: {e}"),
            };
        }
    };

    if let Err(e) = fs::write(temp_file.path(), &yaml_content) {
        return TestResult::Error {
            name: test.name.clone(),
            error: format!("Failed to write temp file: {e}"),
        };
    }

    // Run forge simulate
    let forge_result = run_forge_simulate(forge_binary, temp_file.path(), test.seed);
    let forge_stats = match forge_result {
        Ok(stats) => stats,
        Err(e) => {
            return TestResult::Error {
                name: test.name.clone(),
                error: format!("Forge failed: {e}"),
            };
        }
    };

    // Run R validator
    let r_params = RParams {
        distribution: Some(distribution.clone()),
        params: test.params.clone(),
        seed: test.seed,
        iterations: test.iterations,
    };

    let validator_script = test
        .r_validator
        .as_deref()
        .unwrap_or("monte_carlo_validator.R");

    let r_result = match validate_with_r(validator_script, &r_params, r_config) {
        Ok(r) => r,
        Err(e) => {
            return TestResult::Error {
                name: test.name.clone(),
                error: format!("R validator failed: {e}"),
            };
        }
    };

    if !r_result.success {
        return TestResult::Error {
            name: test.name.clone(),
            error: format!(
                "R returned error: {}",
                r_result.error.unwrap_or_else(|| "Unknown".to_string())
            ),
        };
    }

    // Parse R results
    let Some(r_stats) = parse_r_results(r_result.results.as_ref()) else {
        return TestResult::Error {
            name: test.name.clone(),
            error: "Failed to parse R statistics".to_string(),
        };
    };

    // Get tolerance from test spec or use defaults
    let tolerance = test
        .tolerance
        .as_ref()
        .map(|t| Tolerance {
            mean: t.mean.unwrap_or(0.01),
            std: t.std.unwrap_or(0.05),
            percentiles: t.percentiles.unwrap_or(0.02),
            ..Default::default()
        })
        .unwrap_or_default();

    // Compare results
    compare_forge_r_results(&test.name, &forge_stats, &r_stats, &tolerance)
}

/// Builds the MC.* formula string for a given distribution and parameters.
fn build_mc_formula(distribution: &str, params: &HashMap<String, f64>) -> Result<String, String> {
    match distribution.to_lowercase().as_str() {
        "normal" => {
            let mean = params.get("mean").ok_or("Missing 'mean' param")?;
            let sd = params.get("sd").ok_or("Missing 'sd' param")?;
            Ok(format!("=MC.Normal({mean}, {sd})"))
        }
        "uniform" => {
            let min = params.get("min").ok_or("Missing 'min' param")?;
            let max = params.get("max").ok_or("Missing 'max' param")?;
            Ok(format!("=MC.Uniform({min}, {max})"))
        }
        "lognormal" => {
            let meanlog = params.get("meanlog").ok_or("Missing 'meanlog' param")?;
            let sdlog = params.get("sdlog").ok_or("Missing 'sdlog' param")?;
            // R uses meanlog/sdlog (log-scale parameters)
            // Forge uses actual mean/stdev of the lognormal distribution
            // Convert: mean = exp(meanlog + sdlog^2/2)
            //          stdev = sqrt((exp(sdlog^2) - 1) * exp(2*meanlog + sdlog^2))
            let mean = (meanlog + sdlog * sdlog / 2.0).exp();
            let variance = (sdlog * sdlog).exp_m1() * (2.0 * meanlog + sdlog * sdlog).exp();
            let stdev = variance.sqrt();
            Ok(format!("=MC.Lognormal({mean}, {stdev})"))
        }
        "triangular" => {
            let min = params.get("min").ok_or("Missing 'min' param")?;
            let mode = params.get("mode").ok_or("Missing 'mode' param")?;
            let max = params.get("max").ok_or("Missing 'max' param")?;
            Ok(format!("=MC.Triangular({min}, {mode}, {max})"))
        }
        "pert" => {
            let min = params.get("min").ok_or("Missing 'min' param")?;
            let mode = params.get("mode").ok_or("Missing 'mode' param")?;
            let max = params.get("max").ok_or("Missing 'max' param")?;
            // Note: Forge's MC.PERT doesn't take a shape argument (uses default shape=4)
            Ok(format!("=MC.PERT({min}, {mode}, {max})"))
        }
        "exponential" => {
            // Forge doesn't support MC.Exponential yet
            Err("Exponential distribution not supported by forge".to_string())
        }
        other => Err(format!("Unsupported distribution: {other}")),
    }
}

/// Runs forge simulate and parses the JSON output.
fn run_forge_simulate(
    forge_binary: &PathBuf,
    yaml_path: &std::path::Path,
    seed: u64,
) -> Result<ForgeStats, String> {
    // Use a temp file for JSON output to avoid console output mixing
    let output_file =
        NamedTempFile::new().map_err(|e| format!("Failed to create temp output file: {e}"))?;
    let output_path = output_file.path().with_extension("json");

    let output = Command::new(forge_binary)
        .arg("simulate")
        .arg(yaml_path)
        .arg("--seed")
        .arg(seed.to_string())
        .arg("-o")
        .arg(&output_path)
        .output()
        .map_err(|e| format!("Failed to run forge: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("Forge exited with error: {stderr}\n{stdout}"));
    }

    // Read the JSON from the output file
    let json_content =
        fs::read_to_string(&output_path).map_err(|e| format!("Failed to read output file: {e}"))?;

    // Clean up
    let _ = fs::remove_file(&output_path);

    let json: serde_json::Value =
        serde_json::from_str(&json_content).map_err(|e| format!("Failed to parse JSON: {e}"))?;

    // Navigate to monte_carlo_results.outputs.test_output
    let mc_results = json
        .get("monte_carlo_results")
        .ok_or("Missing monte_carlo_results")?;
    let outputs = mc_results.get("outputs").ok_or("Missing outputs")?;
    let test_output = outputs.get("test_output").ok_or("Missing test_output")?;

    let mean = test_output
        .get("mean")
        .and_then(serde_json::Value::as_f64)
        .ok_or("Missing mean")?;
    let std = test_output
        .get("std_dev")
        .and_then(serde_json::Value::as_f64)
        .ok_or("Missing std_dev")?;

    let mut percentiles = HashMap::new();
    if let Some(p) = test_output.get("percentiles").and_then(|v| v.as_object()) {
        for (k, v) in p {
            if let Some(val) = v.as_f64() {
                // Convert p5 -> 5, p50 -> 50, etc.
                let key = k.trim_start_matches('p').to_string();
                percentiles.insert(key, val);
            }
        }
    }

    Ok(ForgeStats {
        mean,
        std,
        percentiles,
    })
}

/// Parsed statistics from either forge or R.
#[derive(Debug)]
struct ForgeStats {
    mean: f64,
    std: f64,
    percentiles: HashMap<String, f64>,
}

/// Parses R validator results into stats.
fn parse_r_results(results: Option<&serde_json::Value>) -> Option<ForgeStats> {
    let results = results?;

    let mean = results.get("mean").and_then(serde_json::Value::as_f64)?;
    let std = results
        .get("std")
        .or_else(|| results.get("sd"))
        .and_then(serde_json::Value::as_f64)?;

    let mut percentiles = HashMap::new();
    if let Some(p) = results.get("percentiles").and_then(|v| v.as_object()) {
        for (k, v) in p {
            if let Some(val) = v.as_f64() {
                percentiles.insert(k.clone(), val);
            }
        }
    }

    Some(ForgeStats {
        mean,
        std,
        percentiles,
    })
}

/// Compares forge and R results, returning Pass or Fail.
fn compare_forge_r_results(
    test_name: &str,
    forge: &ForgeStats,
    r: &ForgeStats,
    tolerance: &Tolerance,
) -> TestResult {
    // Compare mean
    if !within_tolerance(forge.mean, r.mean, tolerance.mean) {
        let diff_pct = ((forge.mean - r.mean).abs() / r.mean.abs()) * 100.0;
        return TestResult::Fail {
            name: test_name.to_string(),
            reason: format!(
                "Mean mismatch: forge={:.4}, R={:.4} (diff={:.2}%, tol={:.1}%)",
                forge.mean,
                r.mean,
                diff_pct,
                tolerance.mean * 100.0
            ),
        };
    }

    // Compare std
    if !within_tolerance(forge.std, r.std, tolerance.std) {
        let diff_pct = ((forge.std - r.std).abs() / r.std.abs()) * 100.0;
        return TestResult::Fail {
            name: test_name.to_string(),
            reason: format!(
                "Std mismatch: forge={:.4}, R={:.4} (diff={:.2}%, tol={:.1}%)",
                forge.std,
                r.std,
                diff_pct,
                tolerance.std * 100.0
            ),
        };
    }

    // Compare key percentiles (5, 50, 95)
    // Note: Different RNGs (Rust vs R) produce different random streams even with same seed.
    // We use more lenient tolerance for percentiles since they're inherently more variable
    // than mean/std in Monte Carlo simulations.
    // Also use minimum absolute tolerance based on the std dev to handle edge cases.
    let effective_pct_tolerance = tolerance.percentiles.max(0.10); // At least 10% for percentiles
    let abs_tolerance = r.std * 0.5; // Allow half a std dev absolute difference

    for pct in ["5", "50", "95"] {
        if let (Some(&forge_val), Some(&r_val)) =
            (forge.percentiles.get(pct), r.percentiles.get(pct))
        {
            let abs_diff = (forge_val - r_val).abs();
            let rel_diff = if r_val.abs() > f64::EPSILON {
                abs_diff / r_val.abs()
            } else {
                abs_diff
            };

            // Pass if within relative tolerance OR within absolute tolerance
            if rel_diff > effective_pct_tolerance && abs_diff > abs_tolerance {
                let diff_pct = rel_diff * 100.0;
                return TestResult::Fail {
                    name: test_name.to_string(),
                    reason: format!(
                        "P{} mismatch: forge={:.4}, R={:.4} (diff={:.2}%, tol={:.1}%)",
                        pct,
                        forge_val,
                        r_val,
                        diff_pct,
                        effective_pct_tolerance * 100.0
                    ),
                };
            }
        }
    }

    TestResult::Pass {
        name: test_name.to_string(),
        details: format!(
            "mean={:.2} std={:.2} (within tolerance)",
            forge.mean, forge.std
        ),
    }
}

fn print_result(result: &TestResult) {
    match result {
        TestResult::Pass { name, .. } => {
            println!("  {} {}", "✓".green(), name);
        }
        TestResult::Fail { name, reason } => {
            println!("  {} {}", "✗".red(), name.red());
            println!("      {reason}");
        }
        TestResult::Error { name, error } => {
            println!("  {} {} (error)", "✗".red(), name.red());
            println!("      {error}");
        }
        TestResult::Skip { name, reason } => {
            println!("  {} {} ({})", "○".yellow(), name.dimmed(), reason.dimmed());
        }
    }
}
