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

// `forge scenarios`, `forge simulate --seek`, `forge decision-tree` and
// `forge real-options` followed their own rules (six decimals, `$61500.00`,
// `$-117000.00`). They now print numbers as `forge report` does, and money
// with separators and the sign before the `$`. JSON and files keep the raw
// numbers.

#[test]
fn scenarios_print_numbers_as_forge_report_does() {
    let text = run(&["scenarios", "examples/bagel/bagel.yaml"]);
    assert!(text.contains("year.profit: 786.32\n"), "{text}");
    assert!(text.contains("year.revenue: 386,827.45\n"), "{text}");
    assert!(text.contains("year.npv_first_year: -95,579.52\n"), "{text}");
    assert!(text.contains("year.margin: 0.002033\n"), "{text}");
    assert!(
        text.contains("(range -71,197.20 to 80,811.24, P(>0) = 70%)"),
        "{text}"
    );
    assert!(!text.contains("786.320362"), "{text}");
}

#[test]
fn a_probability_seek_prints_the_solution_with_two_decimals() {
    let text = run(&[
        "simulate",
        "examples/bagel/bagel_simulate.yaml",
        "--seek",
        "P(year.profit > 0) = 0.8",
        "--vary",
        "price_per_dozen",
        "--min",
        "14",
        "--max",
        "24",
    ]);
    assert!(
        text.contains("assumptions.price_per_dozen = 18.32\n"),
        "{text}"
    );
    assert!(!text.contains("18.324083"), "{text}");
}

#[test]
fn a_decision_tree_prints_money_with_separators() {
    let text = run(&["decision-tree", "examples/bagel/second_shop.yaml"]);
    for expected in [
        "Expected Value: $61,500.00",
        "Best case:  $198,000.00",
        "Worst case: -$117,000.00",
        "-$12,000.00 with probability 50.0%",
    ] {
        assert!(text.contains(expected), "{expected}\n{text}");
    }
    assert!(!text.contains("$-"), "{text}");
}

#[test]
fn real_options_print_money_with_separators() {
    let text = run(&["real-options", "examples/bagel/wait_option.yaml"]);
    for expected in [
        "invest now, no flexibility): $60,000.00",
        "option value $64,604.68",
        "Underlying value: $240,000.00",
        "exercise when value > $198,000",
        "project value $64,605.",
        "worth $4,605 more than deciding now",
    ] {
        assert!(text.contains(expected), "{expected}\n{text}");
    }
}

#[test]
fn the_json_of_an_analysis_keeps_the_numbers_not_the_display() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("tree.json");
    run(&[
        "decision-tree",
        "examples/bagel/second_shop.yaml",
        "-o",
        file.to_str().unwrap(),
    ]);
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(json["root_expected_value"], 61_500.0);
}
