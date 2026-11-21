//! Bayesian Network Engine
//!
//! High-level interface for Bayesian network inference.

use super::config::BayesianConfig;
use super::inference::BeliefPropagation;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Query result for a variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableResult {
    /// Variable name
    pub name: String,
    /// State names
    pub states: Vec<String>,
    /// Probability for each state
    pub probabilities: Vec<f64>,
    /// Most likely state
    pub most_likely: String,
    /// Probability of most likely state
    pub max_probability: f64,
}

impl VariableResult {
    /// Get probability for a specific state
    #[must_use]
    pub fn get_probability(&self, state: &str) -> Option<f64> {
        self.states
            .iter()
            .position(|s| s == state)
            .map(|idx| self.probabilities[idx])
    }
}

/// Complete Bayesian analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BayesianResult {
    /// Network name
    pub name: String,
    /// Query results
    pub queries: BTreeMap<String, VariableResult>,
    /// Evidence used
    pub evidence: BTreeMap<String, String>,
}

impl BayesianResult {
    /// Export results to YAML format
    #[must_use]
    pub fn to_yaml(&self) -> String {
        yaml_or_comment(yaml_serde::to_string(&crate::core::precision::Rounded(
            self,
        )))
    }

    /// Export results to JSON format
    ///
    /// # Errors
    ///
    /// Returns an error if JSON serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&crate::core::precision::Rounded(self))
    }
}

/// The YAML text, or a comment saying it could not be written
fn yaml_or_comment(yaml: Result<String, yaml_serde::Error>) -> String {
    yaml.unwrap_or_else(|e| format!("# Error serializing results: {e}"))
}

/// Bayesian Network Engine
pub struct BayesianEngine {
    config: BayesianConfig,
    bp: BeliefPropagation,
}

impl BayesianEngine {
    /// Create a new Bayesian engine
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn new(config: BayesianConfig) -> Result<Self, String> {
        let bp = BeliefPropagation::new(config.clone())?;
        Ok(Self { config, bp })
    }

    /// Query the marginal probability of a variable
    ///
    /// # Errors
    ///
    /// Returns an error if the target variable is not found in the network.
    pub fn query(&self, target: &str) -> Result<VariableResult, String> {
        self.query_with_evidence(target, &HashMap::new())
    }

    /// Query with evidence
    ///
    /// # Errors
    ///
    /// Returns an error if the target or evidence variables are not found.
    pub fn query_with_evidence(
        &self,
        target: &str,
        evidence: &HashMap<String, &str>,
    ) -> Result<VariableResult, String> {
        // Convert evidence from state names to indices
        let mut evidence_indices = HashMap::new();
        for (var, state) in evidence {
            let node = self
                .config
                .nodes
                .get(var.as_str())
                .ok_or_else(|| format!("Evidence variable '{var}' not found"))?;

            let idx = node
                .states
                .iter()
                .position(|s| s == state)
                .ok_or_else(|| format!("State '{state}' not found for variable '{var}'"))?;

            evidence_indices.insert(var.clone(), idx);
        }

        let node = self
            .config
            .nodes
            .get(target)
            .ok_or_else(|| format!("Variable '{target}' not found in network"))?;
        let probs = self.bp.query_with_evidence(target, &evidence_indices)?;

        let (max_idx, max_prob) = probs
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map_or((0, 0.0), |(i, p)| (i, *p));

        Ok(VariableResult {
            name: target.to_string(),
            states: node.states.clone(),
            probabilities: probs,
            most_likely: node.states.get(max_idx).cloned().unwrap_or_default(),
            max_probability: max_prob,
        })
    }

    /// Query all variables
    ///
    /// # Errors
    ///
    /// Returns an error if querying any variable fails.
    pub fn query_all(&self) -> Result<BayesianResult, String> {
        let mut queries = BTreeMap::new();

        for name in self.config.nodes.keys() {
            let result = self.query(name)?;
            queries.insert(name.clone(), result);
        }

        Ok(BayesianResult {
            name: self.config.name.clone(),
            queries,
            evidence: BTreeMap::new(),
        })
    }

    /// Query all variables with evidence
    ///
    /// # Errors
    ///
    /// Returns an error if querying any variable fails or evidence is invalid.
    pub fn query_all_with_evidence(
        &self,
        evidence: &HashMap<String, &str>,
    ) -> Result<BayesianResult, String> {
        let mut queries = BTreeMap::new();

        for name in self.config.nodes.keys() {
            // Skip evidence variables (their probability is deterministic)
            if evidence.contains_key(name) {
                continue;
            }

            let result = self.query_with_evidence(name, evidence)?;
            queries.insert(name.clone(), result);
        }

        // Convert evidence to string map
        let evidence_str: BTreeMap<String, String> = evidence
            .iter()
            .map(|(k, v)| (k.clone(), (*v).to_string()))
            .collect();

        Ok(BayesianResult {
            name: self.config.name.clone(),
            queries,
            evidence: evidence_str,
        })
    }

    /// Each variable's own most likely state, given no evidence. This is not the
    /// most probable explanation (MPE) of the whole network, which is the one
    /// combination of states with the highest joint probability, and can differ.
    ///
    /// # Errors
    ///
    /// Returns an error if querying any variable fails.
    pub fn most_likely_states(&self) -> Result<BTreeMap<String, String>, String> {
        let mut explanation = BTreeMap::new();

        for name in self.config.nodes.keys() {
            let result = self.query(name)?;
            explanation.insert(name.clone(), result.most_likely);
        }

        Ok(explanation)
    }

    /// Get the configuration
    #[must_use]
    pub const fn config(&self) -> &BayesianConfig {
        &self.config
    }
}

#[cfg(test)]
mod engine_tests {
    use super::*;
    use crate::bayesian::config::BayesianNode;

    fn create_credit_risk_network() -> BayesianConfig {
        BayesianConfig::new("Credit Risk")
            .with_node(
                "economic_conditions",
                BayesianNode::discrete(vec!["good", "neutral", "bad"])
                    .with_prior(vec![0.3, 0.5, 0.2]),
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

    /// `examples/bagel/slow_week.yaml`, its nodes added in the order given
    fn slow_week(order: [usize; 3]) -> BayesianConfig {
        let nodes = [
            (
                "weather",
                BayesianNode::discrete(vec!["rainy", "dry"]).with_prior(vec![0.3, 0.7]),
            ),
            (
                "roadwork",
                BayesianNode::discrete(vec!["yes", "no"]).with_prior(vec![0.2, 0.8]),
            ),
            (
                "sales",
                BayesianNode::discrete(vec!["slow", "normal"])
                    .with_parents(vec!["weather", "roadwork"])
                    .with_cpt_entry("rainy, yes", vec![0.9, 0.1])
                    .with_cpt_entry("rainy, no", vec![0.6, 0.4])
                    .with_cpt_entry("dry, yes", vec![0.7, 0.3])
                    .with_cpt_entry("dry, no", vec![0.15, 0.85]),
            ),
        ];
        order
            .iter()
            .fold(BayesianConfig::new("slow week"), |config, &at| {
                config.with_node(nodes[at].0, nodes[at].1.clone())
            })
    }

    /// Same input, same output (STABILITY.md 4), to the last bit. The nodes
    /// were kept in a `HashMap`, whose order is random per map, so the factors
    /// were multiplied in a different order from one run to the next, and
    /// P(no roadwork | slow, rainy) came out as 0.7272727272727272 or
    /// 0.7272727272727273.
    /// Each network here is a new map, built in each of the six orders.
    #[test]
    fn test_the_same_network_gives_the_same_bits_every_time() {
        let orders = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        let mut evidence = HashMap::new();
        evidence.insert("sales".to_string(), "slow");
        evidence.insert("weather".to_string(), "rainy");
        let mut answers = std::collections::BTreeSet::new();
        for build in 0..200 {
            let engine = BayesianEngine::new(slow_week(orders[build % 6])).unwrap();
            let roadwork = engine.query_with_evidence("roadwork", &evidence).unwrap();
            let all = engine.query_all().unwrap().to_json().unwrap();
            let bits: Vec<u64> = roadwork.probabilities.iter().map(|p| p.to_bits()).collect();
            answers.insert((bits, all));
        }
        assert_eq!(answers.len(), 1, "{answers:#?}");
        // 3/11 from the joint distribution, to within one unit in the last place
        let (bits, _) = answers.into_iter().next().unwrap();
        let yes = f64::from_bits(bits[0]);
        assert!(
            (yes - 3.0 / 11.0).abs() <= f64::EPSILON * yes,
            "P(roadwork | slow, rainy) = {yes}"
        );
    }

    #[test]
    fn test_engine_creation() {
        let config = create_credit_risk_network();
        let engine = BayesianEngine::new(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_marginal_query() {
        let config = create_credit_risk_network();
        let engine = BayesianEngine::new(config).unwrap();

        let result = engine.query("economic_conditions").unwrap();

        assert_eq!(result.states.len(), 3);
        assert!((result.probabilities[0] - 0.3).abs() < 0.01); // good
        assert!((result.probabilities[1] - 0.5).abs() < 0.01); // neutral
        assert!((result.probabilities[2] - 0.2).abs() < 0.01); // bad

        assert_eq!(result.most_likely, "neutral");
    }

    #[test]
    fn test_evidence_query() {
        let config = create_credit_risk_network();
        let engine = BayesianEngine::new(config).unwrap();

        let mut evidence = HashMap::new();
        evidence.insert("economic_conditions".to_string(), "bad");

        let result = engine
            .query_with_evidence("company_revenue", &evidence)
            .unwrap();

        // P(revenue | economy=bad) = [0.1, 0.3, 0.6]
        assert!((result.probabilities[0] - 0.1).abs() < 0.01); // high
        assert!((result.probabilities[1] - 0.3).abs() < 0.01); // medium
        assert!((result.probabilities[2] - 0.6).abs() < 0.01); // low

        assert_eq!(result.most_likely, "low");
    }

    #[test]
    fn test_query_all() {
        let config = create_credit_risk_network();
        let engine = BayesianEngine::new(config).unwrap();

        let result = engine.query_all().unwrap();

        assert_eq!(result.queries.len(), 3);
        assert!(result.queries.contains_key("economic_conditions"));
        assert!(result.queries.contains_key("company_revenue"));
        assert!(result.queries.contains_key("default_probability"));
    }

    #[test]
    fn test_most_likely_states() {
        let config = create_credit_risk_network();
        let engine = BayesianEngine::new(config).unwrap();

        let mpe = engine.most_likely_states().unwrap();

        assert!(mpe.contains_key("economic_conditions"));
        assert!(mpe.contains_key("company_revenue"));
        assert!(mpe.contains_key("default_probability"));

        // Most likely economy is neutral (0.5)
        assert_eq!(mpe.get("economic_conditions"), Some(&"neutral".to_string()));
    }

    #[test]
    fn test_yaml_export() {
        let config = create_credit_risk_network();
        let engine = BayesianEngine::new(config).unwrap();
        let result = engine.query_all().unwrap();
        let yaml = result.to_yaml();

        assert!(yaml.contains("queries:"));
        assert!(yaml.contains("economic_conditions"));
    }

    #[test]
    fn test_json_export() {
        let config = create_credit_risk_network();
        let engine = BayesianEngine::new(config).unwrap();
        let result = engine.query_all().unwrap();
        let json = result.to_json().unwrap();

        assert!(json.contains("\"queries\""));
        assert!(json.contains("\"economic_conditions\""));
    }

    #[test]
    fn evidence_and_targets_that_are_not_in_the_network_are_refused() {
        let engine = BayesianEngine::new(slow_week([0, 1, 2])).unwrap();
        let message = |r: Result<VariableResult, String>| r.err().unwrap_or_default();
        assert_eq!(
            message(engine.query("ghost")),
            "Variable 'ghost' not found in network"
        );
        assert_eq!(
            message(engine.query_with_evidence("ghost", &HashMap::new())),
            "Variable 'ghost' not found in network"
        );
        assert_eq!(
            message(
                engine.query_with_evidence("sales", &HashMap::from([("ghost".to_string(), "x")]))
            ),
            "Evidence variable 'ghost' not found"
        );
        assert_eq!(
            message(
                engine.query_with_evidence(
                    "sales",
                    &HashMap::from([("weather".to_string(), "snowy")])
                )
            ),
            "State 'snowy' not found for variable 'weather'"
        );
        assert_eq!(
            engine
                .query_all_with_evidence(&HashMap::from([("ghost".to_string(), "x")]))
                .err()
                .unwrap_or_default(),
            "Evidence variable 'ghost' not found"
        );
    }

    #[test]
    fn the_engine_hands_back_its_configuration() {
        let engine = BayesianEngine::new(slow_week([0, 1, 2])).unwrap();
        assert_eq!(engine.config().name, "slow week");
        assert_eq!(engine.config().nodes.len(), 3);
    }

    #[test]
    fn a_yaml_that_cannot_be_written_is_a_comment() {
        let error = <yaml_serde::Error as serde::ser::Error>::custom("broken");
        assert!(yaml_or_comment(Err(error)).starts_with("# Error serializing results: "));
        assert_eq!(yaml_or_comment(Ok("a: 1\n".to_string())), "a: 1\n");
    }
}
