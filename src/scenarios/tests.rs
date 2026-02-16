//! Scenario Analysis Integration Tests

use super::*;
use crate::types::{ParsedModel, Variable};

/// Helper to create a test model with formulas
fn create_profit_model() -> ParsedModel {
    let mut model = ParsedModel::new();

    // Base inputs
    model.scalars.insert(
        "revenue".to_string(),
        Variable::new("revenue".to_string(), Some(1_000_000.0), None),
    );
    model.scalars.insert(
        "costs".to_string(),
        Variable::new("costs".to_string(), Some(700_000.0), None),
    );
    model.scalars.insert(
        "tax_rate".to_string(),
        Variable::new("tax_rate".to_string(), Some(0.25), None),
    );

    // Calculated outputs
    model.scalars.insert(
        "gross_profit".to_string(),
        Variable::new(
            "gross_profit".to_string(),
            None,
            Some("=revenue - costs".to_string()),
        ),
    );
    model.scalars.insert(
        "net_profit".to_string(),
        Variable::new(
            "net_profit".to_string(),
            None,
            Some("=gross_profit * (1 - tax_rate)".to_string()),
        ),
    );

    model
}

/// Helper to create standard 3-scenario config
fn create_three_scenario_config() -> ScenarioConfig {
    let mut config = ScenarioConfig::new();

    config.add_scenario(
        "base_case",
        ScenarioDefinition::new(0.50)
            .with_description("Business as usual")
            .with_scalar("revenue", 1_000_000.0)
            .with_scalar("costs", 700_000.0),
    );

    config.add_scenario(
        "bull_case",
        ScenarioDefinition::new(0.30)
            .with_description("Market expansion")
            .with_scalar("revenue", 1_500_000.0)
            .with_scalar("costs", 900_000.0),
    );

    config.add_scenario(
        "bear_case",
        ScenarioDefinition::new(0.20)
            .with_description("Market contraction")
            .with_scalar("revenue", 600_000.0)
            .with_scalar("costs", 500_000.0),
    );

    config
}

#[test]
fn test_full_scenario_workflow() {
    let config = create_three_scenario_config();
    let model = create_profit_model();

    let engine = ScenarioEngine::new(config, model)
        .unwrap()
        .with_outputs(vec!["gross_profit".to_string(), "net_profit".to_string()]);

    let results = engine.run().unwrap();

    // Verify all scenarios ran
    assert_eq!(results.scenarios.len(), 3);

    // Verify expected value calculation (R's weighted.mean equivalent)
    // Base: 1M - 700K = 300K gross, 225K net
    // Bull: 1.5M - 900K = 600K gross, 450K net
    // Bear: 600K - 500K = 100K gross, 75K net
    // EV(gross) = 0.5*300K + 0.3*600K + 0.2*100K = 150K + 180K + 20K = 350K
    // EV(net) = 0.5*225K + 0.3*450K + 0.2*75K = 112.5K + 135K + 15K = 262.5K

    let ev_gross = results.expected_values.get("gross_profit").unwrap();
    assert!(
        (ev_gross - 350_000.0).abs() < 0.01,
        "Expected gross profit EV of 350K, got {ev_gross}"
    );

    let ev_net = results.expected_values.get("net_profit").unwrap();
    assert!(
        (ev_net - 262_500.0).abs() < 0.01,
        "Expected net profit EV of 262.5K, got {ev_net}"
    );
}

#[test]
fn test_scenario_isolation() {
    // Ensure scenarios don't affect each other
    let config = create_three_scenario_config();
    let model = create_profit_model();

    let engine = ScenarioEngine::new(config, model)
        .unwrap()
        .with_outputs(vec!["gross_profit".to_string()]);

    let results = engine.run().unwrap();

    // Find each scenario and verify correct calculation
    for scenario in &results.scenarios {
        let revenue = scenario.scalars.get("revenue").unwrap();
        let costs = scenario.scalars.get("costs").unwrap();
        let gross = scenario.scalars.get("gross_profit").unwrap();

        let expected = revenue - costs;
        assert!(
            (gross - expected).abs() < 0.01,
            "Scenario {} gross profit mismatch: {} != {}",
            scenario.name,
            gross,
            expected
        );
    }
}

#[test]
fn test_probability_weighting() {
    let mut config = ScenarioConfig::new();

    // Two scenarios with known probabilities
    config.add_scenario(
        "heads",
        ScenarioDefinition::new(0.50).with_scalar("outcome", 100.0),
    );
    config.add_scenario(
        "tails",
        ScenarioDefinition::new(0.50).with_scalar("outcome", 0.0),
    );

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "outcome".to_string(),
        Variable::new("outcome".to_string(), Some(0.0), None),
    );

    let engine = ScenarioEngine::new(config, model)
        .unwrap()
        .with_outputs(vec!["outcome".to_string()]);

    let results = engine.run().unwrap();

    // Expected value should be 50
    let ev = results.expected_values.get("outcome").unwrap();
    assert!((ev - 50.0).abs() < 0.01, "Expected 50.0, got {ev}");
}

#[test]
fn test_probability_positive_calculation() {
    let mut config = ScenarioConfig::new();

    // Mix of positive and negative outcomes
    config.add_scenario(
        "win",
        ScenarioDefinition::new(0.60).with_scalar("pnl", 100.0),
    );
    config.add_scenario(
        "lose",
        ScenarioDefinition::new(0.40).with_scalar("pnl", -50.0),
    );

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "pnl".to_string(),
        Variable::new("pnl".to_string(), Some(0.0), None),
    );

    let engine = ScenarioEngine::new(config, model)
        .unwrap()
        .with_outputs(vec!["pnl".to_string()]);

    let results = engine.run().unwrap();

    // Probability positive = 0.60 (only "win" has positive PnL)
    let prob_pos = results.probability_positive.get("pnl").unwrap();
    assert!(
        (prob_pos - 0.60).abs() < 0.01,
        "Expected 0.60 prob positive, got {prob_pos}"
    );

    // Expected value = 0.60*100 + 0.40*(-50) = 60 - 20 = 40
    let ev = results.expected_values.get("pnl").unwrap();
    assert!((ev - 40.0).abs() < 0.01, "Expected EV of 40, got {ev}");
}

#[test]
fn test_ranges_calculation() {
    let config = create_three_scenario_config();
    let model = create_profit_model();

    let engine = ScenarioEngine::new(config, model)
        .unwrap()
        .with_outputs(vec!["gross_profit".to_string()]);

    let results = engine.run().unwrap();

    // Ranges: Base=300K, Bull=600K, Bear=100K
    let (min, max) = results.ranges.get("gross_profit").unwrap();
    assert!(
        (min - 100_000.0).abs() < 0.01,
        "Expected min 100K, got {min}"
    );
    assert!(
        (max - 600_000.0).abs() < 0.01,
        "Expected max 600K, got {max}"
    );
}

#[test]
fn test_invalid_probabilities_rejected() {
    let mut config = ScenarioConfig::new();
    config.add_scenario("only", ScenarioDefinition::new(0.5));

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("sum to 1.0"));
}

#[test]
fn test_negative_probability_rejected() {
    let mut config = ScenarioConfig::new();
    config.add_scenario("neg", ScenarioDefinition::new(-0.5));
    config.add_scenario("pos", ScenarioDefinition::new(1.5));

    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_empty_config_rejected() {
    let config = ScenarioConfig::new();
    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No scenarios"));
}

/// Roundtrip validation test - matches R's `weighted.mean()`
#[test]
fn test_r_weighted_mean_equivalence() {
    // This test validates against R's weighted.mean() function
    // R code:
    //   scenarios <- c("base", "bull", "bear")
    //   probabilities <- c(0.50, 0.30, 0.20)
    //   npv_values <- c(1250000, 2100000, -450000)
    //   ev <- weighted.mean(npv_values, probabilities)
    //   # Expected: 1165000

    let mut config = ScenarioConfig::new();
    config.add_scenario(
        "base",
        ScenarioDefinition::new(0.50).with_scalar("npv", 1_250_000.0),
    );
    config.add_scenario(
        "bull",
        ScenarioDefinition::new(0.30).with_scalar("npv", 2_100_000.0),
    );
    config.add_scenario(
        "bear",
        ScenarioDefinition::new(0.20).with_scalar("npv", -450_000.0),
    );

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "npv".to_string(),
        Variable::new("npv".to_string(), Some(0.0), None),
    );

    let engine = ScenarioEngine::new(config, model)
        .unwrap()
        .with_outputs(vec!["npv".to_string()]);

    let results = engine.run().unwrap();

    // R's weighted.mean result: 1015000
    let ev = results.expected_values.get("npv").unwrap();
    let r_expected = 0.20f64.mul_add(-450_000.0, 0.50f64.mul_add(1_250_000.0, 0.30 * 2_100_000.0));
    assert!(
        (ev - r_expected).abs() < 0.01,
        "Expected {r_expected} (R weighted.mean), got {ev}"
    );
    assert!(
        (ev - 1_165_000.0).abs() < 0.01,
        "Expected 1165000, got {ev}"
    );
}
