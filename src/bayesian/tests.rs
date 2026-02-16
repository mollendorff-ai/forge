//! Bayesian Networks Integration Tests

use super::*;
use std::collections::HashMap;

fn create_credit_risk_network() -> BayesianConfig {
    BayesianConfig::new("Credit Risk")
        .with_node(
            "economic_conditions",
            BayesianNode::discrete(vec!["good", "neutral", "bad"]).with_prior(vec![0.3, 0.5, 0.2]),
        )
        .with_node(
            "company_revenue",
            BayesianNode::discrete(vec!["high", "medium", "low"])
                .with_parents(vec!["economic_conditions"])
                .with_cpt_entry("good", vec![0.6, 0.3, 0.1])
                .with_cpt_entry("neutral", vec![0.3, 0.5, 0.2])
                .with_cpt_entry("bad", vec![0.1, 0.3, 0.6]),
        )
        .with_node(
            "default_probability",
            BayesianNode::discrete(vec!["low", "medium", "high"])
                .with_parents(vec!["company_revenue"])
                .with_cpt_entry("high", vec![0.8, 0.15, 0.05])
                .with_cpt_entry("medium", vec![0.4, 0.4, 0.2])
                .with_cpt_entry("low", vec![0.1, 0.3, 0.6]),
        )
}

/// ADR Example: Credit Risk Model
#[test]
fn test_adr_credit_risk_example() {
    // From the ADR:
    // [Economic Conditions] → [Company Revenue] → [Default Probability]

    let config = create_credit_risk_network();
    let engine = BayesianEngine::new(config).unwrap();

    // Query prior probabilities
    let econ = engine.query("economic_conditions").unwrap();
    assert!((econ.probabilities[0] - 0.3).abs() < 0.01); // good
    assert!((econ.probabilities[1] - 0.5).abs() < 0.01); // neutral
    assert!((econ.probabilities[2] - 0.2).abs() < 0.01); // bad

    // Query with evidence: if economy is bad, what's default probability?
    let mut evidence = HashMap::new();
    evidence.insert("economic_conditions".to_string(), "bad");

    let default = engine
        .query_with_evidence("default_probability", &evidence)
        .unwrap();

    // With bad economy:
    // P(revenue=high|bad) = 0.1, P(revenue=medium|bad) = 0.3, P(revenue=low|bad) = 0.6
    // P(default=high|revenue=high) = 0.05
    // P(default=high|revenue=medium) = 0.2
    // P(default=high|revenue=low) = 0.6
    // P(default=high|bad) = 0.1*0.05 + 0.3*0.2 + 0.6*0.6 = 0.005 + 0.06 + 0.36 = 0.425

    let p_high_default = default.probabilities[2]; // high is index 2
    assert!(
        p_high_default > 0.3,
        "P(default=high | economy=bad) should be high: {p_high_default}"
    );

    println!("P(default=high | economy=bad) = {p_high_default:.3}");
}

/// Test the chain A -> B -> C
#[test]
fn test_chain_network() {
    let config = BayesianConfig::new("Chain")
        .with_node(
            "A",
            BayesianNode::discrete(vec!["a0", "a1"]).with_prior(vec![0.6, 0.4]),
        )
        .with_node(
            "B",
            BayesianNode::discrete(vec!["b0", "b1"])
                .with_parents(vec!["A"])
                .with_cpt_entry("a0", vec![0.9, 0.1])
                .with_cpt_entry("a1", vec![0.2, 0.8]),
        )
        .with_node(
            "C",
            BayesianNode::discrete(vec!["c0", "c1"])
                .with_parents(vec!["B"])
                .with_cpt_entry("b0", vec![0.7, 0.3])
                .with_cpt_entry("b1", vec![0.1, 0.9]),
        );

    let engine = BayesianEngine::new(config).unwrap();

    // P(B=b1) = P(B=b1|A=a0)*P(A=a0) + P(B=b1|A=a1)*P(A=a1)
    //         = 0.1 * 0.6 + 0.8 * 0.4 = 0.06 + 0.32 = 0.38
    let b_result = engine.query("B").unwrap();
    assert!(
        (b_result.probabilities[1] - 0.38).abs() < 0.01,
        "P(B=b1) should be 0.38, got {}",
        b_result.probabilities[1]
    );
}

/// Test conditional independence (A _||_ C | B)
#[test]
// Standard Bayesian notation: evidence_b/evidence_ab distinguish conditioning sets
#[allow(clippy::similar_names)]
fn test_conditional_independence() {
    let config = BayesianConfig::new("CI Test")
        .with_node(
            "A",
            BayesianNode::discrete(vec!["a0", "a1"]).with_prior(vec![0.5, 0.5]),
        )
        .with_node(
            "B",
            BayesianNode::discrete(vec!["b0", "b1"])
                .with_parents(vec!["A"])
                .with_cpt_entry("a0", vec![0.8, 0.2])
                .with_cpt_entry("a1", vec![0.2, 0.8]),
        )
        .with_node(
            "C",
            BayesianNode::discrete(vec!["c0", "c1"])
                .with_parents(vec!["B"])
                .with_cpt_entry("b0", vec![0.9, 0.1])
                .with_cpt_entry("b1", vec![0.1, 0.9]),
        );

    let engine = BayesianEngine::new(config).unwrap();

    // Given B=b1, C should be independent of A
    let mut evidence_b = HashMap::new();
    evidence_b.insert("B".to_string(), "b1");

    let c_given_b = engine.query_with_evidence("C", &evidence_b).unwrap();

    // Also query with A evidence
    let mut evidence_ab = HashMap::new();
    evidence_ab.insert("A".to_string(), "a0");
    evidence_ab.insert("B".to_string(), "b1");

    let c_given_ab = engine.query_with_evidence("C", &evidence_ab).unwrap();

    // P(C|B) should equal P(C|A,B) due to conditional independence
    assert!(
        (c_given_b.probabilities[0] - c_given_ab.probabilities[0]).abs() < 0.05,
        "Conditional independence should hold: P(C|B)={:.3} vs P(C|A,B)={:.3}",
        c_given_b.probabilities[0],
        c_given_ab.probabilities[0]
    );
}

/// pgmpy validation test
#[test]
fn test_pgmpy_equivalence() {
    // This test validates against pgmpy (Python)
    // from pgmpy.models import BayesianNetwork
    // from pgmpy.factors.discrete import TabularCPD
    // from pgmpy.inference import VariableElimination
    //
    // model = BayesianNetwork([('D', 'I'), ('I', 'G')])
    // cpd_d = TabularCPD('D', 2, [[0.6], [0.4]])
    // cpd_i = TabularCPD('I', 2, [[0.7, 0.3], [0.3, 0.7]], evidence=['D'], evidence_card=[2])
    // cpd_g = TabularCPD('G', 3, [[0.3, 0.05], [0.4, 0.25], [0.3, 0.7]], evidence=['I'], evidence_card=[2])
    // model.add_cpds(cpd_d, cpd_i, cpd_g)
    //
    // infer = VariableElimination(model)
    // print(infer.query(['G']))

    let config = BayesianConfig::new("Student Network")
        .with_node(
            "difficulty",
            BayesianNode::discrete(vec!["easy", "hard"]).with_prior(vec![0.6, 0.4]),
        )
        .with_node(
            "intelligence",
            BayesianNode::discrete(vec!["low", "high"])
                .with_parents(vec!["difficulty"])
                .with_cpt_entry("easy", vec![0.7, 0.3])
                .with_cpt_entry("hard", vec![0.3, 0.7]),
        )
        .with_node(
            "grade",
            BayesianNode::discrete(vec!["A", "B", "C"])
                .with_parents(vec!["intelligence"])
                .with_cpt_entry("low", vec![0.3, 0.4, 0.3])
                .with_cpt_entry("high", vec![0.05, 0.25, 0.7]),
        );

    let engine = BayesianEngine::new(config).unwrap();

    // Query marginal P(Grade)
    let grade = engine.query("grade").unwrap();

    // P(I=low) = P(I=low|D=easy)*P(D=easy) + P(I=low|D=hard)*P(D=hard)
    //          = 0.7*0.6 + 0.3*0.4 = 0.42 + 0.12 = 0.54
    // P(I=high) = 0.46
    //
    // P(G=A) = P(G=A|I=low)*P(I=low) + P(G=A|I=high)*P(I=high)
    //        = 0.3*0.54 + 0.05*0.46 = 0.162 + 0.023 = 0.185

    let p_a = grade.probabilities[0];
    assert!(
        (p_a - 0.185).abs() < 0.02,
        "P(Grade=A) should be ~0.185, got {p_a}"
    );

    println!("P(Grade) = {:?}", grade.probabilities);
}

/// Test probability sum to 1
#[test]
fn test_probability_normalization() {
    let config = create_credit_risk_network();
    let engine = BayesianEngine::new(config).unwrap();

    let result = engine.query_all().unwrap();

    for (name, var_result) in &result.queries {
        let sum: f64 = var_result.probabilities.iter().sum();
        assert!(
            (sum - 1.0).abs() < 0.001,
            "Variable {name} probabilities should sum to 1.0, got {sum}"
        );
    }
}

/// Test with evidence changes posterior
#[test]
fn test_evidence_effect() {
    let config = create_credit_risk_network();
    let engine = BayesianEngine::new(config).unwrap();

    // Prior P(default=high)
    let prior = engine.query("default_probability").unwrap();
    let p_high_prior = prior.probabilities[2];

    // Posterior P(default=high | economy=good)
    let mut evidence = HashMap::new();
    evidence.insert("economic_conditions".to_string(), "good");
    let posterior = engine
        .query_with_evidence("default_probability", &evidence)
        .unwrap();
    let p_high_posterior = posterior.probabilities[2];

    // Good economy should reduce default probability
    assert!(
        p_high_posterior < p_high_prior,
        "Good economy should reduce default risk: prior={p_high_prior}, posterior={p_high_posterior}"
    );
}

/// Test JSON export with evidence
#[test]
fn test_json_with_evidence() {
    let config = create_credit_risk_network();
    let engine = BayesianEngine::new(config).unwrap();

    let mut evidence = HashMap::new();
    evidence.insert("economic_conditions".to_string(), "bad");

    let result = engine.query_all_with_evidence(&evidence).unwrap();
    let json = result.to_json().unwrap();

    assert!(json.contains("\"evidence\""));
    assert!(json.contains("\"bad\""));
}
