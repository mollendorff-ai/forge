//! Two gaps `forge validate` had (T1-VALIDATE-GAPS): a table whose columns
//! have different lengths passed `validate` when no formula read it, and a scalar name
//! with a typo was reported as an unknown column with no suggestion.
//!
//! A coverage build stubs `main` (ADR-006), so these run the real binary only
//! in an ordinary build.

#![cfg(not(coverage))]

use std::path::Path;
use std::process::{Command, Output};

const RAGGED: &str = r#"_forge_version: "5.1.0"
sales:
  month: [Jan, Feb, Mar]
  units: [10, 20]
"#;

const TYPO: &str = r#"_forge_version: "5.1.0"
a:
  prce: 100
b:
  total:
    formula: "=a.price * 2"
"#;

fn forge(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(args)
        .output()
        .expect("forge runs")
}

fn write(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, text).expect("write");
    path.to_string_lossy().into_owned()
}

fn all_output(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn validate_refuses_a_table_with_columns_of_different_lengths() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = write(dir.path(), "ragged.yaml", RAGGED);
    let out = forge(&["validate", &file]);
    let text = all_output(&out);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(!text.contains("All tables are valid"), "{text}");
    assert!(text.contains("sales"), "names the table: {text}");
    assert!(
        text.contains("units") && text.contains("2 rows") && text.contains("3 rows"),
        "names the column and both lengths: {text}"
    );
}

#[test]
fn export_writes_a_table_with_columns_of_different_lengths_and_warns() {
    // Columns of different lengths are ranges in the workbook (the Gnumeric
    // suite's fixtures rely on it), so export warns instead of refusing.
    let dir = tempfile::tempdir().expect("temp dir");
    let file = write(dir.path(), "ragged.yaml", RAGGED);
    let output = dir.path().join("out.xlsx");
    let out = forge(&["export", &file, output.to_str().expect("utf-8")]);
    let text = all_output(&out);
    assert!(out.status.success(), "{text}");
    assert!(output.exists(), "the workbook is written");
    assert!(text.contains("units") && text.contains("2 rows"), "{text}");
}

#[test]
fn a_misspelled_scalar_suggests_the_closest_name() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = write(dir.path(), "typo.yaml", TYPO);
    let out = forge(&["validate", &file]);
    let text = all_output(&out);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.contains("a.price"), "{text}");
    assert!(
        text.contains("a.prce"),
        "suggests the name that exists: {text}"
    );
    assert!(
        !text.contains("Unknown column"),
        "it is a name, not a column: {text}"
    );
}
