//! `forge export` of a model that does not calculate says so (roadmap
//! T1-EXPORT-UNCALCULATED). It said "Export Complete" and nothing else, though
//! the formulas were then written without results, and what a formula column
//! holds is known only from the results (ADR-054).

use mollendorff_forge::cli::{export_buffer_core, export_core};

const FAILING: &str = r#"_forge_version: "5.2.0"
t:
  d: ["2026-01-31", "2026-02-28"]
  next: "=EDATE(d, 1)"
  y4: "=LEFT(next, 4)"
r:
  missing: {formula: "=MATCH(99, t.d, 0)"}
"#;

const FINE: &str = r#"_forge_version: "5.2.0"
t:
  d: ["2026-01-31", "2026-02-28"]
  next: "=EDATE(d, 1)"
r:
  found: {formula: "=MATCH(\"2026-02-28\", t.d, 0)"}
"#;

fn model(dir: &std::path::Path, text: &str) -> std::path::PathBuf {
    let path = dir.join("m.yaml");
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_model_that_does_not_calculate_is_exported_with_the_error_named() {
    let dir = tempfile::tempdir().unwrap();
    let input = model(dir.path(), FAILING);
    let result = export_core(&input, &dir.path().join("m.xlsx")).unwrap();
    assert!(dir.path().join("m.xlsx").exists());
    let first = &result.warnings[0];
    for expected in ["does not calculate", "MATCH: value not found", "r.missing"] {
        assert!(first.contains(expected), "{first}");
    }
}

#[test]
fn the_buffer_export_says_it_too() {
    let dir = tempfile::tempdir().unwrap();
    let input = model(dir.path(), FAILING);
    let result = export_buffer_core(&input).unwrap();
    assert!(result.warnings[0].contains("does not calculate"));
}

#[test]
fn a_model_that_calculates_has_no_such_warning() {
    let dir = tempfile::tempdir().unwrap();
    let input = model(dir.path(), FINE);
    let result = export_core(&input, &dir.path().join("m.xlsx")).unwrap();
    assert_eq!(result.warnings, Vec::<String>::new());
    assert_eq!(
        export_buffer_core(&input).unwrap().warnings,
        Vec::<String>::new()
    );
}

#[cfg(not(coverage))]
#[test]
fn the_command_prints_it_before_export_complete() {
    let dir = tempfile::tempdir().unwrap();
    let input = model(dir.path(), FAILING);
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["export"])
        .arg(&input)
        .arg(dir.path().join("m.xlsx"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let warned = text.find("does not calculate").expect(&text);
    let done = text.find("Export Complete").expect(&text);
    assert!(warned < done, "{text}");
}
