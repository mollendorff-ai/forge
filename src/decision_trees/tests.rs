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

/// The tree in examples/decision-tree.yaml
fn rnd_example() -> DecisionTreeConfig {
    DecisionTreeConfig::new("R&D")
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
            Node::chance("Technology")
                .with_branch(
                    "success",
                    Branch::continuation("commercialize").with_probability(0.6),
                )
                .with_branch(
                    "failure",
                    Branch::terminal(-2_000_000.0).with_probability(0.4),
                ),
        )
        .with_node(
            "commercialize",
            Node::decision("Strategy")
                .with_branch(
                    "scale",
                    Branch::continuation("scale_market").with_cost(5_000_000.0),
                )
                .with_branch(
                    "pilot",
                    Branch::continuation("pilot_market").with_cost(500_000.0),
                ),
        )
        .with_node(
            "scale_market",
            Node::chance("Scale market")
                .with_branch(
                    "success",
                    Branch::terminal(15_000_000.0).with_probability(0.7),
                )
                .with_branch(
                    "failure",
                    Branch::terminal(2_000_000.0).with_probability(0.3),
                ),
        )
        .with_node(
            "pilot_market",
            Node::chance("Pilot market")
                .with_branch(
                    "success",
                    Branch::terminal(3_000_000.0).with_probability(0.5),
                )
                .with_branch("failure", Branch::terminal(500_000.0).with_probability(0.5)),
        )
}

/// Worked by hand: invest then scale gives +$8M (0.6 × 0.7 = 0.42),
/// -$5M (0.6 × 0.3 = 0.18) or -$4M (0.4). The old calculation summed local
/// probabilities of every terminal branch, including rejected ones, and
/// reported 100%, a best case of $15M and a worst case of -$2M.
#[test]
fn test_risk_profile_follows_optimal_policy_and_path_probabilities() {
    let result = DecisionTreeEngine::new(rnd_example())
        .unwrap()
        .analyze()
        .unwrap();
    let risk = &result.risk_profile;

    assert!((risk.probability_positive - 0.42).abs() < 1e-12, "{risk:?}");
    assert!((risk.best_case - 8_000_000.0).abs() < 1e-6, "{risk:?}");
    assert!((risk.worst_case + 5_000_000.0).abs() < 1e-6, "{risk:?}");

    let outcomes: Vec<(f64, f64)> = risk
        .outcomes
        .iter()
        .map(|o| (o.value, o.probability))
        .collect();
    assert_eq!(outcomes.len(), 3, "{outcomes:?}");
    for (value, probability) in [(8e6, 0.42), (-4e6, 0.4), (-5e6, 0.18)] {
        assert!(
            outcomes
                .iter()
                .any(|(v, p)| (v - value).abs() < 1e-6 && (p - probability).abs() < 1e-12),
            "missing {value} at {probability}: {outcomes:?}"
        );
    }

    // The outcomes must add up to the tree's own expected value
    let total_probability: f64 = outcomes.iter().map(|(_, p)| p).sum();
    let expected: f64 = outcomes.iter().map(|(v, p)| v * p).sum();
    assert!((total_probability - 1.0).abs() < 1e-12);
    assert!((expected - result.root_expected_value).abs() < 1e-6);
}

#[test]
fn test_risk_profile_ignores_rejected_branches() {
    // The gamble has the best single outcome but a lower expected value
    let config = DecisionTreeConfig::new("Safe or gamble")
        .with_root(
            Node::decision("Choose")
                .with_branch("safe", Branch::terminal(50.0))
                .with_branch("gamble", Branch::continuation("draw")),
        )
        .with_node(
            "draw",
            Node::chance("Draw")
                .with_branch("win", Branch::terminal(1000.0).with_probability(0.01))
                .with_branch("lose", Branch::terminal(-10.0).with_probability(0.99)),
        );
    let result = DecisionTreeEngine::new(config).unwrap().analyze().unwrap();
    let risk = &result.risk_profile;
    assert_eq!(result.decision_policy["root"], "safe");
    assert_eq!(risk.best_case, 50.0);
    assert_eq!(risk.worst_case, 50.0);
    assert_eq!(risk.probability_positive, 1.0);
    assert_eq!(risk.outcomes.len(), 1);
}

#[test]
fn test_risk_profile_multiplies_nested_chance_and_adds_costs() {
    let config = DecisionTreeConfig::new("Nested")
        .with_root(
            Node::chance("First")
                .with_branch(
                    "up",
                    Branch::continuation("second")
                        .with_probability(0.5)
                        .with_cost(30.0),
                )
                .with_branch("down", Branch::terminal(-20.0).with_probability(0.5)),
        )
        .with_node(
            "second",
            Node::chance("Second")
                .with_branch("up", Branch::terminal(100.0).with_probability(0.5))
                .with_branch("down", Branch::terminal(20.0).with_probability(0.5)),
        );
    let risk = DecisionTreeEngine::new(config)
        .unwrap()
        .analyze()
        .unwrap()
        .risk_profile;
    // up/up: 100 - 30 = 70 at 0.25; up/down: 20 - 30 = -10 at 0.25; down: -20 at 0.5
    assert!((risk.probability_positive - 0.25).abs() < 1e-12, "{risk:?}");
    assert_eq!(risk.best_case, 70.0);
    assert_eq!(risk.worst_case, -20.0);
}
