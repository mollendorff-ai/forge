//! R Validator for analytics E2E tests.
//!
//! Calls R validation scripts and compares results with forge output.

#![allow(clippy::similar_names)]

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::Duration;

use crate::cli_runner::{AnalyticsOutput, Stats};
use crate::stats::Tolerance;

/// Parameters to pass to an R validator.
#[derive(Debug, Clone, Serialize)]
pub struct RParams {
    /// Distribution type.
    pub distribution: Option<String>,
    /// Numeric parameters.
    pub params: HashMap<String, f64>,
    /// Random seed.
    pub seed: u64,
    /// Number of iterations.
    pub iterations: usize,
}

impl Default for RParams {
    fn default() -> Self {
        Self {
            distribution: None,
            params: HashMap::new(),
            seed: 42,
            iterations: 10_000,
        }
    }
}

/// Result from an R validator script.
#[derive(Debug, Clone, Deserialize)]
pub struct RResult {
    /// Validator name.
    pub validator: String,
    /// Validator version.
    #[serde(default)]
    pub version: String,
    /// Whether validation succeeded.
    pub success: bool,
    /// Results (if successful).
    pub results: Option<serde_json::Value>,
    /// Error message (if failed).
    pub error: Option<String>,
}

/// Validation result comparing forge and R outputs.
#[derive(Debug, Clone)]
pub enum ValidationResult {
    /// Results match within tolerance.
    Pass {
        forge_stats: Stats,
        r_stats: Stats,
        details: String,
    },
    /// Results differ beyond tolerance.
    Fail {
        forge_stats: Stats,
        r_stats: Stats,
        reason: String,
    },
    /// Validation could not be performed.
    Error { reason: String },
}

impl ValidationResult {
    #[must_use]
    pub const fn is_pass(&self) -> bool {
        matches!(self, Self::Pass { .. })
    }
}

/// Configuration for the R validator.
#[derive(Debug, Clone)]
pub struct RConfig {
    /// Path to Rscript binary.
    pub rscript_bin: PathBuf,
    /// Directory containing R validator scripts.
    pub validators_dir: PathBuf,
    /// Timeout for R script execution.
    pub timeout: Duration,
}

impl Default for RConfig {
    fn default() -> Self {
        Self {
            rscript_bin: PathBuf::from("Rscript"),
            validators_dir: PathBuf::from("validators/r"),
            timeout: Duration::from_secs(30),
        }
    }
}

/// Runs an R validator script with the given parameters.
///
/// # Errors
///
/// Returns an error if the R script cannot be found, fails to execute, or produces invalid JSON.
pub fn validate_with_r(validator: &str, params: &RParams, config: &RConfig) -> Result<RResult> {
    validate_with_json(validator, &serde_json::to_value(params)?, config)
}

/// Runs an R validator script with arbitrary JSON parameters.
///
/// # Errors
///
/// Returns an error if the R script cannot be found, fails to execute, or produces invalid JSON.
pub fn validate_with_json(
    validator: &str,
    params: &serde_json::Value,
    config: &RConfig,
) -> Result<RResult> {
    let script_path = config.validators_dir.join(validator);
    if !script_path.exists() {
        return Err(anyhow!("R validator not found: {}", script_path.display()));
    }

    let params_json = serde_json::to_string(params)?;

    let mut command = Command::new(&config.rscript_bin);
    command.arg(&script_path);
    command.arg("--json");
    command.arg(&params_json);

    let output = execute_with_timeout(&mut command, config.timeout)
        .with_context(|| format!("Failed to execute R validator: {validator}"))?;

    parse_r_output(&output, validator)
}

/// Compares forge output with R validation result.
#[must_use]
pub fn compare_results(
    forge: &AnalyticsOutput,
    r_result: &RResult,
    tolerance: &Tolerance,
) -> ValidationResult {
    if !r_result.success {
        return ValidationResult::Error {
            reason: r_result
                .error
                .clone()
                .unwrap_or_else(|| "Unknown R error".to_string()),
        };
    }

    let Some(forge_stats) = forge.stats.clone() else {
        return ValidationResult::Error {
            reason: "No statistics in forge output".to_string(),
        };
    };

    let Some(r_stats) = parse_r_stats(r_result.results.as_ref()) else {
        return ValidationResult::Error {
            reason: "No statistics in R output".to_string(),
        };
    };

    // Compare mean
    if let (Some(forge_mean), Some(r_mean)) = (forge_stats.mean, r_stats.mean) {
        if !crate::stats::within_tolerance(forge_mean, r_mean, tolerance.mean) {
            return ValidationResult::Fail {
                forge_stats,
                r_stats,
                reason: format!(
                    "Mean mismatch: forge={:.6}, R={:.6}, tolerance={:.2}%",
                    forge_mean,
                    r_mean,
                    tolerance.mean * 100.0
                ),
            };
        }
    }

    // Compare std
    if let (Some(forge_std), Some(r_std)) = (forge_stats.std, r_stats.std) {
        if !crate::stats::within_tolerance(forge_std, r_std, tolerance.std) {
            return ValidationResult::Fail {
                forge_stats,
                r_stats,
                reason: format!(
                    "Std mismatch: forge={:.6}, R={:.6}, tolerance={:.2}%",
                    forge_std,
                    r_std,
                    tolerance.std * 100.0
                ),
            };
        }
    }

    // Compare percentiles
    let percentile_mismatch = r_stats.percentiles.iter().find_map(|(pct, r_val)| {
        forge_stats.percentiles.get(pct).and_then(|forge_val| {
            if crate::stats::within_tolerance(*forge_val, *r_val, tolerance.percentiles) {
                None
            } else {
                Some(format!(
                    "Percentile {pct} mismatch: forge={forge_val:.6}, R={r_val:.6}"
                ))
            }
        })
    });

    if let Some(reason) = percentile_mismatch {
        return ValidationResult::Fail {
            forge_stats,
            r_stats,
            reason,
        };
    }

    ValidationResult::Pass {
        forge_stats,
        r_stats,
        details: "All statistics within tolerance".to_string(),
    }
}

#[allow(clippy::cast_possible_truncation)]
fn execute_with_timeout(command: &mut Command, timeout: Duration) -> Result<Output> {
    use std::io::{BufRead, BufReader};
    use std::thread;

    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn Rscript process")?;

    let timeout_ms = timeout.as_millis() as u64;
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = child.stdout.take().map_or_else(Vec::new, |s| {
                    BufReader::new(s)
                        .lines()
                        .map_while(Result::ok)
                        .collect::<Vec<_>>()
                        .join("\n")
                        .into_bytes()
                });
                let stderr = child.stderr.take().map_or_else(Vec::new, |s| {
                    BufReader::new(s)
                        .lines()
                        .map_while(Result::ok)
                        .collect::<Vec<_>>()
                        .join("\n")
                        .into_bytes()
                });
                return Ok(Output {
                    status,
                    stdout,
                    stderr,
                });
            },
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(anyhow!("R script timed out after {timeout_ms}ms"));
                }
                thread::sleep(Duration::from_millis(10));
            },
            Err(e) => return Err(anyhow!("Error waiting for R process: {e}")),
        }
    }
}

fn parse_r_output(output: &Output, validator: &str) -> Result<RResult> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Ok(RResult {
            validator: validator.to_string(),
            version: String::new(),
            success: false,
            results: None,
            error: Some(format!(
                "R script failed (exit {}): {}",
                output.status.code().unwrap_or(-1),
                stderr
            )),
        });
    }

    serde_json::from_str(&stdout).with_context(|| {
        format!(
            "Failed to parse R validator JSON: {}",
            stdout.chars().take(200).collect::<String>()
        )
    })
}

fn parse_r_stats(results: Option<&serde_json::Value>) -> Option<Stats> {
    let results = results?;

    let mean = results.get("mean").and_then(serde_json::Value::as_f64);
    let std = results
        .get("std")
        .or_else(|| results.get("sd"))
        .and_then(serde_json::Value::as_f64);

    let percentiles = results
        .get("percentiles")
        .and_then(|p| p.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_f64().map(|f| (k.clone(), f)))
                .collect()
        })
        .unwrap_or_default();

    let samples = results
        .get("samples")
        .and_then(|s| s.as_array())
        .map(|arr| arr.iter().filter_map(serde_json::Value::as_f64).collect())
        .unwrap_or_default();

    Some(Stats {
        mean,
        std,
        percentiles,
        samples,
    })
}

/// Checks if R is available on the system.
///
/// # Errors
///
/// Returns an error if Rscript cannot be executed.
pub fn check_r_available(config: &RConfig) -> Result<String> {
    let output = Command::new(&config.rscript_bin)
        .arg("--version")
        .output()
        .context("Failed to run Rscript --version")?;

    let version = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if version.contains("version") || output.status.success() {
        Ok(version)
    } else {
        Err(anyhow!("Rscript not available"))
    }
}

/// Checks if a required R package is installed.
///
/// # Errors
///
/// Returns an error if the package check command fails to execute.
pub fn check_r_package(package: &str, config: &RConfig) -> Result<bool> {
    let script = format!("cat(requireNamespace('{package}', quietly=TRUE))");

    let output = Command::new(&config.rscript_bin)
        .arg("-e")
        .arg(&script)
        .output()
        .with_context(|| format!("Failed to check R package: {package}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim() == "TRUE")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_r_params_default() {
        let params = RParams::default();
        assert_eq!(params.seed, 42);
        assert_eq!(params.iterations, 10_000);
    }

    #[test]
    fn test_r_config_default() {
        let config = RConfig::default();
        assert_eq!(config.rscript_bin, PathBuf::from("Rscript"));
    }

    #[test]
    fn test_validation_result_is_pass() {
        let result = ValidationResult::Pass {
            forge_stats: Stats::default(),
            r_stats: Stats::default(),
            details: "OK".to_string(),
        };
        assert!(result.is_pass());
    }
}
