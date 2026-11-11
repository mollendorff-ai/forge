//! Numbers Forge writes or prints have at most 15 significant digits, as in
//! Excel (Session Prompt part 1, item 2; `core::precision`). `forge calculate`
//! wrote `418.3299999999999` and `2170.9700000000003`, and the bagel dry run
//! printed `year.worst_month_profit = -7935.533406400002`: binary rounding
//! noise in the last digits. The engine keeps full precision between formulas.

use mollendorff_forge::cli::calculate_core;
use mollendorff_forge::core::precision::to_15_digits;
use std::path::{Path, PathBuf};

/// Every YAML file under `dir`, at any depth, sorted.
fn yaml_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(yaml_files(&path));
        } else if path.extension().is_some_and(|e| e == "yaml") {
            out.push(path);
        }
    }
    out.sort();
    out
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

/// The numbers with more than 15 significant digits in a YAML document, with
/// where they are.
fn noisy_yaml(value: &yaml_serde::Value, at: &str, out: &mut Vec<String>) {
    match value {
        yaml_serde::Value::Number(n) => {
            if let Some(v) = n.as_f64().filter(|_| n.is_f64()) {
                if to_15_digits(v).to_bits() != v.to_bits() {
                    out.push(format!("{at} = {v}"));
                }
            }
        },
        yaml_serde::Value::Sequence(items) => {
            for (i, item) in items.iter().enumerate() {
                noisy_yaml(item, &format!("{at}[{i}]"), out);
            }
        },
        yaml_serde::Value::Mapping(map) => {
            for (k, v) in map {
                let key = k.as_str().map_or_else(|| format!("{k:?}"), str::to_string);
                noisy_yaml(v, &format!("{at}.{key}"), out);
            }
        },
        yaml_serde::Value::Tagged(tagged) => noisy_yaml(&tagged.value, at, out),
        _ => {},
    }
}

fn noisy_json(value: &serde_json::Value, at: &str, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Number(n) if n.is_f64() => {
            let v = n.as_f64().unwrap();
            if to_15_digits(v).to_bits() != v.to_bits() {
                out.push(format!("{at} = {v}"));
            }
        },
        serde_json::Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                noisy_json(item, &format!("{at}[{i}]"), out);
            }
        },
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                noisy_json(v, &format!("{at}.{k}"), out);
            }
        },
        _ => {},
    }
}

fn noisy_file(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap();
    let mut out = Vec::new();
    for (i, document) in text.split("\n---").enumerate() {
        if let Ok(value) = yaml_serde::from_str::<yaml_serde::Value>(document) {
            noisy_yaml(&value, &format!("{}#{i}", path.display()), &mut out);
        }
    }
    out
}

#[test]
fn no_value_stored_in_the_examples_has_more_than_15_digits() {
    let files = yaml_files(Path::new("examples"));
    assert!(files.len() > 20, "{files:?}");
    let noisy: Vec<String> = files.iter().flat_map(|f| noisy_file(f)).collect();
    assert!(noisy.is_empty(), "noise in the examples: {noisy:#?}");
}

#[test]
fn calculating_any_example_stores_and_returns_no_noise() {
    let dir = tempfile::tempdir().unwrap();
    copy_dir(Path::new("examples"), dir.path());
    let mut calculated = 0;
    for file in yaml_files(dir.path()) {
        // Files of other commands (simulate, decision trees, ...) do not calculate
        let Ok(result) = calculate_core(&file, false, None) else {
            continue;
        };
        calculated += 1;
        let mut noisy = noisy_file(&file);
        let json = serde_json::to_value(&result).unwrap();
        noisy_json(&json, &format!("{} (result)", file.display()), &mut noisy);
        assert!(noisy.is_empty(), "{noisy:#?}");
    }
    assert!(calculated >= 10, "only {calculated} examples calculated");
}

#[test]
fn a_noisy_sum_is_stored_as_written_and_the_engine_keeps_every_digit() {
    // 14.11 + 404.22 is 418.33000000000004 in binary floating point
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("m.yaml");
    std::fs::write(
        &path,
        r#"_forge_version: "5.1.0"
prices:
  a: 14.11
  b: 404.22
  total:
    formula: "=a + b"
  # The noise itself, which the engine keeps: about 5.7e-14, not 0
  residual:
    formula: "=(a + b) - 418.33"
"#,
    )
    .unwrap();
    let result = calculate_core(&path, false, None).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("    value: 418.33\n"), "{text}");
    assert_eq!(result.scalars["prices.total"], Some(418.33));
    let residual = result.scalars["prices.residual"].unwrap();
    assert!(residual > 5e-14 && residual < 6e-14, "{residual}");
    assert!(text.contains("    value: 5.6843418860808e-14\n"), "{text}");
}

/// A coverage build stubs `main` (ADR-006), so this runs the real binary only
/// in an ordinary build.
#[cfg(not(coverage))]
#[test]
fn the_bagel_dry_run_prints_the_worst_month_without_noise() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["calculate", "examples/bagel/bagel.yaml", "--dry-run"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("year.worst_month_profit = -7935.5334064\n"),
        "{text}"
    );
    assert!(!text.contains("7935.533406400002"), "{text}");
}
