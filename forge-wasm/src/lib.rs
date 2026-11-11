//! Forge's tools in a browser (ADR-055).
//!
//! Every tool runs the same core function as the command line and the MCP
//! server (`calculate_core`, `simulate_core`, ...) on a model held in memory:
//! nothing here calculates. A request is JSON with the model's files and the
//! tool's arguments, which are the MCP tool's arguments without file paths:
//!
//! ```json
//! {"files": {"bagel.yaml": "...", "costs.yaml": "..."}, "main": "bagel.yaml",
//!  "target": "year.profit", "value": 60000, "vary": "assumptions.staff_hours_per_day"}
//! ```
//!
//! `files` holds the main model and every file it includes, by the path
//! written in `_includes`. The answer is `{"ok": ...}` with the core
//! function's result, or `{"error": "..."}` with Forge's message.

use mollendorff_forge::cli::{
    audit_core, bayesian_core, bootstrap_core, calculate_core, compare_core, decision_tree_core,
    examples_core, functions_core, goal_seek_core, probability_seek_core, real_options_core,
    scenarios_core, schema_core, sensitivity_core, simulate_core, tornado_core, validate_core,
    variance_core, SeekGoal, SimulateOptions,
};
use mollendorff_forge::cli::{
    commands::render, commands::report_core, ReportFormat, ReportOptions,
};
use mollendorff_forge::parser::{parse_model, with_files_in_memory};
use mollendorff_forge::types::ParsedModel;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::Path;
use wasm_bindgen::prelude::wasm_bindgen;

/// The tools, by the names of the MCP tools without `forge_`.
pub const TOOLS: [&str; 20] = [
    "validate",
    "calculate",
    "audit",
    "scenarios",
    "compare",
    "goal_seek",
    "break_even",
    "sensitivity",
    "tornado",
    "simulate",
    "probability_seek",
    "variance",
    "report",
    "decision_tree",
    "real_options",
    "bootstrap",
    "bayesian",
    "functions",
    "examples",
    "schema",
];

/// Tools for Forge Studio that are not MCP tools.
///
/// `model` parses a model without calculating it and answers with what the page shows on its cards:
/// each scalar's input value or formula, each table column's data or formula,
/// in the order of the file, and the included files' scalars under their
/// namespace.
pub const STUDIO_TOOLS: [&str; 1] = ["model"];

/// Run `tool` on `request` (JSON) and answer in JSON: `{"ok": result}` or
/// `{"error": message}`.
#[wasm_bindgen]
#[must_use]
pub fn call(tool: &str, request: &str) -> String {
    // A panic aborts (the release profile), so its message only reaches the
    // console through this hook
    static HOOK: std::sync::Once = std::sync::Once::new();
    HOOK.call_once(|| std::panic::set_hook(Box::new(console_error_panic_hook::hook)));
    let answer = serde_json::from_str::<Value>(request)
        .map_err(|e| format!("The request is not JSON: {e}"))
        .and_then(|request| run(tool, &request));
    match answer {
        Ok(result) => json!({ "ok": result }),
        Err(message) => json!({ "error": message }),
    }
    .to_string()
}

/// The version of Forge this build runs.
#[wasm_bindgen]
#[must_use]
pub fn version() -> String {
    mollendorff_forge::VERSION.to_string()
}

/// [`call`] without JSON text on either side.
///
/// # Errors
///
/// Returns Forge's message when the tool fails, or a message naming what is
/// wrong with the request.
pub fn run(tool: &str, request: &Value) -> Result<Value, String> {
    let args = request
        .as_object()
        .ok_or("The request must be a JSON object")?;
    match tool {
        "functions" => return functions_core().map_err(|e| e.to_string()),
        "examples" => return examples_core(opt_str(args, "name")?).map_err(|e| e.to_string()),
        "schema" => {
            return schema_core(opt_str(args, "version")?)
                .map(Value::String)
                .map_err(|e| e.to_string())
        },
        _ => {},
    }
    if !TOOLS.contains(&tool) && !STUDIO_TOOLS.contains(&tool) {
        return Err(format!(
            "No tool named '{tool}'. The tools are: {}, {}",
            TOOLS.join(", "),
            STUDIO_TOOLS.join(", ")
        ));
    }
    let files = files(args)?;
    let main = opt_str(args, "main")?.unwrap_or("model.yaml");
    if tool != "variance" && !files.contains_key(main) {
        return Err(format!("'main' names {main}, which is not in 'files'"));
    }
    with_files_in_memory(files, || run_on_files(tool, Path::new(main), args))
}

/// [`run`] on files on disk in `dir`, for comparing with the files in memory.
/// `main`, `budget` and `actual` are names in `dir`; `files` is ignored.
///
/// # Errors
///
/// As [`run`].
#[cfg(not(target_arch = "wasm32"))]
pub fn run_on_disk(tool: &str, request: &Value, dir: &Path) -> Result<Value, String> {
    let mut args = request
        .as_object()
        .ok_or("The request must be a JSON object")?
        .clone();
    for key in ["budget", "actual"] {
        if let Some(name) = args.get(key).and_then(Value::as_str) {
            let path = dir.join(name).to_string_lossy().into_owned();
            args.insert(key.to_string(), Value::String(path));
        }
    }
    let main = dir.join(opt_str(&args, "main")?.unwrap_or("model.yaml"));
    run_on_files(tool, &main, &args)
}

/// The tools that read a model, with its files in memory.
fn run_on_files(tool: &str, main: &Path, args: &Map<String, Value>) -> Result<Value, String> {
    let fail = |e: mollendorff_forge::ForgeError| e.to_string();
    match tool {
        "validate" => to_value(validate_core(main).map_err(fail)?),
        "model" => parse_model(main)
            .map(|model| structure(&model))
            .map_err(fail),
        // Results are never written back: there is no file to write to
        "calculate" => {
            to_value(calculate_core(main, true, opt_str(args, "scenario")?).map_err(fail)?)
        },
        "audit" => to_value(audit_core(main, str_arg(args, "variable")?).map_err(fail)?),
        "scenarios" => {
            to_value(scenarios_core(main, opt_str(args, "scenario_filter")?).map_err(fail)?)
        },
        "compare" => to_value(compare_core(main, &strings(args, "scenarios")?).map_err(fail)?),
        "goal_seek" => to_value(
            goal_seek_core(
                main,
                str_arg(args, "target")?,
                f64_arg(args, "value")?,
                str_arg(args, "vary")?,
                (opt_f64(args, "min")?, opt_f64(args, "max")?),
                opt_f64(args, "tolerance")?.unwrap_or(0.0001),
            )
            .map_err(fail)?,
        ),
        // As the command line: a goal seek for 0
        "break_even" => to_value(
            goal_seek_core(
                main,
                str_arg(args, "output")?,
                0.0,
                str_arg(args, "vary")?,
                (opt_f64(args, "min")?, opt_f64(args, "max")?),
                0.0001,
            )
            .map_err(fail)?,
        ),
        "sensitivity" => to_value(
            sensitivity_core(
                main,
                str_arg(args, "vary")?,
                str_arg(args, "range")?,
                opt_str(args, "vary2")?,
                opt_str(args, "range2")?,
                str_arg(args, "output")?,
            )
            .map_err(fail)?,
        ),
        "tornado" => to_value(tornado_core(main, opt_str(args, "output_var")?).map_err(fail)?),
        "simulate" => simulate(main, args),
        "probability_seek" => probability_seek(main, args),
        "variance" => to_value(
            variance_core(
                Path::new(str_arg(args, "budget")?),
                Path::new(str_arg(args, "actual")?),
                opt_f64(args, "threshold")?.unwrap_or(10.0),
            )
            .map_err(fail)?,
        ),
        "report" => report(main, args),
        "decision_tree" => to_value(decision_tree_core(main).map_err(fail)?),
        "real_options" => to_value(real_options_core(main).map_err(fail)?),
        "bootstrap" => bootstrap(main, args),
        "bayesian" => to_value(
            bayesian_core(
                main,
                opt_str(args, "query")?,
                &opt_strings(args, "evidence")?,
            )
            .map_err(fail)?,
        ),
        _ => Err(format!("No tool named '{tool}'")),
    }
}

/// What a model holds, before it is calculated (the `model` tool).
fn structure(model: &ParsedModel) -> Value {
    let scalars = |model: &ParsedModel, prefix: &str| -> Vec<Value> {
        model
            .scalars
            .values()
            .map(|v| {
                json!({
                    "path": format!("{prefix}{}", v.path),
                    "value": v.value,
                    "formula": v.formula,
                    "unit": v.metadata.unit,
                    "notes": v.metadata.notes,
                    "better": v.metadata.better,
                })
            })
            .collect()
    };
    let tables = |model: &ParsedModel, prefix: &str| -> Vec<Value> {
        model
            .tables
            .values()
            .map(|t| {
                let columns: Vec<Value> = t
                    .column_names()
                    .into_iter()
                    .map(|name| {
                        t.row_formulas.get(name).map_or_else(
                            || {
                                let c = &t.columns[name];
                                json!({"name": name, "type": c.values.type_name(),
                                       "unit": c.metadata.unit, "rows": c.values.len()})
                            },
                            |formula| json!({"name": name, "formula": formula}),
                        )
                    })
                    .collect();
                json!({"name": format!("{prefix}{}", t.name), "columns": columns})
            })
            .collect()
    };
    let includes: Vec<Value> = model
        .includes
        .iter()
        .map(|inc| {
            let prefix = format!("@{}.", inc.namespace);
            let (s, t) = model
                .resolved_includes
                .get(&inc.namespace)
                .map_or_else(Default::default, |r| {
                    (scalars(&r.model, &prefix), tables(&r.model, &prefix))
                });
            json!({"file": inc.file, "as": inc.namespace, "scalars": s, "tables": t})
        })
        .collect();
    let mut scenarios: Vec<&String> = model.scenarios.keys().collect();
    scenarios.sort();
    json!({
        "scalars": scalars(model, ""),
        "tables": tables(model, ""),
        "includes": includes,
        "scenarios": scenarios,
    })
}

fn simulate(main: &Path, args: &Map<String, Value>) -> Result<Value, String> {
    let result = simulate_core(main, simulate_options(args)?).map_err(|e| e.to_string())?;
    serde_json::from_str(&result.to_json().map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

fn probability_seek(main: &Path, args: &Map<String, Value>) -> Result<Value, String> {
    let goal = args
        .get("goal")
        .and_then(Value::as_object)
        .ok_or("'goal' must be {\"output\", \"threshold\", \"probability\"}")?;
    let goal = SeekGoal {
        output: str_arg(goal, "output")?.to_string(),
        threshold: str_arg(goal, "threshold")?.to_string(),
        probability: f64_arg(goal, "probability")?,
    };
    let result = probability_seek_core(
        main,
        simulate_options(args)?,
        &goal,
        str_arg(args, "input")?,
        f64_arg(args, "min")?,
        f64_arg(args, "max")?,
    )
    .map_err(|e| e.to_string())?;
    to_value(result)
}

fn report(main: &Path, args: &Map<String, Value>) -> Result<Value, String> {
    let options = ReportOptions {
        groups: opt_strings(args, "groups")?,
        keys: opt_strings(args, "keys")?,
        no_inputs: args
            .get("no_inputs")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    };
    let report = report_core(main, &options).map_err(|e| e.to_string())?;
    serde_json::from_str(&render(&report, ReportFormat::Json)).map_err(|e| e.to_string())
}

fn bootstrap(main: &Path, args: &Map<String, Value>) -> Result<Value, String> {
    let confidence = match args.get("confidence") {
        None | Some(Value::Null) => None,
        Some(_) => Some(numbers(args, "confidence")?),
    };
    let result = bootstrap_core(
        main,
        opt_u64(args, "iterations")?.map(to_usize),
        opt_u64(args, "seed")?,
        confidence,
    )
    .map_err(|e| e.to_string())?;
    to_value(result)
}

fn simulate_options(args: &Map<String, Value>) -> Result<SimulateOptions<'_>, String> {
    Ok(SimulateOptions {
        iterations: opt_u64(args, "iterations")?.map(to_usize),
        seed: opt_u64(args, "seed")?,
        sampling: opt_str(args, "sampling")?,
        scenario: opt_str(args, "scenario")?,
    })
}

fn to_value(result: impl serde::Serialize) -> Result<Value, String> {
    serde_json::to_value(result).map_err(|e| e.to_string())
}

fn files(args: &Map<String, Value>) -> Result<HashMap<String, String>, String> {
    let files = args
        .get("files")
        .and_then(Value::as_object)
        .ok_or("'files' must be an object of file name to YAML text")?;
    files
        .iter()
        .map(|(name, text)| {
            text.as_str()
                .map(|text| (name.clone(), text.to_string()))
                .ok_or_else(|| format!("files.{name} must be the file's text"))
        })
        .collect()
}

fn str_arg<'a>(args: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    opt_str(args, key)?.ok_or_else(|| format!("'{key}' is required"))
}

fn opt_str<'a>(args: &'a Map<String, Value>, key: &str) -> Result<Option<&'a str>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(_) => Err(format!("'{key}' must be text")),
    }
}

fn f64_arg(args: &Map<String, Value>, key: &str) -> Result<f64, String> {
    opt_f64(args, key)?.ok_or_else(|| format!("'{key}' is required"))
}

fn opt_f64(args: &Map<String, Value>, key: &str) -> Result<Option<f64>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_f64()
            .map(Some)
            .ok_or_else(|| format!("'{key}' must be a number")),
    }
}

fn opt_u64(args: &Map<String, Value>, key: &str) -> Result<Option<u64>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .map(Some)
            .ok_or_else(|| format!("'{key}' must be a whole number of 0 or more")),
    }
}

fn strings(args: &Map<String, Value>, key: &str) -> Result<Vec<String>, String> {
    args.get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("'{key}' must be a list of text"))?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("'{key}' must be a list of text"))
        })
        .collect()
}

fn opt_strings(args: &Map<String, Value>, key: &str) -> Result<Vec<String>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(_) => strings(args, key),
    }
}

fn numbers(args: &Map<String, Value>, key: &str) -> Result<Vec<f64>, String> {
    args.get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("'{key}' must be a list of numbers"))?
        .iter()
        .map(|v| {
            v.as_f64()
                .ok_or_else(|| format!("'{key}' must be a list of numbers"))
        })
        .collect()
}

/// A count from JSON; on a 32-bit target a count past `usize::MAX` is capped.
fn to_usize(n: u64) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

/// The test cases of `tests/cases.json` (native builds only): the requests
/// that the native test and the Node check of the WebAssembly build both run.
#[cfg(not(target_arch = "wasm32"))]
pub mod cases {
    use serde_json::{json, Value};
    use std::path::Path;

    /// The cases, from `tests/cases.json`.
    ///
    /// # Panics
    ///
    /// Panics if the file is not the expected JSON (a fault in the repository).
    #[must_use]
    pub fn all() -> Vec<Value> {
        serde_json::from_str(include_str!("../tests/cases.json")).expect("tests/cases.json")
    }

    /// The request of `case`, with its files read from `examples`.
    ///
    /// # Panics
    ///
    /// Panics if a file named by the case cannot be read.
    #[must_use]
    pub fn request(case: &Value, examples: &Path) -> Value {
        let mut request = case.get("args").cloned().unwrap_or_else(|| json!({}));
        if let Some(names) = case.get("files").and_then(Value::as_array) {
            let dir = examples.join(case["dir"].as_str().unwrap_or(""));
            let files: serde_json::Map<String, Value> = names
                .iter()
                .filter_map(Value::as_str)
                .map(|name| {
                    let text = std::fs::read_to_string(dir.join(name))
                        .unwrap_or_else(|e| panic!("{}: {e}", dir.join(name).display()));
                    (name.to_string(), Value::String(text))
                })
                .collect();
            request["files"] = Value::Object(files);
        }
        if let Some(main) = case.get("main") {
            request["main"] = main.clone();
        }
        request
    }
}
