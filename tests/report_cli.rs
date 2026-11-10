//! `forge report` on the command line: what it prints, what it leaves alone.

use std::path::Path;
use std::process::{Command, Output};

const CALCULATED: &str = r#"_forge_version: "5.1.0"
le_george:
  rent: 1450
  utilities: 60
  year1_monthly: {formula: "=le_george.rent + le_george.utilities", value: 1510}
unit_806:
  rent: 1390
  utilities: 90
  year1_monthly: {formula: "=unit_806.rent + unit_806.utilities", value: 1480}
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

#[test]
fn report_prints_markdown_and_leaves_the_file_as_it_was() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = write(dir.path(), "options.yaml", CALCULATED);
    let out = forge(&["report", &file, "--format", "markdown"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "\
| group | rent | utilities | year1_monthly |
| --- | ---: | ---: | ---: |
| le_george | 1,450.00 | 60.00 | 1,510.00 |
| unit_806 | 1,390.00 | 90.00 | 1,480.00 |
"
    );
    assert!(out.stderr.is_empty(), "{out:?}");
    assert_eq!(std::fs::read_to_string(&file).expect("read"), CALCULATED);
}

#[test]
fn report_of_an_uncalculated_file_says_so_and_succeeds() {
    let dir = tempfile::tempdir().expect("temp dir");
    let text = CALCULATED
        .replace(", value: 1510", "")
        .replace(", value: 1480", "");
    let file = write(dir.path(), "options.yaml", &text);
    let out = forge(&["report", &file]);
    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("le_george  1,450.00      60.00              —"),
        "{stdout}"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(
        stderr.starts_with("Note: 2 formulas have no value yet"),
        "{stderr}"
    );
    // report does not calculate: the file still has no values.
    assert_eq!(std::fs::read_to_string(&file).expect("read"), text);
}

#[test]
fn report_csv_has_full_precision() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = write(
        dir.path(),
        "rate.yaml",
        "_forge_version: \"5.1.0\"\nloan:\n  rate: 0.047912345\n  principal: 450000\n",
    );
    let out = forge(&["report", &file, "--format", "csv"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "group,rate,principal\nloan,0.047912345,450000\n"
    );
}

#[test]
fn report_rejects_an_unknown_group_and_an_unknown_format() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = write(dir.path(), "options.yaml", CALCULATED);
    let out = forge(&["report", &file, "--group", "unit_807"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr)
        .contains("'unit_807' is not a group of scalars in the file. Groups: le_george, unit_806"));

    let out = forge(&["report", &file, "--format", "html"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
}

#[test]
fn keys_and_no_inputs_are_one_or_the_other() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = write(dir.path(), "options.yaml", CALCULATED);
    let out = forge(&["report", &file, "--keys", "rent", "--no-inputs"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
}
