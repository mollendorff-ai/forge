//! `forge sensitivity` refuses a date input however it is asked for (ADR-060):
//! an error and a failing exit code, not a table of errors.

#![cfg(not(coverage))]

use std::path::Path;
use std::process::Command;

const MODEL: &str = r#"_forge_version: "5.4.0"
inputs:
  last_actual: 2026-10-31
out:
  next:
    formula: "=EOMONTH(inputs.last_actual, 1)"
"#;

fn write(dir: &Path) -> String {
    let path = dir.join("model.yaml");
    std::fs::write(&path, MODEL).expect("model written");
    path.display().to_string()
}

#[test]
fn sensitivity_refuses_a_date_input_and_says_so_in_its_exit_code() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = write(dir.path());
    let out = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args([
            "sensitivity",
            &file,
            "--vary",
            "inputs.last_actual",
            "--range",
            "1,3,1",
            "--output",
            "out.next",
        ])
        .output()
        .expect("forge runs");
    assert!(!out.status.success(), "a date input must fail the command");
    let text =
        String::from_utf8_lossy(&out.stderr).to_string() + &String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("varying a date is not supported"), "{text}");
    assert!(!text.contains("Sensitivity analysis complete"), "{text}");
}
