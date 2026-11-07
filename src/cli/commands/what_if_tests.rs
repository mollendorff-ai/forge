//! Scenarios and goal seek must change the numbers, find them, or fail; never quietly do neither.

use super::{calculate_core, scenarios_core};
use std::path::PathBuf;

const COSTS: &str =
    "_forge_version: \"5.0.0\"\nprices:\n  flour:\n    value: 2\n    formula: null\n";

/// A model where profit = units * (price - @costs.prices.flour).
fn write_model(dir: &std::path::Path, override_line: &str) -> PathBuf {
    std::fs::write(dir.join("costs.yaml"), COSTS).unwrap();
    let model = format!(
        "_forge_version: \"5.0.0\"\n_includes:\n  - file: costs.yaml\n    as: costs\n\
         inputs:\n  price:\n    value: 5\n    formula: null\n  units:\n    value: 100\n    formula: null\n\
         out:\n  profit:\n    value: null\n    formula: \"=inputs.units * (inputs.price - @costs.prices.flour)\"\n\
         scenarios:\n  base:\n    probability: 0.5\n    scalars: {{}}\n  shock:\n    probability: 0.5\n    scalars:\n      {override_line}\n"
    );
    let path = dir.join("model.yaml");
    std::fs::write(&path, model).unwrap();
    path
}

fn shock_profit(results: &crate::scenarios::ScenarioResults) -> f64 {
    results
        .scenarios
        .iter()
        .find(|s| s.name == "shock")
        .unwrap()
        .scalars["out.profit"]
}

#[test]
fn test_misspelled_override_is_an_error_with_a_suggestion() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_model(dir.path(), "prise: 6");
    let err = scenarios_core(&path, None).unwrap_err().to_string();
    assert!(err.contains("'prise' does not name a scalar"), "{err}");
    assert!(err.contains("Did you mean 'inputs.price'"), "{err}");
    // `forge calculate --scenario` takes the same path
    let err = calculate_core(&path, true, Some("shock"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("'prise' does not name a scalar"), "{err}");
}

#[test]
fn test_bare_and_dotted_overrides_change_the_result() {
    let dir = tempfile::tempdir().unwrap();
    for line in ["price: 6", "inputs.price: 6"] {
        let path = write_model(dir.path(), line);
        let results = scenarios_core(&path, None).unwrap();
        assert!((shock_profit(&results) - 400.0).abs() < 1e-9, "{line}");
    }
}

#[test]
fn test_override_of_an_included_value() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_model(dir.path(), "\"@costs.prices.flour\": 3");
    let results = scenarios_core(&path, None).unwrap();
    assert!((shock_profit(&results) - 200.0).abs() < 1e-9);
    let calc = calculate_core(&path, true, Some("shock")).unwrap();
    assert!((calc.scalars["out.profit"].unwrap() - 200.0).abs() < 1e-9);
}

#[test]
fn test_override_of_an_unknown_include_or_name_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_model(dir.path(), "\"@rates.prices.flour\": 3");
    let err = scenarios_core(&path, None).unwrap_err().to_string();
    assert!(
        err.contains("no `_includes` entry with `as: rates`"),
        "{err}"
    );
    let path = write_model(dir.path(), "\"@costs.prices.flower\": 3");
    let err = scenarios_core(&path, None).unwrap_err().to_string();
    assert!(err.contains("Did you mean 'prices.flour'"), "{err}");
}

#[test]
fn test_outputs_ranges_and_filter() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_model(dir.path(), "price: 1");
    let results = scenarios_core(&path, Some("shock")).unwrap();
    assert_eq!(results.scenarios.len(), 1);
    assert_eq!(results.scenarios[0].name, "shock");
    // base 300, shock -100: half the probability is positive
    assert!((results.probability_positive["out.profit"] - 0.5).abs() < 1e-9);
    assert_eq!(results.ranges["out.profit"], (-100.0, 300.0));
    let err = scenarios_core(&path, Some("nope")).unwrap_err().to_string();
    assert!(err.contains("Scenario 'nope' not found"), "{err}");
}

/// A steep output (each unit of input moves it by 25,000) used to stop with
/// `converged: false` when the answer was already within a cent: the loop
/// compared the width of the input range with a tolerance meant for the output.
#[test]
fn test_goal_seek_converges_on_a_steep_output() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("steep.yaml");
    std::fs::write(
        &path,
        "_forge_version: \"5.0.0\"\ninputs:\n  price:\n    value: 16\n    formula: null\n\
         out:\n  profit:\n    value: null\n    formula: \"=inputs.price * 25000 - 399000\"\n",
    )
    .unwrap();
    for target in [0.0, 60_000.0] {
        let r = super::goal_seek_core(
            &path,
            "out.profit",
            target,
            "inputs.price",
            (Some(10.0), Some(40.0)),
            0.0001,
        )
        .unwrap();
        assert!(r.converged, "{target}: {}", r.error);
        assert!(r.error <= 0.0001);
        assert!((r.solution - (target + 399_000.0) / 25_000.0).abs() < 1e-8);
    }
}
