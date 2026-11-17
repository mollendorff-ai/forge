//! What five fresh agents stumbled on while running docs/PROMPTS.md
//! (roadmap T1-AGENT-FRICTION). Each test is one of the things they reported.
//! The help text's own examples and the short flags are checked in
//! `src/cli/args.rs`.
//!
//! A coverage build stubs `main` (ADR-006), so these run the real binary only
//! in an ordinary build.

#![cfg(not(coverage))]

use std::path::Path;
use std::process::{Command, Output};

fn forge(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(args)
        .output()
        .expect("forge runs")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn all(out: &Output) -> String {
    format!("{}{}", stdout(out), String::from_utf8_lossy(&out.stderr))
}

fn write(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, text).expect("write");
    path.to_string_lossy().into_owned()
}

/// Inputs in one group, the result in another, as a first model has them.
const GROUPED: &str = r#"_forge_version: "5.1.0"
assumptions:
  price_per_dozen: 18
  dozens: 1000
  cost: 12000
results:
  profit: {formula: "=assumptions.price_per_dozen * assumptions.dozens - assumptions.cost", value: 6000}
"#;

// ── Help ─────────────────────────────────────────────────────────────────

#[test]
fn calculate_help_says_it_keeps_a_bak_copy() {
    let text = stdout(&forge(&["calculate", "--help"]));
    assert!(text.contains(".bak"), "{text}");
}

#[test]
fn variance_help_explains_better() {
    let text = stdout(&forge(&["variance", "--help"]));
    assert!(text.contains("better:"), "{text}");
}

#[test]
fn tornado_help_names_the_output_variable_flag() {
    let text = stdout(&forge(&["tornado", "--help"]));
    assert!(!text.contains("--output npv"), "{text}");
    assert!(text.contains("--output-var npv"), "{text}");
}

// ── Names: a bare name works where it names one scalar ───────────────────

#[test]
fn break_even_goal_seek_and_sensitivity_take_a_bare_name() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = write(dir.path(), "m.yaml", GROUPED);

    let out = forge(&[
        "break-even",
        &file,
        "--output-var",
        "profit",
        "--vary",
        "price_per_dozen",
    ]);
    assert!(out.status.success(), "{}", all(&out));
    assert!(stdout(&out).contains("12"), "{}", all(&out));

    let out = forge(&[
        "goal-seek",
        &file,
        "--target",
        "profit",
        "--value",
        "0",
        "--vary",
        "price_per_dozen",
    ]);
    assert!(out.status.success(), "{}", all(&out));

    let out = forge(&[
        "sensitivity",
        &file,
        "--vary",
        "price_per_dozen",
        "--range",
        "10,20,5",
        "--output-var",
        "profit",
    ]);
    assert!(out.status.success(), "{}", all(&out));
    assert!(
        stdout(&out).contains("8,000") || stdout(&out).contains("8000"),
        "{}",
        all(&out)
    );

    // The full name still works, and so does the old --output
    let out = forge(&[
        "break-even",
        &file,
        "--output",
        "results.profit",
        "--vary",
        "assumptions.price_per_dozen",
    ]);
    assert!(out.status.success(), "{}", all(&out));
}

#[test]
fn a_bare_name_that_fits_two_scalars_lists_both() {
    let dir = tempfile::tempdir().expect("temp dir");
    let text = format!("{GROUPED}other:\n  price_per_dozen: 20\n");
    let file = write(dir.path(), "m.yaml", &text);
    let out = forge(&[
        "break-even",
        &file,
        "--output-var",
        "profit",
        "--vary",
        "price_per_dozen",
    ]);
    assert!(!out.status.success());
    let text = all(&out);
    assert!(
        text.contains("assumptions.price_per_dozen") && text.contains("other.price_per_dozen"),
        "{text}"
    );
}

// ── Validate ─────────────────────────────────────────────────────────────

#[test]
fn validate_before_any_calculation_does_not_claim_values_match() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = write(
        dir.path(),
        "new.yaml",
        "_forge_version: \"5.1.0\"\na: 2\nb: {formula: \"=a * 3\"}\n",
    );
    let out = forge(&["validate", &file]);
    let text = all(&out);
    assert!(out.status.success(), "{text}");
    assert!(!text.contains("match their formulas"), "{text}");
    assert!(text.contains("forge calculate"), "{text}");
}

#[test]
fn validate_after_an_edit_says_the_formulas_are_fine_and_the_values_old() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = write(
        dir.path(),
        "edited.yaml",
        "_forge_version: \"5.1.0\"\na: 2\nb: {formula: \"=a * 4\", value: 6}\n",
    );
    let out = forge(&["validate", &file]);
    let text = all(&out);
    assert!(!out.status.success(), "{text}");
    assert!(text.contains("formulas are valid"), "{text}");
    assert!(text.contains("forge calculate"), "{text}");
}

// ── Export and import say what they carry over ───────────────────────────

#[test]
fn export_names_what_stays_only_in_the_yaml() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = write(
        dir.path(),
        "m.yaml",
        r#"_forge_version: "5.1.0"
# Prices for the spring
price: 10
cost: {value: 4, better: lower}
profit: {formula: "=price - cost", value: 6}
tornado:
  output: profit
  inputs:
    - {name: price, low: 8, high: 12}
"#,
    );
    let xlsx = dir.path().join("m.xlsx");
    let out = forge(&["export", &file, xlsx.to_str().expect("path")]);
    let text = all(&out);
    assert!(out.status.success(), "{text}");
    for word in ["tornado", "better:", "comments"] {
        assert!(text.contains(word), "{word}: {text}");
    }
}

#[test]
fn import_says_when_every_formula_was_kept() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = write(
        dir.path(),
        "m.yaml",
        "_forge_version: \"5.1.0\"\nprice: 10\ncost: 4\nprofit: {formula: \"=price - cost\", value: 6}\n",
    );
    let xlsx = dir.path().join("m.xlsx");
    assert!(forge(&["export", &file, xlsx.to_str().expect("path")])
        .status
        .success());
    let back = dir.path().join("back.yaml");
    let out = forge(&[
        "import",
        xlsx.to_str().expect("path"),
        back.to_str().expect("path"),
    ]);
    let text = all(&out);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("kept as formulas"), "{text}");
}

// ── Real options: what waiting adds ──────────────────────────────────────

#[test]
fn real_options_prints_what_each_option_adds() {
    let out = forge(&["real-options", "examples/real-options.yaml"]);
    let text = all(&out);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("Adds over the traditional NPV"), "{text}");
    assert!(!text.contains("): option value"), "{text}");
}

// ── Variance: counts agree with the lines, and a script can fail on alerts ─

const BUDGET: &str = "_forge_version: \"5.1.0\"\nrent: 1000\nsales: {value: 5000, better: higher}\nfood: {value: 300, better: lower}\n";
const ACTUAL: &str = "_forge_version: \"5.1.0\"\nrent: 1000\nsales: {value: 4000, better: higher}\nfood: {value: 280, better: lower}\n";

#[test]
fn variance_counts_agree_with_each_line() {
    let dir = tempfile::tempdir().expect("temp dir");
    let b = write(dir.path(), "b.yaml", BUDGET);
    let a = write(dir.path(), "a.yaml", ACTUAL);
    let report = dir.path().join("r.yaml");
    let out = forge(&["variance", &b, &a, "-o", report.to_str().expect("path")]);
    assert!(out.status.success(), "{}", all(&out));
    let text = std::fs::read_to_string(&report).expect("report");
    let doc: serde_json::Value = yaml_serde::from_str(&text).expect("yaml");
    let lines = doc["variances"].as_object().expect("variances");
    let favorable = lines
        .values()
        .filter(|l| l["is_favorable"] == serde_json::Value::Bool(true))
        .count();
    assert_eq!(favorable, 1, "{text}");
    assert_eq!(
        doc["metadata"]["favorable_count"].as_u64(),
        Some(1),
        "{text}"
    );
    // On target is neither favorable nor unfavorable
    assert_eq!(
        lines["rent"]["is_favorable"],
        serde_json::Value::Null,
        "{text}"
    );
}

#[test]
fn variance_fail_on_alert_exits_1_when_a_line_exceeds_the_threshold() {
    let dir = tempfile::tempdir().expect("temp dir");
    let b = write(dir.path(), "b.yaml", BUDGET);
    let a = write(dir.path(), "a.yaml", ACTUAL);
    assert!(forge(&["variance", &b, &a]).status.success());
    let out = forge(&["variance", &b, &a, "--fail-on-alert"]);
    assert_eq!(out.status.code(), Some(1), "{}", all(&out));
    let same = forge(&["variance", &b, &b, "--fail-on-alert"]);
    assert!(same.status.success(), "{}", all(&same));
}

// ── Examples ─────────────────────────────────────────────────────────────

#[test]
fn example_comments_give_paths_that_work_in_any_examples_folder() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut wrong = Vec::new();
    for entry in walk(&root) {
        if entry.extension().is_some_and(|e| e == "yaml") {
            let text = std::fs::read_to_string(&entry).expect("read");
            for line in text.lines().filter(|l| l.trim_start().starts_with('#')) {
                if line.contains("examples/") {
                    wrong.push(format!("{}: {line}", entry.display()));
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "install.sh copies the examples to forge-examples/, so a path under examples/ fails:\n{}",
        wrong.join("\n")
    );
}

#[test]
fn the_starter_says_where_the_other_format_versions_come_from() {
    let text = stdout(&forge(&["examples", "starter"]));
    assert!(text.contains("forge schema --list"), "{text}");
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).expect("read dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}
