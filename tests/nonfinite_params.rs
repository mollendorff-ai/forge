//! A distribution parameter that is not a finite number (roadmap T1-NONFINITE).
//! `MC.PERT(0, 1, 1E+400)` panicked, which ends the process in a release
//! build, and `MC.Triangular` or `MC.Lognormal` with an infinite argument
//! printed NaN results with exit 0. Each must now stop with exit 1 and name
//! the input and the argument.

#![cfg(not(coverage))]

use std::process::Command;

fn simulate(formula: &str) -> std::process::Output {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = dir.path().join("mc.yaml");
    std::fs::write(
        &file,
        format!(
            r#"_forge_version: "5.1.0"
inputs:
  x:
    formula: "={formula}"
  y:
    formula: "=inputs.x * 2"
monte_carlo:
  iterations: 100
  seed: 1
  outputs:
    - variable: inputs.y
"#
        ),
    )
    .expect("write");
    Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["simulate", file.to_str().expect("path")])
        .output()
        .expect("forge runs")
}

#[test]
fn an_infinite_parameter_stops_with_exit_1_and_names_it() {
    let cases = [
        (
            "MC.PERT(0, 1, 1E+400)",
            "inputs.x: MC.PERT: max must be a finite number, got inf",
        ),
        (
            "MC.Triangular(0, 1, 1E+400)",
            "inputs.x: MC.Triangular: max must be a finite number, got inf",
        ),
        (
            "MC.Lognormal(1, 1E+400)",
            "inputs.x: MC.Lognormal: stdev must be a finite number, got inf",
        ),
        (
            "MC.Normal(1E+400, 1)",
            "inputs.x: MC.Normal: mean must be a finite number, got inf",
        ),
        (
            "MC.Uniform(0, 1E+400)",
            "inputs.x: MC.Uniform: max must be a finite number, got inf",
        ),
    ];
    for (formula, expected) in cases {
        let out = simulate(formula);
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(out.status.code(), Some(1), "{formula}: {stderr}");
        assert!(stderr.contains(expected), "{formula}: {stderr}");
        assert!(!stdout.contains("NaN"), "{formula} printed NaN: {stdout}");
    }
}
