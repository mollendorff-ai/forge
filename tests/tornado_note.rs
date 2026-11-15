//! `forge tornado` says when an input cannot move the output, instead of
//! listing it with a swing of 0 and no reason (roadmap T1-PIPELINE-FINDINGS).

#![cfg(not(coverage))]

use std::process::Command;

#[test]
fn the_printed_tornado_names_the_input_that_cannot_move_the_output() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = dir.path().join("t.yaml");
    std::fs::write(
        &file,
        r#"_forge_version: "5.1.0"
price: 10
tax_rate: 0.2
revenue: {formula: "=price * 100", value: 1000}
tornado:
  output: revenue
  inputs:
    - {name: price, low: 5, high: 15}
    - {name: tax_rate, low: 0.1, high: 0.3}
"#,
    )
    .expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["tornado", file.to_str().expect("path")])
        .output()
        .expect("forge runs");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(text.matches("does not use it").count(), 1, "{text}");
}
