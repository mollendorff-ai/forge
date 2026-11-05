//! Category runners for the analytics specs (bootstrap, tornado, ...).
//!
//! Each runner turns a spec into a Forge model, runs the Forge command, runs
//! the matching R validator, and compares the two. Forge's answer always comes
//! from the real binary; R's answer comes from the validator script.

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::{json, Value as Json};

use crate::r_validator::{validate_with_json, RConfig};
use crate::stats::within_tolerance;
use crate::types::{AnalyticsTestSpec, TestResult};

/// What a runner needs to run a test.
pub struct Env<'a> {
    pub forge: &'a Path,
    pub r: &'a RConfig,
}

/// Runs the runner for the spec's validator, or `None` when the spec belongs
/// to another category.
#[must_use]
pub fn run(spec: &AnalyticsTestSpec, env: &Env) -> Option<TestResult> {
    match spec.r_validator.as_deref() {
        Some("bootstrap_validator.R") => Some(bootstrap(spec, env)),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn pass(spec: &AnalyticsTestSpec, details: String) -> TestResult {
    TestResult::Pass {
        name: spec.name.clone(),
        details,
    }
}

fn fail(spec: &AnalyticsTestSpec, reason: String) -> TestResult {
    TestResult::Fail {
        name: spec.name.clone(),
        reason,
    }
}

fn error(spec: &AnalyticsTestSpec, error: String) -> TestResult {
    TestResult::Error {
        name: spec.name.clone(),
        error,
    }
}

fn skip(spec: &AnalyticsTestSpec, reason: &str) -> TestResult {
    TestResult::Skip {
        name: spec.name.clone(),
        reason: reason.to_string(),
    }
}

/// Reads a number from a YAML value.
fn yaml_f64(value: &serde_yaml_ng::Value) -> Option<f64> {
    value.as_f64()
}

/// Reads a list of numbers from a YAML value.
fn yaml_f64_list(value: &serde_yaml_ng::Value) -> Option<Vec<f64>> {
    value.as_sequence()?.iter().map(yaml_f64).collect()
}

/// Tolerance from the spec, or a default.
fn tolerance(spec: &AnalyticsTestSpec, key: &str, default: f64) -> f64 {
    spec.raw
        .get("tolerance")
        .and_then(|t| t.get(key))
        .and_then(yaml_f64)
        .unwrap_or(default)
}

/// Runs a Forge command and returns an error with its output on failure.
fn run_forge(forge: &Path, args: &[&std::ffi::OsStr]) -> Result<(), String> {
    let output = Command::new(forge)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run forge: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "forge {} failed: {}{}",
            args.first()
                .map(|a| a.to_string_lossy())
                .unwrap_or_default(),
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        ))
    }
}

/// Compares two numbers with a relative tolerance and records a problem if they differ.
fn check(problems: &mut Vec<String>, label: &str, forge: f64, r: f64, tol: f64) {
    if !within_tolerance(forge, r, tol) {
        problems.push(format!(
            "{label}: forge={forge:.6}, R={r:.6}, tolerance={:.1}%",
            tol * 100.0
        ));
    }
}

fn json_f64(value: &Json, key: &str) -> Option<f64> {
    value.get(key).and_then(Json::as_f64)
}

// ─────────────────────────────────────────────────────────────────────────────
// Bootstrap
// ─────────────────────────────────────────────────────────────────────────────

fn bootstrap(spec: &AnalyticsTestSpec, env: &Env) -> TestResult {
    let method = spec
        .raw
        .get("method")
        .and_then(|m| m.as_str())
        .unwrap_or("percentile");
    if method != "percentile" {
        return skip(
            spec,
            &format!("Forge only computes percentile intervals, not {method}"),
        );
    }

    let Some(data) = spec.raw.get("data").and_then(yaml_f64_list) else {
        return error(spec, "Spec has no numeric 'data' list".to_string());
    };
    let statistic = spec
        .raw
        .get("statistic")
        .and_then(|s| s.as_str())
        .unwrap_or("mean");
    let levels = spec
        .raw
        .get("confidence_levels")
        .and_then(yaml_f64_list)
        .unwrap_or_else(|| vec![0.95]);

    let join = |values: &[f64]| {
        values
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let model = format!(
        "_forge_version: \"5.0.0\"\n\nbootstrap:\n  iterations: {}\n  seed: {}\n  confidence_levels: [{}]\n  data: [{}]\n  statistic: {}\n",
        spec.iterations,
        spec.seed,
        join(&levels),
        join(&data),
        statistic
    );

    let dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => return error(spec, format!("Failed to create temp dir: {e}")),
    };
    let model_path = dir.path().join("bootstrap.yaml");
    let out_path = dir.path().join("out.json");
    if let Err(e) = fs::write(&model_path, model) {
        return error(spec, format!("Failed to write model: {e}"));
    }
    if let Err(e) = run_forge(
        env.forge,
        &[
            "bootstrap".as_ref(),
            model_path.as_os_str(),
            "-o".as_ref(),
            out_path.as_os_str(),
        ],
    ) {
        return error(spec, e);
    }
    let forge: Json = match fs::read_to_string(&out_path)
        .map_err(|e| e.to_string())
        .and_then(|t| serde_json::from_str(&t).map_err(|e| e.to_string()))
    {
        Ok(j) => j,
        Err(e) => return error(spec, format!("Cannot read forge output: {e}")),
    };

    let r_params = json!({
        "data": data,
        "statistic": statistic,
        "method": method,
        "confidence_levels": levels,
        "seed": spec.seed,
        "iterations": spec.iterations,
    });
    let r = match validate_with_json("bootstrap_validator.R", &r_params, env.r) {
        Ok(r) if r.success => r.results.unwrap_or(Json::Null),
        Ok(r) => {
            return error(
                spec,
                format!("R returned error: {}", r.error.unwrap_or_default()),
            )
        }
        Err(e) => return error(spec, format!("R validator failed: {e}")),
    };

    compare_bootstrap(spec, &forge, &r, &levels)
}

fn compare_bootstrap(
    spec: &AnalyticsTestSpec,
    forge: &Json,
    r: &Json,
    levels: &[f64],
) -> TestResult {
    let mut problems = Vec::new();
    match (
        json_f64(forge, "original_estimate"),
        json_f64(r, "original_estimate"),
    ) {
        (Some(f), Some(r)) => check(&mut problems, "original_estimate", f, r, 1e-9),
        _ => problems.push("original_estimate missing".to_string()),
    }
    match (json_f64(forge, "bootstrap_mean"), json_f64(r, "mean")) {
        (Some(f), Some(r)) => check(&mut problems, "mean", f, r, tolerance(spec, "mean", 0.02)),
        _ => problems.push("mean missing".to_string()),
    }
    match (json_f64(forge, "bootstrap_std_error"), json_f64(r, "std")) {
        (Some(f), Some(r)) => check(&mut problems, "std", f, r, tolerance(spec, "std", 0.10)),
        _ => problems.push("std missing".to_string()),
    }

    // Interval bounds: the tolerance is a fraction of R's interval width, so a
    // bound near zero is not judged by a relative difference of tiny numbers.
    let ci_tol = tolerance(spec, "ci_bounds", 0.10);
    let forge_cis = forge
        .get("confidence_intervals")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();
    for level in levels {
        let r_ci = r
            .get("confidence_intervals")
            .and_then(|c| c.get(level.to_string()));
        let f_ci = forge_cis
            .iter()
            .find(|c| json_f64(c, "level").is_some_and(|l| (l - level).abs() < 1e-9));
        let (Some(r_ci), Some(f_ci)) = (r_ci, f_ci) else {
            problems.push(format!("interval at {level} missing"));
            continue;
        };
        let (Some(rl), Some(ru), Some(fl), Some(fu)) = (
            json_f64(r_ci, "lower"),
            json_f64(r_ci, "upper"),
            json_f64(f_ci, "lower"),
            json_f64(f_ci, "upper"),
        ) else {
            problems.push(format!("interval at {level} incomplete"));
            continue;
        };
        let allowed = ci_tol * (ru - rl).abs();
        for (label, f, rv) in [("lower", fl, rl), ("upper", fu, ru)] {
            if (f - rv).abs() > allowed {
                problems.push(format!(
                    "{level} {label}: forge={f:.6}, R={rv:.6}, allowed difference {allowed:.6}"
                ));
            }
        }
    }

    if problems.is_empty() {
        pass(
            spec,
            "Forge matches R's boot package within tolerance".to_string(),
        )
    } else {
        fail(spec, problems.join("; "))
    }
}
