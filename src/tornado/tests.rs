//! Tornado Diagram Integration Tests

use super::*;
use crate::types::{ParsedModel, Variable};

fn create_npv_model() -> ParsedModel {
    let mut model = ParsedModel::new();

    // Inputs
    model.scalars.insert(
        "revenue_growth".to_string(),
        Variable::new("revenue_growth".to_string(), Some(0.05), None),
    );
    model.scalars.insert(
        "discount_rate".to_string(),
        Variable::new("discount_rate".to_string(), Some(0.10), None),
    );
    model.scalars.insert(
        "operating_margin".to_string(),
        Variable::new("operating_margin".to_string(), Some(0.20), None),
    );
    model.scalars.insert(
        "tax_rate".to_string(),
        Variable::new("tax_rate".to_string(), Some(0.25), None),
    );
    model.scalars.insert(
        "initial_investment".to_string(),
        Variable::new("initial_investment".to_string(), Some(1_000_000.0), None),
    );

    // Intermediate calculations
    model.scalars.insert(
        "base_revenue".to_string(),
        Variable::new("base_revenue".to_string(), Some(500_000.0), None),
    );
    model.scalars.insert(
        "year1_revenue".to_string(),
        Variable::new(
            "year1_revenue".to_string(),
            None,
            Some("=base_revenue * (1 + revenue_growth)".to_string()),
        ),
    );
    model.scalars.insert(
        "operating_income".to_string(),
        Variable::new(
            "operating_income".to_string(),
            None,
            Some("=year1_revenue * operating_margin".to_string()),
        ),
    );
    model.scalars.insert(
        "after_tax_income".to_string(),
        Variable::new(
            "after_tax_income".to_string(),
            None,
            Some("=operating_income * (1 - tax_rate)".to_string()),
        ),
    );

    // Simplified NPV (just year 1 discounted)
    model.scalars.insert(
        "npv".to_string(),
        Variable::new(
            "npv".to_string(),
            None,
            Some("=after_tax_income / (1 + discount_rate) - initial_investment".to_string()),
        ),
    );

    model
}

#[test]
fn test_full_tornado_workflow() {
    let model = create_npv_model();

    let config = TornadoConfig::new("npv")
        .with_input(InputRange::new("revenue_growth", 0.02, 0.08))
        .with_input(InputRange::new("discount_rate", 0.08, 0.12))
        .with_input(InputRange::new("operating_margin", 0.15, 0.25))
        .with_input(InputRange::new("tax_rate", 0.20, 0.30));

    let engine = TornadoEngine::new(config, model).unwrap();
    let result = engine.analyze().unwrap();

    // Should have 4 sensitivity bars
    assert_eq!(result.bars.len(), 4);

    // All bars should have non-zero swing
    for bar in &result.bars {
        assert!(
            bar.abs_swing > 0.0,
            "Bar {} should have non-zero swing",
            bar.input_name
        );
    }

    // Print the tornado diagram
    println!("{}", result.to_ascii());
}

#[test]
fn test_swing_direction() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "x".to_string(),
        Variable::new("x".to_string(), Some(10.0), None),
    );
    model.scalars.insert(
        "y".to_string(),
        Variable::new("y".to_string(), None, Some("=x * 2".to_string())),
    );

    let config = TornadoConfig::new("y").with_input(InputRange::new("x", 5.0, 15.0));

    let engine = TornadoEngine::new(config, model).unwrap();
    let result = engine.analyze().unwrap();

    let bar = &result.bars[0];

    // y = x * 2, so:
    // At x=5: y=10
    // At x=15: y=30
    assert!(
        (bar.output_at_low - 10.0).abs() < 0.01,
        "Output at low should be 10"
    );
    assert!(
        (bar.output_at_high - 30.0).abs() < 0.01,
        "Output at high should be 30"
    );
    assert!(
        bar.swing > 0.0,
        "Positive relationship should have positive swing"
    );
}

#[test]
fn test_inverse_relationship() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "rate".to_string(),
        Variable::new("rate".to_string(), Some(0.10), None),
    );
    model.scalars.insert(
        "value".to_string(),
        Variable::new(
            "value".to_string(),
            None,
            Some("=100 / (1 + rate)".to_string()),
        ),
    );

    let config = TornadoConfig::new("value").with_input(InputRange::new("rate", 0.05, 0.15));

    let engine = TornadoEngine::new(config, model).unwrap();
    let result = engine.analyze().unwrap();

    let bar = &result.bars[0];

    // Higher rate = lower value (inverse relationship)
    assert!(
        bar.output_at_low > bar.output_at_high,
        "Higher rate should give lower value"
    );
    assert!(
        bar.swing < 0.0,
        "Inverse relationship should have negative swing"
    );
}

#[test]
fn test_r_sensitivity_equivalence() {
    // This test validates against R's sensitivity package concepts
    // One-at-a-time (OAT) sensitivity analysis

    let mut model = ParsedModel::new();
    model.scalars.insert(
        "a".to_string(),
        Variable::new("a".to_string(), Some(1.0), None),
    );
    model.scalars.insert(
        "b".to_string(),
        Variable::new("b".to_string(), Some(2.0), None),
    );
    // y = a + 2*b (linear model)
    model.scalars.insert(
        "y".to_string(),
        Variable::new("y".to_string(), None, Some("=a + 2 * b".to_string())),
    );

    let config = TornadoConfig::new("y")
        .with_input(InputRange::new("a", 0.5, 1.5))
        .with_input(InputRange::new("b", 1.5, 2.5));

    let engine = TornadoEngine::new(config, model).unwrap();
    let result = engine.analyze().unwrap();

    // For y = a + 2*b:
    // Sensitivity to a: dy/da = 1, swing = 1 * (1.5 - 0.5) = 1
    // Sensitivity to b: dy/db = 2, swing = 2 * (2.5 - 1.5) = 2
    // So b should have higher impact

    assert_eq!(
        result.bars[0].input_name, "b",
        "b should have highest impact"
    );
    assert!(
        (result.bars[0].abs_swing - 2.0).abs() < 0.01,
        "b swing should be 2"
    );
    assert!(
        (result.bars[1].abs_swing - 1.0).abs() < 0.01,
        "a swing should be 1"
    );
}

#[test]
fn test_variance_explained() {
    let model = create_npv_model();

    let config = TornadoConfig::new("npv")
        .with_input(InputRange::new("revenue_growth", 0.02, 0.08))
        .with_input(InputRange::new("discount_rate", 0.08, 0.12))
        .with_input(InputRange::new("operating_margin", 0.15, 0.25))
        .with_input(InputRange::new("tax_rate", 0.20, 0.30));

    let engine = TornadoEngine::new(config, model).unwrap();
    let result = engine.analyze().unwrap();

    // Top 2 should explain significant variance
    let top_2_pct = result.variance_explained_by_top(2);
    assert!(
        (0.0..=100.0).contains(&top_2_pct),
        "Variance explained should be 0-100%"
    );

    // Top 4 (all) should explain 100%
    let top_4_pct = result.variance_explained_by_top(4);
    assert!(
        (top_4_pct - 100.0).abs() < 0.01,
        "All inputs should explain 100% variance"
    );
}

#[test]
fn test_single_input() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "x".to_string(),
        Variable::new("x".to_string(), Some(100.0), None),
    );
    model.scalars.insert(
        "y".to_string(),
        Variable::new("y".to_string(), None, Some("=x * 1.1".to_string())),
    );

    let config = TornadoConfig::new("y").with_input(InputRange::new("x", 90.0, 110.0));

    let engine = TornadoEngine::new(config, model).unwrap();
    let result = engine.analyze().unwrap();

    assert_eq!(result.bars.len(), 1);
    assert_eq!(result.bars[0].input_name, "x");
}

#[test]
fn test_with_base_values() {
    let mut model = ParsedModel::new();
    model.scalars.insert(
        "x".to_string(),
        Variable::new("x".to_string(), Some(100.0), None),
    );
    model.scalars.insert(
        "y".to_string(),
        Variable::new("y".to_string(), None, Some("=x".to_string())),
    );

    let config =
        TornadoConfig::new("y").with_input(InputRange::new("x", 80.0, 120.0).with_base(100.0));

    let engine = TornadoEngine::new(config, model).unwrap();
    let result = engine.analyze().unwrap();

    // Base value should be 100
    assert!(
        (result.base_value - 100.0).abs() < 0.01,
        "Base value should be 100"
    );
}
