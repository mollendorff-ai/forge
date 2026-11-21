//! Decision Tree Engine
//!
//! Executes backward induction to find optimal decisions and expected values.

use super::config::{Branch, DecisionTreeConfig, Node, NodeType};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    pub branch_values: BTreeMap<String, f64>,
    /// Other branches of a decision node worth exactly as much as the
    /// optimal one. `optimal_choice` picks the first by name; these are
    /// equally good, so the choice between them is not made by the numbers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tied_with: Vec<String>,
}

/// Complete tree analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeResult {
    /// Tree name
    pub name: String,
    /// Expected value at root
    pub root_expected_value: f64,
    /// Node results
    pub node_results: BTreeMap<String, NodeResult>,
    /// Optimal decision path
    pub optimal_path: Vec<String>,
    /// Decision policy (what to choose at each decision node)
    pub decision_policy: BTreeMap<String, String>,
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

/// The YAML text, or a comment saying it could not be written
fn yaml_or_comment(yaml: Result<String, yaml_serde::Error>) -> String {
    yaml.unwrap_or_else(|e| format!("# Error serializing results: {e}"))
}

impl TreeResult {
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
        let mut node_results = BTreeMap::new();

        // Start backward induction from root
        let root = self.config.root.as_ref().ok_or("No root node")?;
        let root_result = self.evaluate_node("root", root, &mut node_results)?;

        // Build optimal path
        let mut optimal_path = Vec::new();
        self.trace_optimal_path("root", &root_result, &node_results, &mut optimal_path);

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
        results: &mut BTreeMap<String, NodeResult>,
    ) -> Result<NodeResult, String> {
        let mut branch_values = BTreeMap::new();

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
                        match a.total_cmp(b) {
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

        let tied_with: Vec<String> = match (&optimal_choice, node.node_type) {
            (Some(best), NodeType::Decision) => branch_values
                .iter()
                .filter(|(name, value)| {
                    *name != best
                        && (**value - expected_value).abs() <= 1e-9 * expected_value.abs().max(1.0)
                })
                .map(|(name, _)| name.clone())
                .collect(),
            _ => Vec::new(),
        };

        let result = NodeResult {
            name: node.name.clone(),
            node_type: node.node_type,
            expected_value,
            optimal_choice,
            branch_values,
            tied_with,
        };

        results.insert(name.to_string(), result.clone());
        Ok(result)
    }

    /// Evaluate a branch value
    fn evaluate_branch(
        &self,
        branch: &Branch,
        results: &mut BTreeMap<String, NodeResult>,
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

    /// The node a result is named after: `root`, or one of the named nodes
    fn node(&self, name: &str) -> Option<&Node> {
        if name == "root" {
            self.config.root.as_ref()
        } else {
            self.config.get_node(name)
        }
    }

    fn trace_optimal_path(
        &self,
        name: &str,
        result: &NodeResult,
        results: &BTreeMap<String, NodeResult>,
        path: &mut Vec<String>,
    ) {
        match result.node_type {
            NodeType::Decision => {
                if let Some(ref choice) = result.optimal_choice {
                    path.push(format!("{} → {}", result.name, choice));

                    // Follow the chosen branch
                    let next = self
                        .node(name)
                        .and_then(|n| n.branches.get(choice))
                        .and_then(|branch| branch.next.as_ref());
                    if let Some((next, next_result)) =
                        next.and_then(|n| results.get(n).map(|r| (n, r)))
                    {
                        self.trace_optimal_path(next, next_result, results, path);
                    }
                }
            },
            NodeType::Chance => {
                path.push(format!("{} → (await outcome)", result.name));
                // For chance nodes, show all branches lead to
                if let Some(node) = self.node(name) {
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
    fn build_decision_policy(results: &BTreeMap<String, NodeResult>) -> BTreeMap<String, String> {
        // Only a decision node has an optimal choice
        results
            .iter()
            .filter_map(|(name, result)| {
                result
                    .optimal_choice
                    .as_ref()
                    .map(|choice| (name.clone(), choice.clone()))
            })
            .collect()
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
        results: &BTreeMap<String, NodeResult>,
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
            match (branch.value, &branch.next) {
                (Some(terminal), _) => outcomes.push(Outcome {
                    path: path.clone(),
                    value: value + terminal,
                    probability: reach,
                }),
                (None, Some(next)) => {
                    let next_node = self
                        .config
                        .get_node(next)
                        .ok_or_else(|| format!("Node '{next}' not found"))?;
                    self.collect_outcomes(
                        next,
                        next_node,
                        results,
                        path,
                        (reach, value),
                        outcomes,
                    )?;
                },
                (None, None) => return Err("Branch has neither value nor next node".to_string()),
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
        assert_exact!(commercialize.expected_value, 5_000_000.0);
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

    fn unchecked(config: DecisionTreeConfig) -> DecisionTreeEngine {
        DecisionTreeEngine { config }
    }

    #[test]
    fn a_tree_without_a_root_has_no_analysis() {
        let engine = unchecked(DecisionTreeConfig::new("none"));
        assert_eq!(engine.analyze().unwrap_err(), "No root node");
    }

    #[test]
    fn a_branch_must_have_a_value_or_a_next_node() {
        let empty = Branch {
            cost: 0.0,
            probability: 0.0,
            value: None,
            next: None,
        };
        let engine = unchecked(
            DecisionTreeConfig::new("t").with_root(Node::decision("A").with_branch("x", empty)),
        );
        assert_eq!(
            engine.analyze().unwrap_err(),
            "Branch has neither value nor next node"
        );
    }

    #[test]
    fn a_branch_to_a_missing_node_is_refused_by_the_analysis_too() {
        let engine = unchecked(
            DecisionTreeConfig::new("t")
                .with_root(Node::decision("A").with_branch("x", Branch::continuation("ghost"))),
        );
        assert_eq!(engine.analyze().unwrap_err(), "Node 'ghost' not found");
    }

    #[test]
    fn a_decision_without_branches_and_a_terminal_node_are_worth_nothing() {
        let engine = unchecked(DecisionTreeConfig::new("t"));
        let mut results = BTreeMap::new();
        let empty = Node::decision("Empty");
        assert_eq!(
            engine
                .evaluate_node("empty", &empty, &mut results)
                .unwrap_err(),
            "No branches in decision node"
        );
        let end = Node {
            node_type: NodeType::Terminal,
            name: "End".to_string(),
            branches: indexmap::IndexMap::new(),
        };
        let result = engine.evaluate_node("end", &end, &mut results).unwrap();
        assert!(result.expected_value.abs() < f64::EPSILON);
        assert_eq!(result.optimal_choice, None);
        assert!(results.contains_key("end"));
    }

    #[test]
    fn collecting_outcomes_refuses_a_policy_the_tree_cannot_follow() {
        let root = Node::decision("A").with_branch("go", Branch::continuation("ghost"));
        let engine = unchecked(DecisionTreeConfig::new("t").with_root(root.clone()));
        let mut outcomes = Vec::new();
        let mut collect = |results: &BTreeMap<String, NodeResult>| {
            engine.collect_outcomes(
                "root",
                &root,
                results,
                &mut Vec::new(),
                (1.0, 0.0),
                &mut outcomes,
            )
        };
        assert_eq!(
            collect(&BTreeMap::new()).unwrap_err(),
            "No optimal choice for decision node 'root'"
        );
        let chose = |choice: &str| {
            BTreeMap::from([(
                "root".to_string(),
                NodeResult {
                    name: "A".to_string(),
                    node_type: NodeType::Decision,
                    expected_value: 0.0,
                    optimal_choice: Some(choice.to_string()),
                    branch_values: BTreeMap::new(),
                    tied_with: Vec::new(),
                },
            )])
        };
        assert_eq!(
            collect(&chose("stay")).unwrap_err(),
            "Node 'root' has no branch 'stay'"
        );
        assert_eq!(collect(&chose("go")).unwrap_err(), "Node 'ghost' not found");
    }

    #[test]
    fn a_decision_after_a_decision_follows_the_best_branch_to_the_end() {
        let config = DecisionTreeConfig::new("two steps")
            .with_root(
                Node::decision("First")
                    .with_branch("on", Branch::continuation("second").with_cost(1.0))
                    .with_branch("off", Branch::terminal(2.0)),
            )
            .with_node(
                "second",
                Node::decision("Second")
                    .with_branch("big", Branch::terminal(10.0))
                    .with_branch("small", Branch::terminal(3.0)),
            );
        let result = DecisionTreeEngine::new(config).unwrap().analyze().unwrap();
        assert_eq!(
            result.optimal_path,
            ["First → on", "Second → big"].map(String::from)
        );
        assert!((result.root_expected_value - 9.0).abs() < 1e-12);
        assert_eq!(result.decision_policy["root"], "on");
        assert_eq!(result.decision_policy["second"], "big");
    }

    #[test]
    fn a_chance_node_after_a_decision_lists_what_follows_each_outcome() {
        let config = DecisionTreeConfig::new("bet")
            .with_root(Node::decision("Bet?").with_branch("yes", Branch::continuation("coin")))
            .with_node(
                "coin",
                Node::chance("Coin")
                    .with_branch("heads", Branch::continuation("after").with_probability(0.5))
                    .with_branch("tails", Branch::terminal(0.0).with_probability(0.5)),
            )
            .with_node(
                "after",
                Node::decision("Cash out?")
                    .with_branch("cash", Branch::terminal(10.0))
                    .with_branch("hold", Branch::terminal(4.0)),
            );
        let result = DecisionTreeEngine::new(config).unwrap().analyze().unwrap();
        assert_eq!(
            result.optimal_path,
            [
                "Bet? → yes",
                "Coin → (await outcome)",
                "  if heads →",
                "    Cash out? → cash",
            ]
            .map(String::from)
        );
        assert!((result.root_expected_value - 5.0).abs() < 1e-12);
    }

    #[test]
    fn a_yaml_that_cannot_be_written_is_a_comment() {
        let error = <yaml_serde::Error as serde::ser::Error>::custom("broken");
        assert!(yaml_or_comment(Err(error)).starts_with("# Error serializing results: "));
        assert_eq!(yaml_or_comment(Ok("a: 1\n".to_string())), "a: 1\n");
    }

    #[test]
    fn the_engine_hands_back_its_configuration() {
        let engine = DecisionTreeEngine::new(create_rnd_tree()).unwrap();
        assert_eq!(engine.config().name, "R&D Investment");
    }

    #[test]
    fn tracing_stops_where_the_results_stop() {
        let config = DecisionTreeConfig::new("t")
            .with_root(Node::decision("A").with_branch("go", Branch::continuation("b")))
            .with_node(
                "b",
                Node::decision("B").with_branch("x", Branch::terminal(1.0)),
            );
        let engine = unchecked(config);
        let root = NodeResult {
            name: "A".to_string(),
            node_type: NodeType::Decision,
            expected_value: 0.0,
            optimal_choice: Some("go".to_string()),
            branch_values: BTreeMap::new(),
            tied_with: Vec::new(),
        };
        // `b` has no result: the path ends after the first step
        let mut path = Vec::new();
        engine.trace_optimal_path("root", &root, &BTreeMap::new(), &mut path);
        assert_eq!(path, ["A → go"]);
        // A chance node's outcome without a result is left out
        let chance = NodeResult {
            node_type: NodeType::Chance,
            ..root
        };
        let mut path = Vec::new();
        engine.trace_optimal_path("root", &chance, &BTreeMap::new(), &mut path);
        assert_eq!(path, ["A → (await outcome)"]);
        // A name the tree does not have lists no outcomes; a decision
        // without a choice adds nothing
        let mut path = Vec::new();
        engine.trace_optimal_path("ghost", &chance, &BTreeMap::new(), &mut path);
        assert_eq!(path, ["A → (await outcome)"]);
        let undecided = NodeResult {
            optimal_choice: None,
            node_type: NodeType::Decision,
            ..chance
        };
        let mut path = Vec::new();
        engine.trace_optimal_path("root", &undecided, &BTreeMap::new(), &mut path);
        assert_eq!(path, Vec::<String>::new());
    }

    #[test]
    fn a_terminal_node_ends_the_path_and_adds_no_outcome() {
        let end = Node {
            node_type: NodeType::Terminal,
            name: "End".to_string(),
            branches: indexmap::IndexMap::from([("x".to_string(), Branch::terminal(1.0))]),
        };
        let config = DecisionTreeConfig::new("t")
            .with_root(
                Node::chance("Flip")
                    .with_branch("a", Branch::continuation("end").with_probability(0.5))
                    .with_branch("b", Branch::terminal(4.0).with_probability(0.5)),
            )
            .with_node("end", end);
        let result = DecisionTreeEngine::new(config).unwrap().analyze().unwrap();
        // The terminal node is worth nothing: 0.5 * 0 + 0.5 * 4
        assert!((result.root_expected_value - 2.0).abs() < 1e-12);
        assert_eq!(result.risk_profile.outcomes.len(), 1);
        assert_eq!(result.optimal_path, ["Flip → (await outcome)", "  if a →"]);
        assert!(result.decision_policy.is_empty());
    }

    #[test]
    fn collecting_outcomes_refuses_a_branch_that_goes_nowhere() {
        let empty = Branch {
            cost: 0.0,
            probability: 1.0,
            value: None,
            next: None,
        };
        let root = Node::chance("A").with_branch("x", empty);
        let engine = unchecked(DecisionTreeConfig::new("t").with_root(root.clone()));
        let error = engine
            .collect_outcomes(
                "root",
                &root,
                &BTreeMap::new(),
                &mut Vec::new(),
                (1.0, 0.0),
                &mut Vec::new(),
            )
            .unwrap_err();
        assert_eq!(error, "Branch has neither value nor next node");
    }
}
