//! Numbers in the output of goal-seek and tornado follow `forge report`'s rule
//! (Session Prompt part 1, item 5): two decimals and thousands separators,
//! four significant digits under 1. They printed 1148.273882 and `swing 0.18`.
//!
//! A coverage build stubs `main` (ADR-006), so these run the real binary only
//! in an ordinary build.

#![cfg(not(coverage))]

use std::process::Command;

fn run(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn goal_seek_prints_the_solution_with_two_decimals() {
    // examples/mortgage: 397.04 a month saves 80,000 of interest (README)
    let text = run(&[
        "goal-seek",
        "examples/mortgage/mortgage.yaml",
        "--target",
        "saved.interest",
        "--value",
        "80000",
        "--vary",
        "loan.extra_per_month",
    ]);
    assert!(
        text.contains("loan.extra_per_month = 397.04 → saved.interest = 80,000.00"),
        "{text}"
    );
}

#[test]
fn tornado_prints_amounts_with_separators() {
    let text = run(&["tornado", "examples/tornado.yaml"]);
    assert!(text.contains("Base value: -625,000.00"), "{text}");
    // initial_investment 700,000 x 1: -425,000 at low, -1,125,000 at high
    assert!(
        text.contains("swing 700,000.00  (output -425,000.00 at low, -1,125,000.00 at high)"),
        "{text}"
    );
}

#[test]
fn tornado_keeps_the_digits_of_a_rate() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("rate.yaml");
    // margin = profit / revenue = (1000 - cost) / 1000: cost 840 gives 0.16,
    // cost 903 gives 0.097; the swing is 0.063 (0.06 at two decimals)
    std::fs::write(
        &file,
        r#"_forge_version: "5.1.0"
tornado:
  output: plan.margin
  inputs:
    - name: plan.cost
      low: 840
      high: 903
plan:
  revenue: 1000
  cost: 870
  margin: {formula: "=(plan.revenue - plan.cost) / plan.revenue"}
"#,
    )
    .unwrap();
    let text = run(&["tornado", file.to_str().unwrap()]);
    assert!(text.contains("swing 0.063"), "{text}");
    assert!(
        text.contains("0.16 at low") || text.contains("0.160 at low"),
        "{text}"
    );
}
