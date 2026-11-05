//! Decision Tree Engine
//!
//! Executes backward induction to find optimal decisions and expected values.

use super::config::{Branch, DecisionTreeConfig, Node, NodeType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result for a single node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResult {
    /// Node name
    pub name: String,
    /// Node type
    pub node_type: NodeType,
    /// Expected value at this node
    pub expected_value: f64,
    /// Optimal choice (for decision nodes)
    pub optimal_choice: Option<String>,
    /// Branch values
    pub branch_values: HashMap<String, f64>,
}

/// Complete tree analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeResult {
    /// Tree name
    pub name: String,
    /// Expected value at root
    pub root_expected_value: f64,
    /// Node results
    pub node_results: HashMap<String, NodeResult>,
    /// Optimal decision path
    pub optimal_path: Vec<String>,
    /// Decision policy (what to choose at each decision node)
    pub decision_policy: HashMap<String, String>,
    /// Risk profile
    pub risk_profile: RiskProfile,
}

/// Outcomes of the tree when every decision follows the optimal policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskProfile {
    /// Best outcome under the optimal policy
    pub best_case: f64,
    /// Worst outcome under the optimal policy
    pub worst_case: f64,
    /// Probability that the outcome is above zero
    pub probability_positive: f64,
    /// Every end point reachable under the optimal policy, best first
    pub outcomes: Vec<Outcome>,
}

/// One end point of the tree under the optimal policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    /// Branches taken from the root, as `node.branch`
    pub path: Vec<String>,
    /// Terminal value minus every cost paid along the path
    pub value: f64,
    /// Product of the chance probabilities along the path
    pub probability: f64,
}

impl TreeResult {
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

/// Decision Tree Engine
pub struct DecisionTreeEngine {
    config: DecisionTreeConfig,
}

impl DecisionTreeEngine {
    /// Create a new decision tree engine
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn new(config: DecisionTreeConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Analyze the decision tree using backward induction
    ///
    /// # Errors
    ///
    /// Returns an error if the tree has no root or a branch references a missing node.
    pub fn analyze(&self) -> Result<TreeResult, String> {
        let mut node_results = HashMap::new();

        // Start backward induction from root
        let root = self.config.root.as_ref().ok_or("No root node")?;
        let root_result = self.evaluate_node("root", root, &mut node_results)?;

        // Build optimal path
        let optimal_path = self.build_optimal_path(&node_results);

        // Build decision policy
        let decision_policy = Self::build_decision_policy(&node_results);

        // Follow the optimal policy to every end point
        let mut outcomes = Vec::new();
        self.collect_outcomes(
            "root",
            root,
            &node_results,
            &mut Vec::new(),
            (1.0, 0.0),
            &mut outcomes,
        )?;
        let risk_profile = Self::calculate_risk_profile(outcomes);

        Ok(TreeResult {
            name: self.config.name.clone(),
            root_expected_value: root_result.expected_value,
            node_results,
            optimal_path,
            decision_policy,
            risk_profile,
        })
    }

    /// Evaluate a node recursively using backward induction
    fn evaluate_node(
        &self,
        name: &str,
        node: &Node,
        results: &mut HashMap<String, NodeResult>,
    ) -> Result<NodeResult, String> {
        let mut branch_values = HashMap::new();

        // Evaluate each branch
        for (branch_name, branch) in &node.branches {
            let branch_value = self.evaluate_branch(branch, results)?;
            branch_values.insert(branch_name.clone(), branch_value);
        }

        // Calculate expected value based on node type
        let (expected_value, optimal_choice) = match node.node_type {
            NodeType::Decision => {
                // Decision node: choose maximum value branch
                // Use alphabetical ordering as tie-breaker for deterministic results
                let (best_name, best_value) = branch_values
                    .iter()
                    .max_by(|(name_a, a), (name_b, b)| {
                        match a.partial_cmp(b).unwrap() {
                            // When values are equal, prefer earlier alphabetically
                            std::cmp::Ordering::Equal => name_b.cmp(name_a),
                            other => other,
                        }
                    })
                    .map(|(n, v)| (n.clone(), *v))
                    .ok_or("No branches in decision node")?;
                (best_value, Some(best_name))
            },
            NodeType::Chance => {
                // Chance node: probability-weighted expected value
                let ev: f64 = node
                    .branches
                    .iter()
                    .map(|(branch_name, branch)| {
                        branch.probability * branch_values.get(branch_name).unwrap_or(&0.0)
                    })
                    .sum();
                (ev, None)
            },
            NodeType::Terminal => {
                // Terminal nodes shouldn't have branches in typical usage
                (0.0, None)
            },
        };

        let result = NodeResult {
            name: node.name.clone(),
            node_type: node.node_type,
            expected_value,
            optimal_choice,
            branch_values,
        };

        results.insert(name.to_string(), result.clone());
        Ok(result)
    }

    /// Evaluate a branch value
    fn evaluate_branch(
        &self,
        branch: &Branch,
        results: &mut HashMap<String, NodeResult>,
    ) -> Result<f64, String> {
        let base_value = if let Some(value) = branch.value {
            value
        } else if let Some(ref next) = branch.next {
            // Continuation branch - recurse
            let next_node = self
                .config
                .get_node(next)
                .ok_or_else(|| format!("Node '{next}' not found"))?;
            let next_result = self.evaluate_node(next, next_node, results)?;
            next_result.expected_value
        } else {
            return Err("Branch has neither value nor next node".to_string());
        };

        // Subtract cost (for decision branches)
        Ok(base_value - branch.cost)
    }

    /// Build the optimal decision path
    fn build_optimal_path(&self, results: &HashMap<String, NodeResult>) -> Vec<String> {
        let mut path = Vec::new();

        if let Some(root_result) = results.get("root") {
            self.trace_optimal_path("root", root_result, results, &mut path);
        }

        path
    }

    fn trace_optimal_path(
        &self,
        name: &str,
        result: &NodeResult,
        results: &HashMap<String, NodeResult>,
        path: &mut Vec<String>,
    ) {
        match result.node_type {
            NodeType::Decision => {
                if let Some(ref choice) = result.optimal_choice {
                    path.push(format!("{} → {}", result.name, choice));

                    // Follow the chosen branch
                    if let Some(root) = &self.config.root {
                        if name == "root" {
                            if let Some(branch) = root.branches.get(choice) {
                                if let Some(ref next) = branch.next {
                                    if let Some(next_result) = results.get(next) {
                                        self.trace_optimal_path(next, next_result, results, path);
                                    }
                                }
                            }
                        }
                    }

                    if let Some(node) = self.config.get_node(name) {
                        if let Some(branch) = node.branches.get(choice) {
                            if let Some(ref next) = branch.next {
                                if let Some(next_result) = results.get(next) {
                                    self.trace_optimal_path(next, next_result, results, path);
                                }
                            }
                        }
                    }
                }
            },
            NodeType::Chance => {
                path.push(format!("{} → (await outcome)", result.name));
                // For chance nodes, show all branches lead to
                if let Some(node) = self.config.get_node(name) {
                    for (branch_name, branch) in &node.branches {
                        if let Some(ref next) = branch.next {
                            if let Some(next_result) = results.get(next) {
                                path.push(format!("  if {branch_name} →"));
                                let mut sub_path = Vec::new();
                                self.trace_optimal_path(next, next_result, results, &mut sub_path);
                                for p in sub_path {
                                    path.push(format!("    {p}"));
                                }
                            }
                        }
                    }
                }
            },
            NodeType::Terminal => {
                // End of path
            },
        }
    }

    /// Build decision policy
    fn build_decision_policy(results: &HashMap<String, NodeResult>) -> HashMap<String, String> {
        let mut policy = HashMap::new();

        for (name, result) in results {
            if result.node_type == NodeType::Decision {
                if let Some(ref choice) = result.optimal_choice {
                    policy.insert(name.clone(), choice.clone());
                }
            }
        }

        policy
    }

    /// Collect the end points reachable when every decision follows the policy.
    ///
    /// `(probability, paid)` are the probability of reaching `node` and the
    /// value accumulated on the way there (costs are negative). Decision nodes
    /// follow only their optimal branch; chance nodes follow every branch with
    /// a probability above zero, in name order.
    fn collect_outcomes(
        &self,
        name: &str,
        node: &Node,
        results: &HashMap<String, NodeResult>,
        path: &mut Vec<String>,
        (probability, paid): (f64, f64),
        outcomes: &mut Vec<Outcome>,
    ) -> Result<(), String> {
        let mut branches: Vec<(&String, &Branch, f64)> = match node.node_type {
            NodeType::Decision => {
                let choice = results
                    .get(name)
                    .and_then(|r| r.optimal_choice.as_ref())
                    .ok_or_else(|| format!("No optimal choice for decision node '{name}'"))?;
                let (branch_name, branch) = node
                    .branches
                    .get_key_value(choice)
                    .ok_or_else(|| format!("Node '{name}' has no branch '{choice}'"))?;
                vec![(branch_name, branch, 1.0)]
            },
            NodeType::Chance => node
                .branches
                .iter()
                .filter(|(_, branch)| branch.probability > 0.0)
                .map(|(branch_name, branch)| (branch_name, branch, branch.probability))
                .collect(),
            NodeType::Terminal => Vec::new(),
        };
        branches.sort_by(|a, b| a.0.cmp(b.0));

        for (branch_name, branch, branch_probability) in branches {
            let reach = probability * branch_probability;
            let value = paid - branch.cost;
            path.push(format!("{name}.{branch_name}"));
            if let Some(terminal) = branch.value {
                outcomes.push(Outcome {
                    path: path.clone(),
                    value: value + terminal,
                    probability: reach,
                });
            } else if let Some(ref next) = branch.next {
                let next_node = self
                    .config
                    .get_node(next)
                    .ok_or_else(|| format!("Node '{next}' not found"))?;
                self.collect_outcomes(next, next_node, results, path, (reach, value), outcomes)?;
            }
            path.pop();
        }
        Ok(())
    }

    /// Summarize the outcomes reachable under the optimal policy
    fn calculate_risk_profile(mut outcomes: Vec<Outcome>) -> RiskProfile {
        outcomes.sort_by(|a, b| b.value.total_cmp(&a.value));
        let best_case = outcomes.first().map_or(0.0, |o| o.value);
        let worst_case = outcomes.last().map_or(0.0, |o| o.value);
        let probability_positive = outcomes
            .iter()
            .filter(|o| o.value > 0.0)
            .map(|o| o.probability)
            .sum();

        RiskProfile {
            best_case,
            worst_case,
            probability_positive,
            outcomes,
        }
    }

    /// Get the configuration
    #[must_use]
    pub const fn config(&self) -> &DecisionTreeConfig {
        &self.config
    }
}

#[cfg(test)]
// Financial math: exact float comparison validated against Excel/Gnumeric/R
#[allow(clippy::float_cmp)]
mod engine_tests {
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
                    .with_branch("failure", Branch::terminal(0.0).with_probability(0.40)),
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
    fn test_backward_induction() {
        let config = create_rnd_tree();
        let engine = DecisionTreeEngine::new(config).unwrap();
        let result = engine.analyze().unwrap();

        // commercialize_decision: max($5M, $8M-$3M) = $5M (license)
        let commercialize = result.node_results.get("commercialize").unwrap();
        assert_eq!(commercialize.expected_value, 5_000_000.0);
        assert_eq!(commercialize.optimal_choice, Some("license".to_string()));

        // tech_outcome: 0.6 × $5M + 0.4 × $0 = $3M
        let tech = result.node_results.get("tech_outcome").unwrap();
        assert!((tech.expected_value - 3_000_000.0).abs() < 0.01);

        // root: max($3M - $2M, $0) = $1M (invest)
        assert!((result.root_expected_value - 1_000_000.0).abs() < 0.01);
    }

    #[test]
    fn test_decision_policy() {
        let config = create_rnd_tree();
        let engine = DecisionTreeEngine::new(config).unwrap();
        let result = engine.analyze().unwrap();

        assert_eq!(
            result.decision_policy.get("root"),
            Some(&"invest".to_string())
        );
        assert_eq!(
            result.decision_policy.get("commercialize"),
            Some(&"license".to_string())
        );
    }

    /// Roundtrip validation - matches SciPy/NumPy backward induction
    #[test]
    fn test_scipy_numpy_equivalence() {
        // This test validates against Python's SciPy/NumPy
        // Python code:
        //   license_value = 5_000_000
        //   manufacture_value = 8_000_000 - 3_000_000  # net of cost
        //   commercialize_ev = max(license_value, manufacture_value)  # $5,000,000
        //
        //   p_success, p_failure = 0.60, 0.40
        //   failure_value = 0
        //   tech_ev = p_success * commercialize_ev + p_failure * failure_value  # $3,000,000
        //
        //   invest_cost = 2_000_000
        //   invest_ev = tech_ev - invest_cost  # $1,000,000
        //   no_invest_ev = 0
        //   root_ev = max(invest_ev, no_invest_ev)  # $1,000,000

        let config = create_rnd_tree();
        let engine = DecisionTreeEngine::new(config).unwrap();
        let result = engine.analyze().unwrap();

        // Validate against Python calculation
        assert!(
            (result.root_expected_value - 1_000_000.0).abs() < 0.01,
            "Root EV should be $1M, got {}",
            result.root_expected_value
        );
    }

    #[test]
    fn test_simple_coin_flip() {
        let config = DecisionTreeConfig::new("Coin Flip").with_root(
            Node::chance("Flip coin")
                .with_branch("heads", Branch::terminal(100.0).with_probability(0.5))
                .with_branch("tails", Branch::terminal(0.0).with_probability(0.5)),
        );

        let engine = DecisionTreeEngine::new(config).unwrap();
        let result = engine.analyze().unwrap();

        // EV = 0.5 * 100 + 0.5 * 0 = 50
        assert!(
            (result.root_expected_value - 50.0).abs() < 0.01,
            "Expected 50, got {}",
            result.root_expected_value
        );
    }

    #[test]
    fn test_yaml_export() {
        let config = create_rnd_tree();
        let engine = DecisionTreeEngine::new(config).unwrap();
        let result = engine.analyze().unwrap();
        let yaml = result.to_yaml();

        assert!(yaml.contains("root_expected_value"));
        assert!(yaml.contains("decision_policy"));
    }
}
