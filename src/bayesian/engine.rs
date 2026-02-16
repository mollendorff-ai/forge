//! Bayesian Network Engine
//!
//! High-level interface for Bayesian network inference.

use super::config::BayesianConfig;
use super::inference::BeliefPropagation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub queries: HashMap<String, VariableResult>,
    /// Evidence used
    pub evidence: HashMap<String, String>,
}

impl BayesianResult {
    /// Export results to YAML format
    #[must_use]
    pub fn to_yaml(&self) -> String {
        serde_yaml_ng::to_string(self).unwrap_or_else(|_| "# Error serializing results".to_string())
    }

    /// Export results to JSON format
    ///
    /// # Errors
    ///
    /// Returns an error if JSON serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
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
        let probs = self.bp.query(target)?;

        let node = self
            .config
            .nodes
            .get(target)
            .ok_or_else(|| format!("Variable '{target}' not found"))?;

        let (max_idx, max_prob) = probs
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map_or((0, 0.0), |(i, p)| (i, *p));

        Ok(VariableResult {
            name: target.to_string(),
            states: node.states.clone(),
            probabilities: probs,
            most_likely: node.states.get(max_idx).cloned().unwrap_or_default(),
            max_probability: max_prob,
        })
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

        let probs = self.bp.query_with_evidence(target, &evidence_indices)?;

        let node = self
            .config
            .nodes
            .get(target)
            .ok_or_else(|| format!("Variable '{target}' not found"))?;

        let (max_idx, max_prob) = probs
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
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
        let mut queries = HashMap::new();

        for name in self.config.nodes.keys() {
            let result = self.query(name)?;
            queries.insert(name.clone(), result);
        }

        Ok(BayesianResult {
            name: self.config.name.clone(),
            queries,
            evidence: HashMap::new(),
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
        let mut queries = HashMap::new();

        for name in self.config.nodes.keys() {
            // Skip evidence variables (their probability is deterministic)
            if evidence.contains_key(name) {
                continue;
            }

            let result = self.query_with_evidence(name, evidence)?;
            queries.insert(name.clone(), result);
        }

        // Convert evidence to string map
        let evidence_str: HashMap<String, String> = evidence
            .iter()
            .map(|(k, v)| (k.clone(), (*v).to_string()))
            .collect();

        Ok(BayesianResult {
            name: self.config.name.clone(),
            queries,
            evidence: evidence_str,
        })
    }

    /// Get the most likely explanation (MPE) for all variables
    ///
    /// # Errors
    ///
    /// Returns an error if querying any variable fails.
    pub fn most_likely_explanation(&self) -> Result<HashMap<String, String>, String> {
        let mut explanation = HashMap::new();

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
    fn test_most_likely_explanation() {
        let config = create_credit_risk_network();
        let engine = BayesianEngine::new(config).unwrap();

        let mpe = engine.most_likely_explanation().unwrap();

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
}
