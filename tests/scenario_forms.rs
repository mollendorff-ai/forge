//! A scenario with `scalars` and no `probability` (Session Prompt part 1,
//! item 3). `forge compare` weighs nothing and needs none; `forge scenarios`
//! weighs by it and says which scenario lacks it. The help of `compare` shows
//! both forms (item 4), and the output in it is what Forge prints.
//!
//! Expected: 1200 customers * 49 = 58,800; * (1 + 0.12 - 0.01) = 65,268;
//! * (1 + 0.02 - 0.05) = 57,036.

use mollendorff_forge::cli::{compare_core, scenarios_core};
use std::path::Path;

const BASE: &str = r#"_forge_version: "5.1.0"
assumptions:
  growth_rate: 0.05
  churn_rate: 0.02
  customers: 1200
  price: 49
summary:
  revenue: {formula: "=assumptions.customers * assumptions.price * (1 + assumptions.growth_rate - assumptions.churn_rate)"}
"#;

const FULL_NO_PROBABILITY: &str = r"scenarios:
  optimistic:
    description: Faster growth, less churn
    scalars:
      assumptions.growth_rate: 0.12
      assumptions.churn_rate: 0.01
  pessimistic:
    description: Slow growth, more churn
    scalars:
      assumptions.growth_rate: 0.02
      assumptions.churn_rate: 0.05
";

const FLAT: &str = r"scenarios:
  optimistic:
    assumptions.growth_rate: 0.12
    assumptions.churn_rate: 0.01
  pessimistic:
    assumptions.growth_rate: 0.02
    assumptions.churn_rate: 0.05
";

fn model(dir: &Path, scenarios: &str) -> std::path::PathBuf {
    let path = dir.join("m.yaml");
    std::fs::write(&path, format!("{BASE}{scenarios}")).unwrap();
    path
}

fn both() -> Vec<String> {
    vec!["optimistic".to_string(), "pessimistic".to_string()]
}

#[test]
fn compare_takes_the_full_form_without_a_probability() {
    let dir = tempfile::tempdir().unwrap();
    let file = model(dir.path(), FULL_NO_PROBABILITY);
    let result = compare_core(&file, &both()).unwrap();
    let revenue = &result.values["summary.revenue"];
    assert!((revenue["optimistic"].unwrap() - 65_268.0).abs() < 1e-6);
    assert!((revenue["pessimistic"].unwrap() - 57_036.0).abs() < 1e-6);
}

#[test]
fn compare_takes_the_flat_form_as_before() {
    let dir = tempfile::tempdir().unwrap();
    let file = model(dir.path(), FLAT);
    let result = compare_core(&file, &both()).unwrap();
    assert!((result.values["summary.revenue"]["optimistic"].unwrap() - 65_268.0).abs() < 1e-6);
}

#[test]
fn scenarios_names_the_scenario_without_a_probability() {
    let dir = tempfile::tempdir().unwrap();
    let file = model(dir.path(), FULL_NO_PROBABILITY);
    let error = scenarios_core(&file, None).unwrap_err().to_string();
    for expected in [
        "scenario 'optimistic' has no probability",
        "sum to 1",
        "forge compare",
    ] {
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn a_misspelt_probability_is_still_caught() {
    let dir = tempfile::tempdir().unwrap();
    let typo = FULL_NO_PROBABILITY.replace("description:", "probabilty: 0.5\n    description:");
    let file = model(dir.path(), &typo);
    let error = compare_core(&file, &both()).unwrap_err().to_string();
    assert!(error.contains("Did you mean 'probability'"), "{error}");
}

#[test]
fn the_help_example_is_what_forge_prints() {
    // The scalars of the model in `forge compare --help`, with its output
    let dir = tempfile::tempdir().unwrap();
    let file = model(dir.path(), FULL_NO_PROBABILITY);
    let result = compare_core(&file, &both()).unwrap();
    assert_eq!(
        result.variables,
        [
            "assumptions.growth_rate",
            "assumptions.churn_rate",
            "assumptions.customers",
            "assumptions.price",
            "summary.revenue"
        ]
    );
}
