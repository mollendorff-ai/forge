//! Decision Tree Configuration
//!
//! Handles parsing and validation of decision tree structures from YAML.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of node in the decision tree
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    /// Choice point (we control)
    Decision,
    /// Uncertainty (we don't control)
    Chance,
    /// End state with value
    Terminal,
}

/// A branch from a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    /// Cost incurred when taking this branch (for decision nodes)
    #[serde(default)]
    pub cost: f64,
    /// Probability of this outcome (for chance nodes, must sum to 1.0)
    #[serde(default)]
    pub probability: f64,
    /// Terminal value if this is an endpoint
    pub value: Option<f64>,
    /// Next node reference if not terminal
    pub next: Option<String>,
}

impl Branch {
    /// Create a terminal branch with a value
    #[must_use]
    pub const fn terminal(value: f64) -> Self {
        Self {
            cost: 0.0,
            probability: 0.0,
            value: Some(value),
            next: None,
        }
    }

    /// Create a continuation branch
    #[must_use]
    pub fn continuation(next: &str) -> Self {
        Self {
            cost: 0.0,
            probability: 0.0,
            value: None,
            next: Some(next.to_string()),
        }
    }

    /// Add a cost to this branch
    #[must_use]
    pub const fn with_cost(mut self, cost: f64) -> Self {
        self.cost = cost;
        self
    }

    /// Add a probability to this branch
    #[must_use]
    pub const fn with_probability(mut self, probability: f64) -> Self {
        self.probability = probability;
        self
    }

    /// Check if this branch is terminal
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.value.is_some() && self.next.is_none()
    }
}

/// A node in the decision tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Type of node
    #[serde(rename = "type")]
    pub node_type: NodeType,
    /// Human-readable name
    #[serde(default)]
    pub name: String,
    /// Branches from this node
    pub branches: HashMap<String, Branch>,
}

impl Node {
    /// Create a new decision node
    #[must_use]
    pub fn decision(name: &str) -> Self {
        Self {
            node_type: NodeType::Decision,
            name: name.to_string(),
            branches: HashMap::new(),
        }
    }

    /// Create a new chance node
    #[must_use]
    pub fn chance(name: &str) -> Self {
        Self {
            node_type: NodeType::Chance,
            name: name.to_string(),
            branches: HashMap::new(),
        }
    }

    /// Add a branch to this node
    #[must_use]
    pub fn with_branch(mut self, name: &str, branch: Branch) -> Self {
        self.branches.insert(name.to_string(), branch);
        self
    }

    /// Validate node structure
    ///
    /// # Errors
    ///
    /// Returns an error if the node has no branches or chance node probabilities
    /// do not sum to 1.0.
    pub fn validate(&self) -> Result<(), String> {
        const TOLERANCE: f64 = 0.001;

        if self.branches.is_empty() {
            return Err(format!("Node '{}' has no branches", self.name));
        }

        if self.node_type == NodeType::Chance {
            let total_prob: f64 = self.branches.values().map(|b| b.probability).sum();
            if (total_prob - 1.0).abs() > TOLERANCE {
                return Err(format!(
                    "Chance node '{}' probabilities must sum to 1.0, got {:.4}",
                    self.name, total_prob
                ));
            }
        }

        Ok(())
    }
}

/// Configuration for a decision tree
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecisionTreeConfig {
    /// Name of the decision tree
    #[serde(default)]
    pub name: String,
    /// Root node definition
    pub root: Option<Node>,
    /// Additional nodes by name
    #[serde(default)]
    pub nodes: HashMap<String, Node>,
}

impl DecisionTreeConfig {
    /// Create a new empty configuration
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            root: None,
            nodes: HashMap::new(),
        }
    }

    /// Set the root node
    #[must_use]
    pub fn with_root(mut self, root: Node) -> Self {
        self.root = Some(root);
        self
    }

    /// Add a named node
    #[must_use]
    pub fn with_node(mut self, name: &str, node: Node) -> Self {
        self.nodes.insert(name.to_string(), node);
        self
    }

    /// Validate the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the tree has no root, any node is invalid,
    /// references are broken, or the graph contains cycles.
    pub fn validate(&self) -> Result<(), String> {
        let root = self.root.as_ref().ok_or("No root node defined")?;
        root.validate()?;

        // Validate all referenced nodes exist
        self.validate_references(root)?;

        // Validate all nodes
        for (name, node) in &self.nodes {
            node.validate().map_err(|e| format!("Node '{name}': {e}"))?;
            self.validate_references(node)?;
        }

        // Check for cycles
        self.check_cycles()?;

        Ok(())
    }

    /// Validate that all referenced nodes exist
    fn validate_references(&self, node: &Node) -> Result<(), String> {
        for (branch_name, branch) in &node.branches {
            if let Some(ref next) = branch.next {
                if !self.nodes.contains_key(next) {
                    return Err(format!(
                        "Branch '{branch_name}' references non-existent node '{next}'"
                    ));
                }
            }
        }
        Ok(())
    }

    /// Check for cycles in the tree (must be a DAG)
    fn check_cycles(&self) -> Result<(), String> {
        let mut visited = std::collections::HashSet::new();
        let mut stack = std::collections::HashSet::new();

        if let Some(ref root) = self.root {
            self.dfs_cycle_check("root", root, &mut visited, &mut stack)?;
        }

        Ok(())
    }

    fn dfs_cycle_check(
        &self,
        name: &str,
        node: &Node,
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

        for branch in node.branches.values() {
            if let Some(ref next) = branch.next {
                if let Some(next_node) = self.nodes.get(next) {
                    self.dfs_cycle_check(next, next_node, visited, stack)?;
                }
            }
        }

        stack.remove(name);
        Ok(())
    }

    /// Get a node by name
    #[must_use]
    pub fn get_node(&self, name: &str) -> Option<&Node> {
        self.nodes.get(name)
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    fn create_rnd_tree() -> DecisionTreeConfig {
        DecisionTreeConfig::new("R&D Investment")
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
            )
    }

    #[test]
    fn test_tree_config_validation() {
        let tree = create_rnd_tree();
        assert!(tree.validate().is_ok());
    }

    #[test]
    fn test_missing_root_rejected() {
        let tree = DecisionTreeConfig::new("Empty");
        let result = tree.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No root node"));
    }

    #[test]
    fn test_invalid_reference_rejected() {
        let tree = DecisionTreeConfig::new("Bad Ref").with_root(
            Node::decision("Start").with_branch("go", Branch::continuation("nonexistent")),
        );

        let result = tree.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("non-existent node"));
    }

    #[test]
    fn test_chance_probabilities_must_sum_to_one() {
        let tree = DecisionTreeConfig::new("Bad Probs").with_root(
            Node::chance("Coin flip")
                .with_branch("heads", Branch::terminal(100.0).with_probability(0.5))
                .with_branch("tails", Branch::terminal(0.0).with_probability(0.3)),
        );

        let result = tree.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("sum to 1.0"));
    }

    #[test]
    fn test_cycle_detection() {
        let tree = DecisionTreeConfig::new("Cycle")
            .with_root(Node::decision("A").with_branch("go", Branch::continuation("b")))
            .with_node(
                "b",
                Node::decision("B").with_branch("back", Branch::continuation("b")),
            );

        let result = tree.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cycle"));
    }
}
