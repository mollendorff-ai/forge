//! Unknown keys are errors, not silently ignored
//!
//! A misspelt or unsupported key used to be dropped, so the model ran with a
//! default instead (for example `expansion_cost` gave an Expand option a cost
//! of 0). Each case below names a key that some file actually used.

use crate::bayesian::BayesianConfig;
use crate::bootstrap::BootstrapConfig;
use crate::decision_trees::DecisionTreeConfig;
use crate::monte_carlo::MonteCarloConfig;
use crate::parser::validate_against_schema;
use crate::real_options::RealOptionsConfig;
use crate::scenarios::ScenarioDefinition;
use crate::tornado::TornadoConfig;
use serde::de::DeserializeOwned;

fn rejects<T: DeserializeOwned>(yaml: &str, key: &str) {
    let err = yaml_serde::from_str::<T>(yaml)
        .err()
        .unwrap_or_else(|| panic!("accepted unknown key `{key}`:\n{yaml}"))
        .to_string();
    assert!(err.contains(&format!("unknown field `{key}`")), "{err}");
}

fn accepts<T: DeserializeOwned>(yaml: &str) {
    if let Err(e) = yaml_serde::from_str::<T>(yaml) {
        panic!("rejected valid config: {e}\n{yaml}");
    }
}

const UNDERLYING: &str =
    "underlying: {current_value: 100, volatility: 0.3, risk_free_rate: 0.05, time_horizon: 1}\n";

#[test]
fn test_real_options_unknown_keys() {
    accepts::<RealOptionsConfig>(&format!(
        "{UNDERLYING}binomial_steps: 50\noptions:\n  - {{type: expand, name: e, exercise_cost: 5, expansion_factor: 2}}\n"
    ));
    rejects::<RealOptionsConfig>(
        &format!("{UNDERLYING}options:\n  - {{type: expand, name: e, expansion_cost: 5}}\n"),
        "expansion_cost",
    );
    rejects::<RealOptionsConfig>(
        &format!("{UNDERLYING}options:\n  - {{type: abandon, name: a, available_after: 1}}\n"),
        "available_after",
    );
    rejects::<RealOptionsConfig>(
        "underlying: {current_value: 100, volatility: 0.3, risk_free_rate: 0.05, time_horizon: 1, steps: 50}\n",
        "steps",
    );
    rejects::<RealOptionsConfig>(
        &format!("{UNDERLYING}investment: {{cost: 10}}\n"),
        "investment",
    );
}

#[test]
fn test_tornado_unknown_keys() {
    accepts::<TornadoConfig>(
        "output: npv\ninputs:\n  - {name: g, low: 0.1, high: 0.2, base: 0.15}\n",
    );
    rejects::<TornadoConfig>("output: npv\nbase_values: {g: 0.1}\n", "base_values");
    rejects::<TornadoConfig>(
        "output: npv\ninputs:\n  - {name: g, low: 0.1, high: 0.2, label: Growth}\n",
        "label",
    );
}

#[test]
fn test_bootstrap_unknown_keys() {
    accepts::<BootstrapConfig>("data: [1, 2, 3]\nstatistic: percentile\npercentile_value: 5\n");
    rejects::<BootstrapConfig>(
        "data: [1, 2, 3]\nstatistics:\n  - {type: mean}\n",
        "statistics",
    );
}

#[test]
fn test_monte_carlo_unknown_keys() {
    accepts::<MonteCarloConfig>(
        "iterations: 100\nseed: 1\noutputs:\n  - {variable: npv, percentiles: [10, 90], threshold: \"> 0\"}\n",
    );
    rejects::<MonteCarloConfig>("iteration: 100\n", "iteration");
    rejects::<MonteCarloConfig>(
        "outputs:\n  - {variable: npv, percentile: [10]}\n",
        "percentile",
    );
    rejects::<MonteCarloConfig>(
        "correlations:\n  - {variables: [a, b], correlation: 0.5}\n",
        "correlation",
    );
}

#[test]
fn test_decision_tree_unknown_keys() {
    accepts::<DecisionTreeConfig>(
        "root:\n  type: decision\n  branches:\n    go: {cost: 10, value: 100}\n",
    );
    rejects::<DecisionTreeConfig>(
        "root:\n  type: chance\n  branches:\n    up: {probabilty: 0.6, value: 100}\n",
        "probabilty",
    );
    rejects::<DecisionTreeConfig>("root:\n  type: decision\n  options: {}\n", "options");
}

#[test]
fn test_bayesian_and_scenario_unknown_keys() {
    accepts::<BayesianConfig>(
        "nodes:\n  rain: {type: discrete, states: [y, n], prior: [0.2, 0.8]}\n",
    );
    rejects::<BayesianConfig>(
        "nodes:\n  rain: {type: discrete, states: [y, n], priors: [0.2, 0.8]}\n",
        "priors",
    );
    accepts::<ScenarioDefinition>("probability: 0.3\nscalars: {growth: 0.1}\n");
    rejects::<ScenarioDefinition>("probability: 0.3\ngrowth: 0.1\n", "growth");
}

#[test]
fn test_schema_rejects_unknown_scalar_keys() {
    let validate = |yaml: &str| validate_against_schema(&yaml_serde::from_str(yaml).unwrap());
    // A misspelt `formula` used to pass validation, and the scalar was never calculated
    for yaml in [
        "_forge_version: \"5.0.0\"\ns:\n  b: {value: null, fromula: \"=1 + 1\"}\n",
        "_forge_version: \"1.0.0\"\ns:\n  b: {value: null, fromula: \"=1 + 1\"}\n",
        "_forge_version: \"5.0.0\"\ns:\n  b: {value: 2, formula: null, expected: 2}\n",
        "_forge_version: \"5.0.0\"\nt:\n  x: {value: [1, 2], units: USD}\n",
        "_forge_version: \"5.0.0\"\nt:\n  x: [1, 2]\n  y: {formula: \"=x * 2\", note: double}\n",
    ] {
        assert!(validate(yaml).is_err(), "accepted:\n{yaml}");
    }
    for yaml in [
        "_forge_version: \"5.0.0\"\ns:\n  b: {value: 2, formula: null, unit: USD, notes: n, source: s, validation_status: VALIDATED, last_updated: \"2026-11-05\"}\n",
        "_forge_version: \"5.0.0\"\nt:\n  x: {value: [1, 2], unit: USD, notes: n}\n  y: {formula: \"=x * 2\", unit: USD}\n",
    ] {
        assert!(validate(yaml).is_ok(), "rejected:\n{yaml}\n{:?}", validate(yaml).err());
    }
}
