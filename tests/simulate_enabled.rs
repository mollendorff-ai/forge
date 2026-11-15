//! `monte_carlo.enabled` is accepted but `forge simulate` never reads it: a
//! model whose section says `enabled: false`, or leaves it out, still runs.
//! docs/MCP.md says so; this keeps the doc honest.

#![cfg(not(coverage))]

use std::process::Command;

fn model(enabled_line: &str) -> String {
    format!(
        r#"_forge_version: "5.1.0"
monte_carlo:
{enabled_line}  iterations: 100
  seed: 42
  outputs:
    - variable: revenue
      percentiles: [50]
revenue:
  value: 100000
  formula: "=MC.Normal(100000, 15000)"
"#
    )
}

fn simulate(text: &str) -> std::process::Output {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = dir.path().join("mc.yaml");
    std::fs::write(&file, text).expect("write");
    Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["simulate", file.to_str().expect("path")])
        .output()
        .expect("forge runs")
}

#[test]
fn simulate_runs_without_enabled_and_with_enabled_false() {
    for line in ["", "  enabled: false\n", "  enabled: true\n"] {
        let out = simulate(&model(line));
        assert!(out.status.success(), "{line:?}: {out:?}");
    }
}

#[test]
fn mcp_md_does_not_say_enabled_is_required() {
    let doc = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/MCP.md"))
        .expect("docs/MCP.md");
    assert!(
        !doc.contains("with `enabled: true`"),
        "docs/MCP.md says a Monte Carlo section needs enabled: true; forge simulate never reads it"
    );
}
