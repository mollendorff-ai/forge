//! Decision Trees Integration Tests

// Financial math: exact float comparison validated against Excel/Gnumeric/R
#![allow(clippy::float_cmp)]

use super::*;

/// ADR-019 Example: R&D Investment Decision
#[test]
fn test_adr019_example() {
    // From ADR-019:
    // [Decision] Invest in R&D?
    // ├── Yes ($2M cost)
    // │   └── [Chance] Technology works?
    // │       ├── Success (60%): [Decision] License or Manufacture?
    // │       │   ├── License: NPV = $5M
    // │       │   └── Manufacture: NPV = $8M (but $3M more investment)
    // │       └── Failure (40%): NPV = -$2M
    // └── No
    //     └── NPV = $0
    //
    // Expected Value Calculation:
    // E[Invest] = 0.6 × max($5M, $8M-$3M) + 0.4 × (-$2M) - $2M
    //          = 0.6 × $5M + 0.4 × (-$2M) - $2M
    //          = $3M - $0.8M - $2M = $0.2M
    // E[Don't Invest] = $0
    // Decision: Invest (EV = $0.2M > $0)

    let config = DecisionTreeConfig::new("R&D Investment")
        .with_root(
            Node::decision("Invest in R&D?")
                .with_branch(
                    "invest",
                    Branch::continuation("tech_outcome").with_cost(2_000_000.0),
                )
                .with_branch("dont_invest", Branch::terminal(0.0)),
        )
        .with_node(
            "tech_outcome",
            Node::chance("Technology works?")
                .with_branch(
                    "success",
                    Branch::continuation("commercialize").with_probability(0.60),
                )
                .with_branch(
                    "failure",
                    Branch::terminal(-2_000_000.0).with_probability(0.40),
                ),
        )
        .with_node(
            "commercialize",
            Node::decision("How to commercialize?")
                .with_branch("license", Branch::terminal(5_000_000.0))
                .with_branch(
                    "manufacture",
                    Branch::terminal(8_000_000.0).with_cost(3_000_000.0),
                ),
        );

    let engine = DecisionTreeEngine::new(config).unwrap();
    let result = engine.analyze().unwrap();

    // Verify the ADR-019 expected calculations
    // commercialize_decision: max($5M, $8M-$3M) = $5M (license)
    let commercialize = result.node_results.get("commercialize").unwrap();
    assert!(
        (commercialize.expected_value - 5_000_000.0).abs() < 0.01,
        "Commercialize EV should be $5M"
    );
    assert_eq!(
        commercialize.optimal_choice,
        Some("license".to_string()),
        "Should choose license"
    );

    // tech_outcome: 0.6 × $5M + 0.4 × (-$2M) = $3M - $0.8M = $2.2M
    let tech = result.node_results.get("tech_outcome").unwrap();
    assert!(
        (tech.expected_value - 2_200_000.0).abs() < 0.01,
        "Tech outcome EV should be $2.2M, got {}",
        tech.expected_value
    );

    // root: max($2.2M - $2M, $0) = $0.2M (invest)
    assert!(
        (result.root_expected_value - 200_000.0).abs() < 0.01,
        "Root EV should be $0.2M, got {}",
        result.root_expected_value
    );

    // Should recommend invest
    assert_eq!(
        result.decision_policy.get("root"),
        Some(&"invest".to_string()),
        "Should recommend investing"
    );
}

/// Test a three-branch decision
#[test]
fn test_three_way_decision() {
    let config = DecisionTreeConfig::new("Three Options").with_root(
        Node::decision("Choose strategy")
            .with_branch("conservative", Branch::terminal(100_000.0))
            .with_branch("moderate", Branch::terminal(150_000.0))
            .with_branch("aggressive", Branch::terminal(200_000.0)),
    );

    let engine = DecisionTreeEngine::new(config).unwrap();
    let result = engine.analyze().unwrap();

    // Should choose aggressive (highest value)
    assert_eq!(result.root_expected_value, 200_000.0);
    assert_eq!(
        result.decision_policy.get("root"),
        Some(&"aggressive".to_string())
    );
}

/// Test nested chance nodes
#[test]
fn test_nested_chance_nodes() {
    let config = DecisionTreeConfig::new("Nested Chance")
        .with_root(
            Node::chance("First flip")
                .with_branch(
                    "heads",
                    Branch::continuation("second_flip").with_probability(0.5),
                )
                .with_branch("tails", Branch::terminal(0.0).with_probability(0.5)),
        )
        .with_node(
            "second_flip",
            Node::chance("Second flip")
                .with_branch("heads", Branch::terminal(100.0).with_probability(0.5))
                .with_branch("tails", Branch::terminal(50.0).with_probability(0.5)),
        );

    let engine = DecisionTreeEngine::new(config).unwrap();
    let result = engine.analyze().unwrap();

    // Second flip EV = 0.5 * 100 + 0.5 * 50 = 75
    // First flip EV = 0.5 * 75 + 0.5 * 0 = 37.5
    assert!(
        (result.root_expected_value - 37.5).abs() < 0.01,
        "Expected 37.5, got {}",
        result.root_expected_value
    );
}

/// Test risk profile calculation
#[test]
fn test_risk_profile() {
    let config = DecisionTreeConfig::new("Risk Test").with_root(
        Node::chance("Outcome")
            .with_branch("best", Branch::terminal(1000.0).with_probability(0.3))
            .with_branch("middle", Branch::terminal(500.0).with_probability(0.5))
            .with_branch("worst", Branch::terminal(-200.0).with_probability(0.2)),
    );

    let engine = DecisionTreeEngine::new(config).unwrap();
    let result = engine.analyze().unwrap();

    assert_eq!(result.risk_profile.best_case, 1000.0);
    assert_eq!(result.risk_profile.worst_case, -200.0);

    // Probability positive = 0.3 + 0.5 = 0.8
    assert!(
        (result.risk_profile.probability_positive - 0.8).abs() < 0.01,
        "Expected 0.8 prob positive, got {}",
        result.risk_profile.probability_positive
    );
}

/// Test JSON export
#[test]
fn test_json_export() {
    let config = DecisionTreeConfig::new("Export Test").with_root(
        Node::decision("Choose")
            .with_branch("a", Branch::terminal(100.0))
            .with_branch("b", Branch::terminal(50.0)),
    );

    let engine = DecisionTreeEngine::new(config).unwrap();
    let result = engine.analyze().unwrap();
    let json = result.to_json().unwrap();

    assert!(json.contains("\"root_expected_value\""));
    assert!(json.contains("\"decision_policy\""));
    assert!(json.contains("\"risk_profile\""));
}
