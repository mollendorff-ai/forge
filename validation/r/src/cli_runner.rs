//! CLI Runner for forge analytics commands.
//!
//! Executes forge CLI commands and captures JSON output for validation against R.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

/// A forge CLI command to execute.
#[derive(Debug, Clone)]
pub struct ForgeCommand {
    /// Command name (e.g., "simulate", "tornado").
    pub cmd: String,
    /// Command arguments.
    pub args: Vec<String>,
    /// Path to the YAML fixture file.
    pub fixture: PathBuf,
}

/// Summary statistics from analytics output.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Stats {
    /// Mean value.
    pub mean: Option<f64>,
    /// Standard deviation.
    pub std: Option<f64>,
    /// Percentiles.
    pub percentiles: HashMap<String, f64>,
    /// Sample values (for KS test).
    #[serde(default)]
    pub samples: Vec<f64>,
}

/// Output from a forge analytics command.
#[derive(Debug, Clone)]
pub struct AnalyticsOutput {
    /// Raw JSON output from forge.
    pub raw_json: serde_json::Value,
    /// Parsed summary statistics.
    pub stats: Option<Stats>,
    /// Exit code.
    pub exit_code: i32,
    /// Stderr output.
    pub stderr: String,
}

/// Configuration for the CLI runner.
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Path to the forge binary.
    pub forge_bin: PathBuf,
    /// Timeout for command execution.
    pub timeout: Duration,
    /// Working directory.
    pub working_dir: Option<PathBuf>,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            forge_bin: PathBuf::from("forge"),
            timeout: Duration::from_secs(30),
            working_dir: None,
        }
    }
}

/// Runs a forge CLI command and captures the output.
///
/// # Errors
///
/// Returns an error if the forge process fails to spawn or produces invalid JSON.
pub fn run_forge_command(cmd: &ForgeCommand, config: &RunnerConfig) -> Result<AnalyticsOutput> {
    let mut command = Command::new(&config.forge_bin);
    command.arg(&cmd.cmd);
    command.arg(&cmd.fixture);
    command.args(&cmd.args);
    command.arg("--output");
    command.arg("json");

    if let Some(ref dir) = config.working_dir {
        command.current_dir(dir);
    }

    let output = execute_with_timeout(&mut command, config.timeout).with_context(|| {
        format!(
            "Failed to execute: forge {} {}",
            cmd.cmd,
            cmd.fixture.display()
        )
    })?;

    parse_forge_output(&output)
}

#[allow(clippy::cast_possible_truncation)]
fn execute_with_timeout(command: &mut Command, timeout: Duration) -> Result<Output> {
    use std::io::{BufRead, BufReader};
    use std::thread;

    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn forge process")?;

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
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(anyhow!("Command timed out after {timeout_ms}ms"));
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(anyhow!("Error waiting for process: {e}")),
        }
    }
}

fn parse_forge_output(output: &Output) -> Result<AnalyticsOutput> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    let raw_json: serde_json::Value = if stdout.trim().is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_str(&stdout).with_context(|| {
            format!(
                "Failed to parse JSON: {}",
                stdout.chars().take(200).collect::<String>()
            )
        })?
    };

    let stats = extract_stats(&raw_json);

    Ok(AnalyticsOutput {
        raw_json,
        stats,
        exit_code,
        stderr,
    })
}

fn extract_stats(json: &serde_json::Value) -> Option<Stats> {
    let mean = json.get("mean").and_then(serde_json::Value::as_f64);
    let std = json
        .get("std")
        .or_else(|| json.get("stddev"))
        .or_else(|| json.get("sd"))
        .and_then(serde_json::Value::as_f64);

    let percentiles: HashMap<String, f64> = json
        .get("percentiles")
        .and_then(|p| p.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_f64().map(|f| (k.clone(), f)))
                .collect()
        })
        .unwrap_or_default();

    let samples = json
        .get("samples")
        .and_then(|s| s.as_array())
        .map(|arr| arr.iter().filter_map(serde_json::Value::as_f64).collect())
        .unwrap_or_default();

    if mean.is_some() || std.is_some() || !percentiles.is_empty() {
        Some(Stats {
            mean,
            std,
            percentiles,
            samples,
        })
    } else {
        None
    }
}

/// Checks if forge is available.
///
/// # Errors
///
/// Returns an error if forge cannot be executed or returns non-zero.
pub fn check_forge_available(config: &RunnerConfig) -> Result<String> {
    let output = Command::new(&config.forge_bin)
        .arg("--version")
        .output()
        .context("Failed to run forge --version")?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(version)
    } else {
        Err(anyhow!(
            "forge returned non-zero: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// Finds the forge binary.
#[must_use]
pub fn find_forge_binary() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("FORGE_BIN") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    let relative = Path::new("../forge/target/release/forge");
    if relative.exists() {
        return Some(relative.to_path_buf());
    }

    if Command::new("forge").arg("--version").output().is_ok() {
        return Some(PathBuf::from("forge"));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_config_default() {
        let config = RunnerConfig::default();
        assert_eq!(config.forge_bin, PathBuf::from("forge"));
    }

    #[test]
    fn test_extract_stats() {
        let json = serde_json::json!({
            "mean": 100.5,
            "std": 15.2
        });
        let stats = extract_stats(&json).unwrap();
        assert!((stats.mean.unwrap() - 100.5).abs() < f64::EPSILON);
    }
}
