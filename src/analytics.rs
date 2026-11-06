//! Category runners for the analytics specs (bootstrap, tornado, ...).
//!
//! Each runner turns a spec into a Forge model, runs the Forge command, runs
//! the matching R validator, and compares the two. Forge's answer always comes
//! from the real binary; R's answer comes from the validator script.

// The runners are long straight-line sequences (build model, run, compare); splitting
// them further would only scatter that sequence.
#![allow(clippy::too_many_lines, clippy::format_push_string)]

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
    /// Compare Forge with the spec's `r_expected` values and do not run R.
    /// The decision tree, real options and Bayesian runners use this, for
    /// machines without the R packages data.tree, derivmkts and bnlearn.
    pub expected_only: bool,
}

/// Runs the runner for the spec's validator, or `None` when the spec belongs
/// to another category.
#[must_use]
pub fn run(spec: &AnalyticsTestSpec, env: &Env) -> Option<TestResult> {
    match spec.r_validator.as_deref() {
        Some("bootstrap_validator.R") => Some(bootstrap(spec, env)),
        Some("tornado_validator.R") => Some(tornado(spec, env)),
        Some("sensitivity_validator.R") => Some(sensitivity(spec, env)),
        Some("decision_tree_validator.R") => Some(decision_tree(spec, env)),
        Some("real_options_validator.R") => Some(real_options(spec, env)),
        Some("bayesian_validator.R") => Some(bayesian(spec, env)),
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
fn yaml_f64(value: &yaml_serde::Value) -> Option<f64> {
    value.as_f64()
}

/// Reads a list of numbers from a YAML value.
fn yaml_f64_list(value: &yaml_serde::Value) -> Option<Vec<f64>> {
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
        "_forge_version: \"5.0.0\"\n\nbootstrap:\n  iterations: {}\n  seed: {}\n  confidence_levels: [{}]\n  data: [{}]\n  statistic: {}\n  method: {}\n",
        spec.iterations,
        spec.seed,
        join(&levels),
        join(&data),
        statistic,
        method
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

// ─────────────────────────────────────────────────────────────────────────────
// Shared plumbing for model-based categories
// ─────────────────────────────────────────────────────────────────────────────

/// Runs `forge <command> model.yaml -o out.json` on a model and returns the JSON.
fn forge_json(env: &Env, command: &str, model: &str) -> Result<Json, String> {
    forge_json_with(env, command, model, &[])
}

/// Like `forge_json`, with extra command-line arguments after the model path.
/// Some commands write YAML to the output file, so YAML is accepted too.
fn forge_json_with(
    env: &Env,
    command: &str,
    model: &str,
    extra: &[String],
) -> Result<Json, String> {
    let dir = tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {e}"))?;
    let model_path = dir.path().join("model.yaml");
    let out_path = dir.path().join("out.json");
    fs::write(&model_path, model).map_err(|e| format!("Failed to write model: {e}"))?;
    let mut args: Vec<&std::ffi::OsStr> = vec![
        command.as_ref(),
        model_path.as_os_str(),
        "-o".as_ref(),
        out_path.as_os_str(),
    ];
    args.extend(extra.iter().map(|a| a.as_ref() as &std::ffi::OsStr));
    run_forge(env.forge, &args)?;
    let text =
        fs::read_to_string(&out_path).map_err(|e| format!("Cannot read forge output: {e}"))?;
    serde_json::from_str(&text)
        .or_else(|_| yaml_serde::from_str::<Json>(&text))
        .map_err(|e| format!("Cannot parse forge output: {e}"))
}

/// Runs an R validator and returns its `results` object.
fn r_results(env: &Env, validator: &str, params: &Json) -> Result<Json, String> {
    match validate_with_json(validator, params, env.r) {
        Ok(r) if r.success => Ok(r.results.unwrap_or(Json::Null)),
        Ok(r) => Err(format!("R returned error: {}", r.error.unwrap_or_default())),
        Err(e) => Err(format!("R validator failed: {e}")),
    }
}

/// Converts a YAML value from a spec to JSON.
fn to_json(value: &yaml_serde::Value) -> Json {
    serde_json::to_value(value).unwrap_or(Json::Null)
}

/// Turns a list of problems into a pass or fail.
fn verdict(spec: &AnalyticsTestSpec, problems: &[String], ok: &str) -> TestResult {
    if problems.is_empty() {
        pass(spec, ok.to_string())
    } else {
        fail(spec, problems.join("; "))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tornado
// ─────────────────────────────────────────────────────────────────────────────

fn tornado(spec: &AnalyticsTestSpec, env: &Env) -> TestResult {
    let model_kind = spec
        .raw
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("linear");
    let Some(variables) = spec.raw.get("variables").and_then(|v| v.as_sequence()) else {
        return error(spec, "Spec has no 'variables' list".to_string());
    };
    let coefficients = spec.raw.get("coefficients");
    let has_flat_range = variables
        .iter()
        .any(|v| v.get("low").and_then(yaml_f64) == v.get("high").and_then(yaml_f64));
    if has_flat_range {
        return skip(spec, "Forge rejects an input whose low equals its high");
    }
    let coef = |name: &str| coefficients.and_then(|c| c.get(name)).and_then(yaml_f64);

    let mut assumptions = String::new();
    let mut inputs = String::new();
    let mut terms = Vec::new();
    for v in variables {
        let (Some(name), Some(low), Some(high), Some(base)) = (
            v.get("name").and_then(|n| n.as_str()),
            v.get("low").and_then(yaml_f64),
            v.get("high").and_then(yaml_f64),
            v.get("base").and_then(yaml_f64),
        ) else {
            return error(spec, "Variable needs name, low, high and base".to_string());
        };
        assumptions.push_str(&format!(
            "  {name}:\n    value: {base}\n    formula: null\n"
        ));
        inputs.push_str(&format!(
            "    - name: {name}\n      low: {low}\n      high: {high}\n"
        ));
        if let Some(c) = coef(name) {
            terms.push(if model_kind == "multiplicative" {
                format!("(assumptions.{name} ^ ({c}))")
            } else {
                format!("(assumptions.{name} * ({c}))")
            });
        }
    }
    let joiner = if model_kind == "multiplicative" {
        " * "
    } else {
        " + "
    };
    let model = format!(
        "_forge_version: \"5.0.0\"\n\ntornado:\n  output: result.total\n  inputs:\n{inputs}\nassumptions:\n{assumptions}\nresult:\n  total:\n    value: null\n    formula: \"={}\"\n",
        terms.join(joiner)
    );

    let forge = match forge_json(env, "tornado", &model) {
        Ok(j) => j,
        Err(e) => return error(spec, e),
    };
    let r_params = json!({
        "base_value": spec.raw.get("base_value").map_or(Json::Null, to_json),
        "variables": variables.iter().map(to_json).collect::<Vec<_>>(),
        "model": model_kind,
        "coefficients": coefficients.map_or(Json::Null, to_json),
    });
    let r = match r_results(env, "tornado_validator.R", &r_params) {
        Ok(r) => r,
        Err(e) => return error(spec, e),
    };

    let tol = tolerance(spec, "swing", 0.001);
    let mut problems = Vec::new();
    let bars = forge
        .get("bars")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();
    let r_vars = r
        .get("variables")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();
    let swing_of = |bars: &[Json], name: &str| {
        bars.iter()
            .find(|b| {
                b.get("input_name")
                    .and_then(Json::as_str)
                    .is_some_and(|n| n.rsplit('.').next() == Some(name))
            })
            .cloned()
    };
    for rv in &r_vars {
        let Some(name) = rv.get("name").and_then(Json::as_str) else {
            continue;
        };
        let Some(bar) = swing_of(&bars, name) else {
            problems.push(format!("{name}: missing from forge output"));
            continue;
        };
        for (label, f, rr) in [
            ("swing", json_f64(&bar, "abs_swing"), json_f64(rv, "swing")),
            (
                "output_at_low",
                json_f64(&bar, "output_at_low"),
                json_f64(rv, "output_at_low"),
            ),
            (
                "output_at_high",
                json_f64(&bar, "output_at_high"),
                json_f64(rv, "output_at_high"),
            ),
        ] {
            match (f, rr) {
                (Some(f), Some(rr)) => check(&mut problems, &format!("{name} {label}"), f, rr, tol),
                _ => problems.push(format!("{name} {label}: missing")),
            }
        }
    }
    // Ranking: wherever R's swings clearly differ, Forge must order them the same way.
    let position = |name: &str| {
        bars.iter().position(|b| {
            b.get("input_name")
                .and_then(Json::as_str)
                .is_some_and(|n| n.rsplit('.').next() == Some(name))
        })
    };
    for (i, a) in r_vars.iter().enumerate() {
        for b in &r_vars[i + 1..] {
            let (Some(na), Some(nb), Some(sa), Some(sb)) = (
                a.get("name").and_then(Json::as_str),
                b.get("name").and_then(Json::as_str),
                json_f64(a, "swing"),
                json_f64(b, "swing"),
            ) else {
                continue;
            };
            if sa > sb.mul_add(1.0 + tol, 1e-12) {
                if let (Some(pa), Some(pb)) = (position(na), position(nb)) {
                    if pa > pb {
                        problems.push(format!("ranking: {na} should rank above {nb}"));
                    }
                }
            }
        }
    }
    verdict(spec, &problems, "Forge tornado matches the R reference")
}

// ─────────────────────────────────────────────────────────────────────────────
// Sensitivity
// ─────────────────────────────────────────────────────────────────────────────

/// Builds a Forge model with one assumption per variable and a single output formula.
fn linear_or_product_model(spec: &AnalyticsTestSpec) -> Option<String> {
    let model_kind = spec
        .raw
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("linear");
    let base = spec.raw.get("base_values")?.as_mapping()?;
    let coefficients = spec.raw.get("coefficients");
    let mut assumptions = String::new();
    let mut terms = Vec::new();
    for (name, value) in base {
        let name = name.as_str()?;
        let value = yaml_f64(value)?;
        assumptions.push_str(&format!(
            "  {name}:\n    value: {value}\n    formula: null\n"
        ));
        if let Some(c) = coefficients.and_then(|c| c.get(name)).and_then(yaml_f64) {
            terms.push(if model_kind == "multiplicative" {
                format!("(assumptions.{name} ^ ({c}))")
            } else {
                format!("(assumptions.{name} * ({c}))")
            });
        }
    }
    let joiner = if model_kind == "multiplicative" {
        " * "
    } else {
        " + "
    };
    Some(format!(
        "_forge_version: \"5.0.0\"\n\nassumptions:\n{assumptions}\nresult:\n  total:\n    value: null\n    formula: \"={}\"\n",
        terms.join(joiner)
    ))
}

/// Range "start,end,step" for `steps` evenly spaced points from low to high.
fn range_arg(spec: &AnalyticsTestSpec, var: &str) -> Result<String, String> {
    let r = spec
        .raw
        .get("range")
        .and_then(|r| r.get(var))
        .ok_or_else(|| format!("No range for {var}"))?;
    let (Some(low), Some(high), Some(steps)) = (
        r.get("low").and_then(yaml_f64),
        r.get("high").and_then(yaml_f64),
        r.get("steps").and_then(yaml_f64),
    ) else {
        return Err(format!("Range for {var} needs low, high and steps"));
    };
    if steps < 2.0 || (high - low).abs() < f64::EPSILON {
        return Err("single-point ranges".to_string());
    }
    Ok(format!("{low},{high},{}", (high - low) / (steps - 1.0)))
}

/// Numbers on each data row of Forge's printed table (rows made only of numbers).
fn numeric_rows(stdout: &str) -> Vec<Vec<f64>> {
    stdout
        .lines()
        .filter_map(|line| {
            let nums: Vec<f64> = line
                .split_whitespace()
                .filter_map(|t| t.parse().ok())
                .collect();
            (!nums.is_empty() && nums.len() == line.split_whitespace().count()).then_some(nums)
        })
        .collect()
}

fn sensitivity(spec: &AnalyticsTestSpec, env: &Env) -> TestResult {
    let kind = spec
        .raw
        .get("analysis_type")
        .and_then(|k| k.as_str())
        .unwrap_or("one_way");
    if kind == "elasticity" {
        return skip(spec, "Forge has no elasticity command");
    }
    let vary: Vec<String> = spec
        .raw
        .get("vary")
        .and_then(|v| v.as_sequence())
        .map(|s| {
            s.iter()
                .filter_map(|n| n.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let Some(model) = linear_or_product_model(spec) else {
        return error(spec, "Spec has no usable base_values".to_string());
    };
    let Some(first) = vary.first() else {
        return error(spec, "Spec has no 'vary' list".to_string());
    };
    let range1 = match range_arg(spec, first) {
        Ok(r) => r,
        Err(e) if e == "single-point ranges" => {
            return skip(spec, "Forge needs at least two points in a range");
        }
        Err(e) => return error(spec, e),
    };

    let dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => return error(spec, format!("Failed to create temp dir: {e}")),
    };
    let path = dir.path().join("model.yaml");
    if let Err(e) = fs::write(&path, model) {
        return error(spec, format!("Failed to write model: {e}"));
    }
    let mut cmd = Command::new(env.forge);
    cmd.arg("sensitivity")
        .arg(&path)
        .args([
            "--vary",
            &format!("assumptions.{first}"),
            "--range",
            &range1,
        ])
        .args(["--output", "result.total"]);
    if kind == "two_way" {
        let Some(second) = vary.get(1) else {
            return error(spec, "two_way needs two variables".to_string());
        };
        let range2 = match range_arg(spec, second) {
            Ok(r) => r,
            Err(e) => return error(spec, e),
        };
        cmd.args([
            "--vary2",
            &format!("assumptions.{second}"),
            "--range2",
            &range2,
        ]);
    }
    let out = match cmd.output() {
        Ok(o) => o,
        Err(e) => return error(spec, format!("Failed to run forge: {e}")),
    };
    if !out.status.success() {
        return error(
            spec,
            format!(
                "forge sensitivity failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ),
        );
    }
    let rows = numeric_rows(&String::from_utf8_lossy(&out.stdout));

    let r_params = json!({
        "analysis_type": kind,
        "base_values": spec.raw.get("base_values").map_or(Json::Null, to_json),
        "vary": vary,
        "range": spec.raw.get("range").map_or(Json::Null, to_json),
        "model": spec.raw.get("model").map_or(Json::Null, to_json),
        "coefficients": spec.raw.get("coefficients").map_or(Json::Null, to_json),
    });
    let r = match r_results(env, "sensitivity_validator.R", &r_params) {
        Ok(r) => r,
        Err(e) => return error(spec, e),
    };
    let data = r.get("data").cloned().unwrap_or(Json::Null);
    let tol = tolerance(spec, "output", 0.001);
    let mut problems = Vec::new();

    if kind == "two_way" {
        // Forge's first row is the column header (second variable's values).
        let grid: Vec<Vec<f64>> = data
            .get("grid")
            .and_then(Json::as_array)
            .map(|g| {
                g.iter()
                    .map(|row| {
                        row.as_array()
                            .map(|r| r.iter().filter_map(Json::as_f64).collect())
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .unwrap_or_default();
        // Rows of the table: [row_value, out, out, ...]
        let body: Vec<&Vec<f64>> = rows
            .iter()
            .filter(|r| r.len() == grid.first().map_or(0, Vec::len) + 1)
            .collect();
        if body.len() != grid.len() {
            problems.push(format!(
                "grid has {} rows in forge, {} in R",
                body.len(),
                grid.len()
            ));
        }
        for (i, (f_row, r_row)) in body.iter().zip(&grid).enumerate() {
            for (j, (f, rv)) in f_row[1..].iter().zip(r_row).enumerate() {
                check(&mut problems, &format!("grid[{i}][{j}]"), *f, *rv, tol);
            }
        }
    } else {
        let r_values: Vec<f64> = data
            .get("outputs")
            .and_then(Json::as_array)
            .map(|a| a.iter().filter_map(Json::as_f64).collect())
            .unwrap_or_default();
        let body: Vec<&Vec<f64>> = rows.iter().filter(|r| r.len() == 2).collect();
        if body.len() != r_values.len() {
            problems.push(format!(
                "{} points in forge, {} in R",
                body.len(),
                r_values.len()
            ));
        }
        for (i, (f_row, rv)) in body.iter().zip(&r_values).enumerate() {
            check(&mut problems, &format!("output[{i}]"), f_row[1], *rv, tol);
        }
    }
    verdict(
        spec,
        &problems,
        "Forge sensitivity table matches the R reference",
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Decision trees
// ─────────────────────────────────────────────────────────────────────────────

/// Lower-case identifier made from a node name, for use as a Forge branch key.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

/// Writes the branches of `node` and appends any child nodes to `nodes`.
fn tree_branches(
    node: &yaml_serde::Value,
    chance: bool,
    indent: &str,
    nodes: &mut String,
    path: &str,
) -> Result<String, String> {
    let children = node
        .get("children")
        .and_then(|c| c.as_sequence())
        .ok_or("node has no children")?;
    let mut out = String::new();
    for child in children {
        let name = child
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or("child has no name")?;
        let key = slug(name);
        out.push_str(&format!("{indent}{key}:\n"));
        if chance {
            let p = child
                .get("probability")
                .and_then(yaml_f64)
                .ok_or("chance child needs probability")?;
            out.push_str(&format!("{indent}  probability: {p}\n"));
        }
        if let Some(cost) = child.get("cost").and_then(yaml_f64) {
            out.push_str(&format!("{indent}  cost: {cost}\n"));
        }
        let kind = child
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("terminal");
        if kind == "terminal" {
            let payoff = child.get("payoff").and_then(yaml_f64).unwrap_or(0.0);
            out.push_str(&format!("{indent}  value: {payoff}\n"));
        } else {
            let id = format!("{path}_{key}");
            out.push_str(&format!("{indent}  next: {id}\n"));
            let inner = tree_branches(child, kind == "chance", "        ", nodes, &id)?;
            nodes.push_str(&format!(
                "    {id}:\n      type: {kind}\n      name: \"{}\"\n      branches:\n{inner}",
                name.replace('"', "'")
            ));
        }
    }
    Ok(out)
}

fn decision_tree(spec: &AnalyticsTestSpec, env: &Env) -> TestResult {
    let Some(tree) = spec.raw.get("tree") else {
        return error(spec, "Spec has no 'tree'".to_string());
    };
    let name = tree.get("name").and_then(|n| n.as_str()).unwrap_or("Tree");
    let root_kind = tree
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("decision");
    let mut nodes = String::new();
    let branches = match tree_branches(tree, root_kind == "chance", "        ", &mut nodes, "n") {
        Ok(b) => b,
        Err(e) => return error(spec, e),
    };
    let model = format!(
        "_forge_version: \"5.0.0\"\n\ndecision_tree:\n  name: \"{n}\"\n  root:\n    type: {root_kind}\n    name: \"{n}\"\n    branches:\n{branches}  nodes:\n{nodes}",
        n = name.replace('"', "'")
    );
    let forge = match forge_json(env, "decision-tree", &model) {
        Ok(j) => j,
        Err(e) => return error(spec, format!("{e}\nmodel:\n{model}")),
    };

    // Reference values: from R, or from the spec when --expected-only is set.
    let (ref_emv, ref_choice, source) = if env.expected_only {
        let e = spec.raw.get("r_expected");
        (
            e.and_then(|e| e.get("root_emv")).and_then(yaml_f64),
            e.and_then(|e| e.get("optimal_decision"))
                .and_then(|d| d.as_str())
                .map(String::from),
            "spec's expected values, R not run",
        )
    } else {
        let r = match r_results(
            env,
            "decision_tree_validator.R",
            &json!({ "tree": to_json(tree) }),
        ) {
            Ok(r) => r,
            Err(e) => return error(spec, e),
        };
        (
            json_f64(&r, "root_emv"),
            r.get("optimal_decision")
                .and_then(Json::as_str)
                .map(String::from),
            "R",
        )
    };

    let mut problems = Vec::new();
    match (json_f64(&forge, "root_expected_value"), ref_emv) {
        (Some(f), Some(r)) => check(
            &mut problems,
            "root EMV",
            f,
            r,
            tolerance(spec, "emv", 0.001),
        ),
        _ => problems.push("root EMV missing".to_string()),
    }
    if root_kind == "decision" {
        if let Some(choice) = ref_choice {
            let forge_choice = forge
                .get("node_results")
                .and_then(|n| n.get("root"))
                .and_then(|r| r.get("optimal_choice"))
                .and_then(Json::as_str)
                .unwrap_or("");
            if forge_choice != slug(&choice) {
                problems.push(format!(
                    "optimal decision: forge={forge_choice}, reference={choice}"
                ));
            }
        }
    }
    verdict(spec, &problems, &format!("Forge matches the {source}"))
}

// ─────────────────────────────────────────────────────────────────────────────
// Real options
// ─────────────────────────────────────────────────────────────────────────────

/// Forge values a "defer" option as a call and an "abandon" option as a put,
/// both American (binomial) or European (Black-Scholes). The spec's plain
/// call and put are mapped onto those two.
#[allow(clippy::many_single_char_names)]
fn real_options(spec: &AnalyticsTestSpec, env: &Env) -> TestResult {
    let num = |key: &str| spec.raw.get(key).and_then(yaml_f64);
    let text = |key: &str| spec.raw.get(key).and_then(|v| v.as_str());
    let (Some(s), Some(k)) = (num("S"), num("K")) else {
        return error(spec, "Spec needs S and K".to_string());
    };
    let (r, sigma, t, q) = (
        num("r").unwrap_or(0.05),
        num("sigma").unwrap_or(0.3),
        num("T").unwrap_or(1.0),
        num("q").unwrap_or(0.0),
    );
    let method = text("model").unwrap_or("black_scholes");
    let kind = text("option_type").unwrap_or("call");
    let american = spec
        .raw
        .get("american")
        .and_then(yaml_serde::Value::as_bool)
        .unwrap_or(false);
    // Steps are a whole number in the spec; this cast cannot lose a real value.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let steps = num("n").unwrap_or(100.0) as usize;

    let option = match (method, kind) {
        ("black_scholes", "call") => format!(
            "    - type: defer\n      name: \"option\"\n      max_deferral: {t}\n      exercise_cost: {k}\n"
        ),
        ("black_scholes", "put") => format!(
            "    - type: abandon\n      name: \"option\"\n      salvage_value: {k}\n"
        ),
        ("binomial", "call") if american || q == 0.0 => format!(
            "    - type: defer\n      name: \"option\"\n      max_deferral: {t}\n      exercise_cost: {k}\n"
        ),
        ("binomial", "put") if american => format!(
            "    - type: abandon\n      name: \"option\"\n      salvage_value: {k}\n"
        ),
        _ => {
            return skip(
                spec,
                "Forge values these options as American; a European put, or a European call with a dividend yield, cannot be expressed",
            )
        },
    };
    let steps_line = if method == "binomial" {
        format!("  binomial_steps: {steps}\n")
    } else {
        String::new()
    };
    let model = format!(
        "_forge_version: \"5.0.0\"\n\nreal_options:\n  name: \"{name}\"\n  method: {method}\n  underlying:\n    current_value: {s}\n    volatility: {sigma}\n    risk_free_rate: {r}\n    time_horizon: {t}\n    dividend_yield: {q}\n{steps_line}  options:\n{option}",
        name = spec.name
    );
    let forge = match forge_json(env, "real-options", &model) {
        Ok(j) => j,
        Err(e) => return error(spec, format!("{e}\nmodel:\n{model}")),
    };
    let Some(forge_value) = forge
        .get("options")
        .and_then(|o| o.get("option"))
        .and_then(|o| json_f64(o, "value"))
    else {
        return error(spec, "Forge output has no option value".to_string());
    };

    let tol = tolerance(spec, "price", 0.001);
    let mut problems = Vec::new();
    let source = if env.expected_only {
        let expected = spec
            .raw
            .get("r_expected")
            .and_then(|e| e.get("price"))
            .and_then(yaml_f64);
        let Some(expected) = expected else {
            return error(spec, "Spec has no expected price".to_string());
        };
        check(&mut problems, "price", forge_value, expected, tol);
        "spec's expected values, R not run"
    } else {
        let params = json!({
            "option_type": kind, "model": method, "S": s, "K": k, "r": r,
            "sigma": sigma, "T": t, "q": q, "n": steps, "american": true,
        });
        let r_out = match r_results(env, "real_options_validator.R", &params) {
            Ok(r) => r,
            Err(e) => return error(spec, e),
        };
        let Some(r_price) = json_f64(&r_out, "price") else {
            return error(spec, "R returned no price".to_string());
        };
        check(&mut problems, "price", forge_value, r_price, tol);
        // derivmkts, when installed, prices the same option independently.
        if let Some(d) = json_f64(&r_out, "price_derivmkts") {
            check(&mut problems, "price vs derivmkts", forge_value, d, tol);
        }
        "R"
    };
    verdict(spec, &problems, &format!("Forge matches the {source}"))
}

// ─────────────────────────────────────────────────────────────────────────────
// Bayesian networks
// ─────────────────────────────────────────────────────────────────────────────

fn string_list(value: Option<&yaml_serde::Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_sequence())
        .map(|s| {
            s.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Translates the spec's network into Forge's `bayesian_network` model.
/// The spec lists a table row by row: one row per state of the node, one
/// column per combination of parent states (first parent fastest).
fn bayesian_model(network: &yaml_serde::Value) -> Result<String, String> {
    let nodes = string_list(network.get("nodes"));
    let cpts = network.get("cpts").ok_or("Network has no 'cpts'")?;
    let mut out = String::from(
        "_forge_version: \"5.0.0\"\n\nbayesian_network:\n  name: \"network\"\n  nodes:\n",
    );
    for node in &nodes {
        let cpt = cpts
            .get(node.as_str())
            .ok_or_else(|| format!("No cpt for {node}"))?;
        let levels = string_list(cpt.get("levels"));
        let probs = cpt
            .get("probs")
            .and_then(yaml_f64_list)
            .ok_or_else(|| format!("No probs for {node}"))?;
        let parents = string_list(cpt.get("parents"));
        let quoted = |l: &[String]| {
            l.iter()
                .map(|x| format!("\"{x}\""))
                .collect::<Vec<_>>()
                .join(", ")
        };
        out.push_str(&format!(
            "    \"{node}\":\n      type: discrete\n      states: [{}]\n",
            quoted(&levels)
        ));
        if parents.is_empty() {
            out.push_str(&format!("      prior: {probs:?}\n"));
            continue;
        }
        // One column per combination of parent states, the first parent
        // varying fastest. Forge names a row by the parent states in the
        // order of `parents`, separated by commas.
        let parent_levels: Vec<Vec<String>> = parents
            .iter()
            .map(|p| string_list(cpts.get(p.as_str()).and_then(|c| c.get("levels"))))
            .collect();
        let combinations: usize = parent_levels.iter().map(Vec::len).product();
        if probs.len() != combinations * levels.len() {
            return Err(format!(
                "{node}: {} probabilities, expected {} states x {combinations} parent combinations",
                probs.len(),
                levels.len()
            ));
        }
        out.push_str(&format!(
            "      parents: [{}]\n      cpt:\n",
            quoted(&parents)
        ));
        for c in 0..combinations {
            let mut rest = c;
            let key: Vec<&str> = parent_levels
                .iter()
                .map(|l| {
                    let state = l[rest % l.len()].as_str();
                    rest /= l.len();
                    state
                })
                .collect();
            let row: Vec<f64> = (0..levels.len())
                .map(|state| probs[state * combinations + c])
                .collect();
            out.push_str(&format!("        \"{}\": {row:?}\n", key.join(", ")));
        }
    }
    Ok(out)
}

fn bayesian(spec: &AnalyticsTestSpec, env: &Env) -> TestResult {
    let Some(network) = spec.raw.get("network") else {
        return error(spec, "Spec has no 'network'".to_string());
    };
    let Some(target) = spec
        .raw
        .get("query")
        .and_then(|q| q.get("target"))
        .and_then(|t| t.as_str())
    else {
        return skip(spec, "No query: marginal probabilities are not compared");
    };
    let model = match bayesian_model(network) {
        Ok(m) => m,
        Err(e) => return error(spec, e),
    };
    let evidence: Vec<(String, String)> = spec
        .raw
        .get("query")
        .and_then(|q| q.get("evidence"))
        .and_then(|e| e.as_mapping())
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default();
    let mut extra = vec!["-q".to_string(), target.to_string()];
    for (var, state) in &evidence {
        extra.push("-e".to_string());
        extra.push(format!("{var}={state}"));
    }
    let forge = match forge_json_with(env, "bayesian", &model, &extra) {
        Ok(j) => j,
        Err(e) => return error(spec, format!("{e}\nmodel:\n{model}")),
    };
    let states = forge.get("states").and_then(Json::as_array);
    let probs = forge.get("probabilities").and_then(Json::as_array);
    let (Some(states), Some(probs)) = (states, probs) else {
        return error(
            spec,
            "Forge output has no states and probabilities".to_string(),
        );
    };

    // Reference posterior: from R, or from the spec when --expected-only is set.
    let (reference, source): (Vec<(String, f64)>, &str) = if env.expected_only {
        let posterior = spec
            .raw
            .get("r_expected")
            .and_then(|e| e.get("posterior"))
            .and_then(|p| p.as_mapping());
        let Some(posterior) = posterior else {
            return error(spec, "Spec has no expected posterior".to_string());
        };
        (
            posterior
                .iter()
                .filter_map(|(k, v)| Some((k.as_str()?.to_string(), yaml_f64(v)?)))
                .collect(),
            "spec's expected values, R not run",
        )
    } else {
        let evidence_json: serde_json::Map<String, Json> = evidence
            .iter()
            .map(|(k, v)| (k.clone(), json!(v)))
            .collect();
        let params = json!({
            "network": to_json(network),
            "query": { "target": target, "evidence": evidence_json },
        });
        let r_out = match r_results(env, "bayesian_validator.R", &params) {
            Ok(r) => r,
            Err(e) => return error(spec, e),
        };
        let Some(posterior) = r_out.get("posterior").and_then(Json::as_object) else {
            return error(spec, "R returned no posterior".to_string());
        };
        (
            posterior
                .iter()
                .filter_map(|(k, v)| Some((k.clone(), v.as_f64()?)))
                .collect(),
            "R",
        )
    };

    // R estimates the posterior by sampling, so the tolerance is absolute.
    let tol = tolerance(spec, "probability", 0.02);
    let mut problems = Vec::new();
    for (state, expected) in &reference {
        let forge_p = states
            .iter()
            .position(|s| s.as_str() == Some(state))
            .and_then(|i| probs.get(i))
            .and_then(Json::as_f64);
        match forge_p {
            Some(p) if (p - expected).abs() <= tol => {},
            Some(p) => problems.push(format!(
                "P({target}={state}): forge={p:.4}, reference={expected:.4}, tolerance={tol} (absolute)"
            )),
            None => problems.push(format!("state {state} missing from Forge output")),
        }
    }
    verdict(spec, &problems, &format!("Forge matches the {source}"))
}
