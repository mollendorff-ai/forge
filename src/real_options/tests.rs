//! Real Options Integration Tests

use super::*;

/// ADR-020 Example Test
#[test]
fn test_adr020_full_example() {
    // From ADR-020:
    // real_options:
    //   name: "Phased Factory Investment"
    //   method: binomial
    //   underlying:
    //     current_value: 10000000
    //     volatility: 0.30
    //     risk_free_rate: 0.05
    //     time_horizon: 3
    //   options:
    //     - type: defer
    //       name: "Wait up to 2 years"
    //       max_deferral: 2
    //       exercise_cost: 8000000
    //     - type: expand
    //       name: "Build Phase 2"
    //       expansion_factor: 1.5
    //       exercise_cost: 4000000
    //     - type: abandon
    //       name: "Sell assets"
    //       salvage_value: 3000000

    let config = RealOptionsConfig::new(
        "Phased Factory Investment",
        UnderlyingConfig::new(10_000_000.0, 0.30, 0.05, 3.0),
    )
    .with_method(ValuationMethod::Binomial)
    .with_option(OptionDefinition::defer(
        "Wait up to 2 years",
        2.0,
        8_000_000.0,
    ))
    .with_option(OptionDefinition::expand("Build Phase 2", 1.5, 4_000_000.0))
    .with_option(OptionDefinition::abandon("Sell assets", 3_000_000.0))
    .with_binomial_steps(100);

    let engine = RealOptionsEngine::new(config)
        .unwrap()
        .with_traditional_npv(-500_000.0);

    let result = engine.analyze().unwrap();

    // Validate structure
    assert_eq!(result.options.len(), 3);
    assert_eq!(result.traditional_npv, -500_000.0);

    // All options should have positive value
    for (name, opt) in &result.options {
        assert!(opt.value > 0.0, "Option {name} should have positive value");
    }

    // Project value with options should be better than traditional NPV
    assert!(
        result.project_value_with_options > result.traditional_npv,
        "Options should add value"
    );

    println!("=== ADR-020 Example Results ===");
    println!("Traditional NPV: ${:.0}", result.traditional_npv);
    println!("Options:");
    for (name, opt) in &result.options {
        println!("  {}: ${:.0}", name, opt.value);
    }
    println!("Total Option Value: ${:.0}", result.total_option_value);
    println!(
        "Project Value with Options: ${:.0}",
        result.project_value_with_options
    );
    println!("Decision: {}", result.decision);
}

/// Test that high volatility increases option value
#[test]
fn test_volatility_impact() {
    // Higher volatility should increase option value
    let low_vol = RealOptionsConfig::new("Low Vol", UnderlyingConfig::new(100.0, 0.10, 0.05, 1.0))
        .with_option(OptionDefinition::defer("Wait", 1.0, 100.0));

    let high_vol =
        RealOptionsConfig::new("High Vol", UnderlyingConfig::new(100.0, 0.40, 0.05, 1.0))
            .with_option(OptionDefinition::defer("Wait", 1.0, 100.0));

    let low_result = RealOptionsEngine::new(low_vol).unwrap().analyze().unwrap();
    let high_result = RealOptionsEngine::new(high_vol).unwrap().analyze().unwrap();

    assert!(
        high_result.total_option_value > low_result.total_option_value,
        "Higher volatility should increase option value"
    );
}

/// Test that longer time horizon increases option value
#[test]
fn test_time_horizon_impact() {
    let short_time = RealOptionsConfig::new("Short", UnderlyingConfig::new(100.0, 0.20, 0.05, 0.5))
        .with_option(OptionDefinition::defer("Wait", 0.5, 100.0));

    let long_time = RealOptionsConfig::new("Long", UnderlyingConfig::new(100.0, 0.20, 0.05, 2.0))
        .with_option(OptionDefinition::defer("Wait", 2.0, 100.0));

    let short_result = RealOptionsEngine::new(short_time)
        .unwrap()
        .analyze()
        .unwrap();
    let long_result = RealOptionsEngine::new(long_time)
        .unwrap()
        .analyze()
        .unwrap();

    assert!(
        long_result.total_option_value > short_result.total_option_value,
        "Longer time should increase option value"
    );
}

/// Test abandon option value
#[test]
fn test_abandon_option() {
    let config = RealOptionsConfig::new(
        "Abandon Test",
        UnderlyingConfig::new(1_000_000.0, 0.30, 0.05, 2.0),
    )
    .with_option(OptionDefinition::abandon("Exit", 400_000.0));

    let engine = RealOptionsEngine::new(config).unwrap();
    let result = engine.analyze().unwrap();

    let abandon = result.options.get("Exit").unwrap();
    assert!(abandon.value > 0.0, "Abandon option should have value");

    // Value should be less than salvage value
    assert!(
        abandon.value < 400_000.0,
        "Abandon value should be less than full salvage"
    );
}

/// Test expand option value
#[test]
fn test_expand_option() {
    let config = RealOptionsConfig::new(
        "Expand Test",
        UnderlyingConfig::new(1_000_000.0, 0.30, 0.05, 2.0),
    )
    .with_option(OptionDefinition::expand("Scale Up", 2.0, 500_000.0));

    let engine = RealOptionsEngine::new(config).unwrap();
    let result = engine.analyze().unwrap();

    let expand = result.options.get("Scale Up").unwrap();
    assert!(expand.value > 0.0, "Expand option should have value");
}

/// Test Black-Scholes vs Binomial convergence
#[test]
fn test_bs_binomial_convergence() {
    // For European-style options, BS and Binomial should converge
    let bs_config = RealOptionsConfig::new("BS", UnderlyingConfig::new(100.0, 0.20, 0.05, 1.0))
        .with_method(ValuationMethod::BlackScholes)
        .with_option(OptionDefinition::defer("Wait", 1.0, 100.0));

    let bin_config =
        RealOptionsConfig::new("Binomial", UnderlyingConfig::new(100.0, 0.20, 0.05, 1.0))
            .with_method(ValuationMethod::Binomial)
            .with_option(OptionDefinition::defer("Wait", 1.0, 100.0))
            .with_binomial_steps(200); // More steps for accuracy

    let bs_result = RealOptionsEngine::new(bs_config)
        .unwrap()
        .analyze()
        .unwrap();
    let bin_result = RealOptionsEngine::new(bin_config)
        .unwrap()
        .analyze()
        .unwrap();

    let bs_value = bs_result.options.get("Wait").unwrap().value;
    let bin_value = bin_result.options.get("Wait").unwrap().value;

    // Should be within 5% of each other
    let diff_pct = ((bs_value - bin_value) / bs_value).abs() * 100.0;
    assert!(
        diff_pct < 5.0,
        "BS ({bs_value}) and Binomial ({bin_value}) should converge, diff: {diff_pct:.1}%"
    );
}

/// QuantLib validation test
#[test]
fn test_quantlib_reference() {
    // Reference values from QuantLib for standard European call
    // S=100, K=100, r=5%, σ=20%, T=1
    // Call price ≈ 10.45

    let config = RealOptionsConfig::new(
        "QuantLib Test",
        UnderlyingConfig::new(100.0, 0.20, 0.05, 1.0),
    )
    .with_method(ValuationMethod::BlackScholes)
    .with_option(OptionDefinition::defer("Call", 1.0, 100.0));

    let result = RealOptionsEngine::new(config).unwrap().analyze().unwrap();
    let call_value = result.options.get("Call").unwrap().value;

    // Should be close to QuantLib reference (10.45)
    assert!(
        (call_value - 10.45).abs() < 0.5,
        "Call value ({call_value}) should match QuantLib reference (10.45)"
    );
}

/// Test multiple options interaction
#[test]
fn test_multiple_options() {
    let config = RealOptionsConfig::new(
        "Multi-Option",
        UnderlyingConfig::new(1_000_000.0, 0.25, 0.05, 3.0),
    )
    .with_option(OptionDefinition::defer("Wait", 1.0, 900_000.0))
    .with_option(OptionDefinition::expand("Scale", 1.5, 400_000.0))
    .with_option(OptionDefinition::abandon("Exit", 300_000.0))
    .with_option(OptionDefinition::contract("Shrink", 0.5, 100_000.0));

    let engine = RealOptionsEngine::new(config)
        .unwrap()
        .with_traditional_npv(-100_000.0);

    let result = engine.analyze().unwrap();

    // Should have all 4 options
    assert_eq!(result.options.len(), 4);

    // Total should be sum of individual (simplified - no interaction)
    let sum: f64 = result.options.values().map(|o| o.value).sum();
    assert!(
        (sum - result.total_option_value).abs() < 1.0,
        "Total should equal sum of options"
    );
}

/// Test decision recommendation
#[test]
fn test_decision_recommendation() {
    // Negative NPV but positive with options
    let config = RealOptionsConfig::new(
        "Decision Test",
        UnderlyingConfig::new(1_000_000.0, 0.30, 0.05, 2.0),
    )
    .with_option(OptionDefinition::defer("Wait", 2.0, 800_000.0))
    .with_option(OptionDefinition::abandon("Exit", 500_000.0));

    let engine = RealOptionsEngine::new(config)
        .unwrap()
        .with_traditional_npv(-50_000.0);

    let result = engine.analyze().unwrap();

    if result.project_value_with_options > 0.0 {
        assert!(result.decision.contains("ACCEPT"));
    } else {
        assert!(result.decision.contains("REJECT"));
    }

    // Should have recommendation text
    assert!(!result.recommendation.is_empty());
}
