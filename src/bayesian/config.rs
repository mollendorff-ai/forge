//! Bayesian Network Configuration
//!
//! Handles parsing and validation of Bayesian network definitions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of node in the Bayesian network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    /// Discrete node with finite states
    #[default]
    Discrete,
    /// Continuous node (Gaussian)
    Continuous,
}

/// A node in the Bayesian network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BayesianNode {
    /// Node type
    #[serde(default, rename = "type")]
    pub node_type: NodeType,
    /// Possible states (for discrete nodes)
    #[serde(default)]
    pub states: Vec<String>,
    /// Prior probabilities (for root nodes)
    #[serde(default)]
    pub prior: Vec<f64>,
    /// Parent node names
    #[serde(default)]
    pub parents: Vec<String>,
    /// Conditional probability table (CPT)
    /// Keys are parent state combinations, values are probabilities for this node's states
    #[serde(default)]
    pub cpt: HashMap<String, Vec<f64>>,
    /// For continuous nodes: mean
    #[serde(default)]
    pub mean: f64,
    /// For continuous nodes: standard deviation
    #[serde(default)]
    pub std: f64,
}

impl BayesianNode {
    /// Create a new discrete node with states
    pub fn discrete(states: Vec<&str>) -> Self {
        Self {
            node_type: NodeType::Discrete,
            states: states
                .into_iter()
                .map(std::string::ToString::to_string)
                .collect(),
            prior: Vec::new(),
            parents: Vec::new(),
            cpt: HashMap::new(),
            mean: 0.0,
            std: 1.0,
        }
    }

    /// Create a new continuous node
    pub fn continuous(mean: f64, std: f64) -> Self {
        Self {
            node_type: NodeType::Continuous,
            states: Vec::new(),
            prior: Vec::new(),
            parents: Vec::new(),
            cpt: HashMap::new(),
            mean,
            std,
        }
    }

    /// Set prior probabilities (for root nodes)
    pub fn with_prior(mut self, prior: Vec<f64>) -> Self {
        self.prior = prior;
        self
    }

    /// Set parent nodes
    pub fn with_parents(mut self, parents: Vec<&str>) -> Self {
        self.parents = parents
            .into_iter()
            .map(std::string::ToString::to_string)
            .collect();
        self
    }

    /// Add CPT entry
    pub fn with_cpt_entry(mut self, parent_state: &str, probs: Vec<f64>) -> Self {
        self.cpt.insert(parent_state.to_string(), probs);
        self
    }

    /// Validate the node
    pub fn validate(&self, name: &str) -> Result<(), String> {
        match self.node_type {
            NodeType::Discrete => self.validate_discrete(name),
            NodeType::Continuous => self.validate_continuous(name),
        }
    }

    fn validate_discrete(&self, name: &str) -> Result<(), String> {
        if self.states.is_empty() {
            return Err(format!("Node '{name}': discrete node must have states"));
        }

        // If root node, check prior
        if self.parents.is_empty() {
            if self.prior.is_empty() {
                return Err(format!(
                    "Node '{name}': root node must have prior probabilities"
                ));
            }
            if self.prior.len() != self.states.len() {
                return Err(format!(
                    "Node '{}': prior length ({}) must match states ({})",
                    name,
                    self.prior.len(),
                    self.states.len()
                ));
            }
            let sum: f64 = self.prior.iter().sum();
            if (sum - 1.0).abs() > 0.001 {
                return Err(format!(
                    "Node '{name}': prior probabilities must sum to 1.0, got {sum}"
                ));
            }
        } else {
            // Child node, check CPT
            if self.cpt.is_empty() {
                return Err(format!("Node '{name}': child node must have CPT"));
            }
            for (key, probs) in &self.cpt {
                if probs.len() != self.states.len() {
                    return Err(format!(
                        "Node '{}': CPT entry '{}' length ({}) must match states ({})",
                        name,
                        key,
                        probs.len(),
                        self.states.len()
                    ));
                }
                let sum: f64 = probs.iter().sum();
                if (sum - 1.0).abs() > 0.001 {
                    return Err(format!(
                        "Node '{name}': CPT entry '{key}' must sum to 1.0, got {sum}"
                    ));
                }
            }
        }

        Ok(())
    }

    fn validate_continuous(&self, name: &str) -> Result<(), String> {
        if self.std <= 0.0 {
            return Err(format!(
                "Node '{name}': standard deviation must be positive"
            ));
        }
        Ok(())
    }

    /// Check if this is a root node
    pub fn is_root(&self) -> bool {
        self.parents.is_empty()
    }

    /// Get probability for a state given parent state
    pub fn get_probability(&self, state_idx: usize, parent_state: Option<&str>) -> f64 {
        if self.is_root() {
            self.prior.get(state_idx).copied().unwrap_or(0.0)
        } else if let Some(ps) = parent_state {
            self.cpt
                .get(ps)
                .and_then(|probs| probs.get(state_idx))
                .copied()
                .unwrap_or(0.0)
        } else {
            0.0
        }
    }
}

/// Configuration for Bayesian network
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BayesianConfig {
    /// Network name
    #[serde(default)]
    pub name: String,
    /// Nodes by name
    #[serde(default)]
    pub nodes: HashMap<String, BayesianNode>,
}

impl BayesianConfig {
    /// Create a new configuration
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            nodes: HashMap::new(),
        }
    }

    /// Add a node
    pub fn with_node(mut self, name: &str, node: BayesianNode) -> Self {
        self.nodes.insert(name.to_string(), node);
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.nodes.is_empty() {
            return Err("Network must have at least one node".to_string());
        }

        // Validate each node
        for (name, node) in &self.nodes {
            node.validate(name)?;

            // Check parent references
            for parent in &node.parents {
                if !self.nodes.contains_key(parent) {
                    return Err(format!(
                        "Node '{name}' references non-existent parent '{parent}'"
                    ));
                }
            }
        }

        // Check for cycles
        self.check_cycles()?;

        Ok(())
    }

    /// Check for cycles (must be a DAG)
    fn check_cycles(&self) -> Result<(), String> {
        let mut visited = std::collections::HashSet::new();
        let mut stack = std::collections::HashSet::new();

        for name in self.nodes.keys() {
            self.dfs_cycle_check(name, &mut visited, &mut stack)?;
        }

        Ok(())
    }

    fn dfs_cycle_check(
        &self,
        name: &str,
        visited: &mut std::collections::HashSet<String>,
        stack: &mut std::collections::HashSet<String>,
    ) -> Result<(), String> {
        if stack.contains(name) {
            return Err(format!("Cycle detected involving node '{name}'"));
        }
        if visited.contains(name) {
            return Ok(());
        }

        visited.insert(name.to_string());
        stack.insert(name.to_string());

        if let Some(node) = self.nodes.get(name) {
            for parent in &node.parents {
                self.dfs_cycle_check(parent, visited, stack)?;
            }
        }

        stack.remove(name);
        Ok(())
    }

    /// Get topological order of nodes
    pub fn topological_order(&self) -> Vec<String> {
        let mut order = Vec::new();
        let mut visited = std::collections::HashSet::new();

        fn visit(
            name: &str,
            config: &BayesianConfig,
            visited: &mut std::collections::HashSet<String>,
            order: &mut Vec<String>,
        ) {
            if visited.contains(name) {
                return;
            }
            visited.insert(name.to_string());

            if let Some(node) = config.nodes.get(name) {
                for parent in &node.parents {
                    visit(parent, config, visited, order);
                }
            }

            order.push(name.to_string());
        }

        for name in self.nodes.keys() {
            visit(name, self, &mut visited, &mut order);
        }

        order
    }

    /// Get root nodes
    pub fn root_nodes(&self) -> Vec<&str> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.is_root())
            .map(|(name, _)| name.as_str())
            .collect()
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

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
    fn test_config_validation() {
        let config = create_credit_risk_network();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_empty_network_rejected() {
        let config = BayesianConfig::new("Empty");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_missing_parent_rejected() {
        let config = BayesianConfig::new("Bad Ref").with_node(
            "child",
            BayesianNode::discrete(vec!["a", "b"])
                .with_parents(vec!["nonexistent"])
                .with_cpt_entry("x", vec![0.5, 0.5]),
        );

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_prior_sum_rejected() {
        let config = BayesianConfig::new("Bad Prior").with_node(
            "node",
            BayesianNode::discrete(vec!["a", "b"]).with_prior(vec![0.3, 0.3]),
        );

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_topological_order() {
        let config = create_credit_risk_network();
        let order = config.topological_order();

        // economic_conditions should come before company_revenue
        let ec_idx = order
            .iter()
            .position(|n| n == "economic_conditions")
            .unwrap();
        let cr_idx = order.iter().position(|n| n == "company_revenue").unwrap();
        let dp_idx = order
            .iter()
            .position(|n| n == "default_probability")
            .unwrap();

        assert!(ec_idx < cr_idx);
        assert!(cr_idx < dp_idx);
    }

    #[test]
    fn test_root_nodes() {
        let config = create_credit_risk_network();
        let roots = config.root_nodes();

        assert_eq!(roots.len(), 1);
        assert!(roots.contains(&"economic_conditions"));
    }
}
