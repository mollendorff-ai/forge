//! A network as a model writes it (ADR-074).
//!
//! Each probability is a number or a formula over the model's scalars,
//! worked out before inference, and named queries give posteriors that are
//! scalars of the model, `bayesian_network.<query>.<state>`.

use super::config::{BayesianConfig, BayesianNode, Query};
use super::engine::BayesianEngine;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// The section a network is written in, and the start of its results' names.
pub const SECTION: &str = "bayesian_network";

/// A probability as a model writes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Probability {
    /// A number, read as it is
    Number(f64),
    /// A formula (`=1 - p`), worked out after the scalars it reads
    Formula(String),
}

/// A network as a model writes it.
pub type WrittenNetwork = BayesianConfig<Probability>;

/// A name a query, or a state of a node a query asks about, can have: a
/// letter or `_`, then letters, digits and `_`. It becomes part of a
/// scalar's name.
fn is_name(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// The nodes of a network, for a message.
fn node_list<P>(config: &BayesianConfig<P>) -> String {
    config
        .nodes
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

impl<P> BayesianConfig<P> {
    /// A query names a node, evidence of other nodes in states they have,
    /// and a node whose states can be part of a scalar's name.
    pub(super) fn check_query(&self, name: &str, query: &Query) -> Result<(), String> {
        let at = format!("{SECTION}.queries.{name}");
        if !is_name(name) {
            return Err(format!(
                "{SECTION}.queries: \"{name}\" is not a name for a query: letters, digits and _, \
                 starting with a letter"
            ));
        }
        let Some(node) = self.nodes.get(&query.node) else {
            return Err(format!(
                "{at}: '{}' is not a node of the network ({})",
                query.node,
                node_list(self)
            ));
        };
        if let Some(state) = node.states.iter().find(|s| !is_name(s)) {
            return Err(format!(
                "{at}: state '{state}' of {} cannot be part of a result's name \
                 ({SECTION}.{name}.<state>): a state a query asks about is named with letters, \
                 digits and _, starting with a letter",
                query.node
            ));
        }
        for (known, state) in &query.evidence {
            if known == &query.node {
                return Err(format!(
                    "{at}: {known} is the node asked about, so it cannot be evidence too"
                ));
            }
            let Some(evidence) = self.nodes.get(known) else {
                return Err(format!(
                    "{at}: the evidence names '{known}', which is not a node of the network ({})",
                    node_list(self)
                ));
            };
            if !evidence.states.contains(state) {
                return Err(format!(
                    "{at}: {known} has no state '{state}' ({})",
                    evidence.states.join(", ")
                ));
            }
        }
        Ok(())
    }

    /// The names of the results of the queries, in name order: each state of
    /// each query's node.
    #[must_use]
    pub fn result_names(&self) -> Vec<String> {
        self.queries
            .iter()
            .flat_map(|(name, query)| {
                self.nodes[&query.node]
                    .states
                    .iter()
                    .map(move |state| format!("{SECTION}.{name}.{state}"))
            })
            .collect()
    }

    /// Why `name`, under `bayesian_network.`, is not a result of the network.
    #[must_use]
    pub fn why_not(&self, name: &str) -> String {
        let queries: Vec<&str> = self.queries.keys().map(String::as_str).collect();
        if queries.is_empty() {
            return format!(
                "'{name}' is not a result of the network: it has no queries, and each query \
                 under {SECTION}.queries gives {SECTION}.<query>.<state>"
            );
        }
        let asked = name
            .strip_prefix(&format!("{SECTION}."))
            .and_then(|rest| rest.split_once('.'))
            .and_then(|(query, _)| self.queries.get(query).map(|q| (query, q)));
        let Some((query, q)) = asked else {
            return format!(
                "'{name}' is not a result of the network: its queries are {}",
                queries.join(", ")
            );
        };
        let states: Vec<String> = self.nodes[&q.node]
            .states
            .iter()
            .map(|s| format!("{SECTION}.{query}.{s}"))
            .collect();
        format!(
            "'{name}' is not a result of the network: query '{query}' gives {}",
            states.join(", ")
        )
    }
}

impl WrittenNetwork {
    /// Read a `bayesian_network:` section, and check its shape and queries.
    ///
    /// # Errors
    ///
    /// Returns an error when the section cannot be read as a network, its
    /// shape is wrong, a query is wrong, or a probability is text that is
    /// not a formula.
    pub fn read(section: &yaml_serde::Value) -> Result<Self, String> {
        let network: Self = yaml_serde::from_value(section.clone())
            .map_err(|e| format!("{SECTION} config error: {e}"))?;
        network.validate_shape()?;
        for (at, p) in network.entries() {
            if let Probability::Formula(text) = p {
                if !text.trim_start().starts_with('=') {
                    return Err(format!(
                        "{at} is the text \"{text}\": a probability is a number, or a formula \
                         that starts with ="
                    ));
                }
            }
        }
        Ok(network)
    }

    /// Every probability, with where it is written (`bayesian_network.nodes.
    /// partner.prior.defector`, `bayesian_network.nodes.move_1.cpt.
    /// cooperator.defect`). The shape is checked, so each row has a
    /// probability for each state.
    fn entries(&self) -> Vec<(String, &Probability)> {
        let mut entries = Vec::new();
        for (name, node) in &self.nodes {
            let at = format!("{SECTION}.nodes.{name}");
            for (state, p) in node.states.iter().zip(&node.prior) {
                entries.push((format!("{at}.prior.{state}"), p));
            }
            for (key, row) in &node.cpt {
                for (state, p) in node.states.iter().zip(row) {
                    entries.push((format!("{at}.cpt.{key}.{state}"), p));
                }
            }
        }
        entries
    }

    /// The formulas of the probabilities, which the network reads.
    #[must_use]
    pub fn formulas(&self) -> Vec<&str> {
        self.entries()
            .into_iter()
            .filter_map(|(_, p)| match p {
                Probability::Formula(f) => Some(f.as_str()),
                Probability::Number(_) => None,
            })
            .collect()
    }

    /// The network with each formula worked out by `value(formula, at)`.
    ///
    /// # Errors
    ///
    /// The first error `value` returns.
    pub fn work_out<E>(
        &self,
        mut value: impl FnMut(&str, &str) -> Result<f64, E>,
    ) -> Result<BayesianConfig, E> {
        let mut nodes = BTreeMap::new();
        for (name, node) in &self.nodes {
            let at = format!("{SECTION}.nodes.{name}");
            let mut row = |row: &[Probability], at: &str| -> Result<Vec<f64>, E> {
                node.states
                    .iter()
                    .zip(row)
                    .map(|(state, p)| match p {
                        Probability::Number(n) => Ok(*n),
                        Probability::Formula(f) => value(f, &format!("{at}.{state}")),
                    })
                    .collect()
            };
            let prior = row(&node.prior, &format!("{at}.prior"))?;
            let mut cpt = BTreeMap::new();
            for (key, probs) in &node.cpt {
                cpt.insert(key.clone(), row(probs, &format!("{at}.cpt.{key}"))?);
            }
            nodes.insert(
                name.clone(),
                BayesianNode {
                    node_type: node.node_type,
                    states: node.states.clone(),
                    prior,
                    parents: node.parents.clone(),
                    cpt,
                    mean: node.mean,
                    std: node.std,
                },
            );
        }
        Ok(BayesianConfig {
            name: self.name.clone(),
            nodes,
            queries: self.queries.clone(),
        })
    }
}

impl BayesianConfig {
    /// The posteriors of every query, each state's as a result named
    /// `bayesian_network.<query>.<state>`, queries in name order.
    ///
    /// # Errors
    ///
    /// Returns an error when the network's probabilities are not
    /// probabilities (a row that does not add up to 1, a number outside 0 to
    /// 1), or a query's evidence has probability 0.
    ///
    /// # Panics
    ///
    /// Never: the engine checks each query with the network, so inference
    /// finds its node and evidence.
    pub fn posteriors(&self) -> Result<Vec<(String, f64)>, String> {
        let engine = BayesianEngine::new(self.clone())?;
        let mut results = Vec::new();
        for (name, query) in &self.queries {
            let evidence: HashMap<String, &str> = query
                .evidence
                .iter()
                .map(|(node, state)| (node.clone(), state.as_str()))
                .collect();
            let posterior = engine
                .query_with_evidence(&query.node, &evidence)
                .expect("the query was checked with the network");
            if posterior.probabilities.iter().sum::<f64>() <= 0.0 {
                return Err(format!(
                    "{SECTION}.queries.{name}: the evidence cannot happen in this network (its \
                     probability is 0), so the query has no posterior"
                ));
            }
            for (state, p) in posterior.states.iter().zip(posterior.probabilities) {
                results.push((format!("{SECTION}.{name}.{state}"), p));
            }
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// kinship-protocol's mistake or defection, with its prior `p` and noise
    /// `e` as formulas, and queries after one and two defections.
    const MISTAKE: &str = "
name: Mistake or defection?
nodes:
  partner: {type: discrete, states: [cooperator, defector], prior: ['=1 - p', '=p']}
  move_1:
    type: discrete
    states: [cooperate, defect]
    parents: [partner]
    cpt: {cooperator: ['=1 - e', '=e'], defector: ['=e', '=1 - e']}
  move_2:
    type: discrete
    states: [cooperate, defect]
    parents: [partner]
    cpt: {cooperator: [0.95, 0.05], defector: [0.05, 0.95]}
queries:
  one: {node: partner, evidence: {move_1: defect}}
  two: {node: partner, evidence: {move_1: defect, move_2: defect}}
  none: {node: move_2}
";

    fn read(text: &str) -> Result<WrittenNetwork, String> {
        WrittenNetwork::read(&yaml_serde::from_str(text).unwrap())
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn formulas_are_worked_out_and_each_query_gives_its_posteriors() {
        let network = read(MISTAKE).unwrap();
        assert_eq!(
            network.formulas(),
            ["=1 - e", "=e", "=e", "=1 - e", "=1 - p", "=p"]
        );
        let mut asked = Vec::new();
        let worked = network
            .work_out(|formula, at| -> Result<f64, String> {
                asked.push(at.to_string());
                Ok(match formula {
                    "=p" => 0.1,
                    "=1 - p" => 0.9,
                    "=e" => 0.05,
                    _ => 0.95,
                })
            })
            .unwrap();
        assert_eq!(
            asked,
            [
                "bayesian_network.nodes.move_1.cpt.cooperator.cooperate",
                "bayesian_network.nodes.move_1.cpt.cooperator.defect",
                "bayesian_network.nodes.move_1.cpt.defector.cooperate",
                "bayesian_network.nodes.move_1.cpt.defector.defect",
                "bayesian_network.nodes.partner.prior.cooperator",
                "bayesian_network.nodes.partner.prior.defector",
            ]
        );
        let results = worked.posteriors().unwrap();
        let names: Vec<&str> = results.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, network.result_names());
        // By hand: 0.1 * 0.95 / (0.1 * 0.95 + 0.9 * 0.05) = 0.095 / 0.14, and
        // after two, 0.09025 / (0.09025 + 0.00225) = 0.09025 / 0.0925
        let defector = |name: &str| {
            results
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, p)| *p)
                .unwrap()
        };
        assert!(close(
            defector("bayesian_network.one.defector"),
            0.095 / 0.14
        ));
        assert!(close(
            defector("bayesian_network.two.defector"),
            0.09025 / 0.0925
        ));
        // No evidence: 0.9 * 0.05 + 0.1 * 0.95 = 0.14
        assert!(close(defector("bayesian_network.none.defect"), 0.14));
    }

    #[test]
    fn the_first_formula_that_fails_stops_the_work() {
        /// Every formula fails, saying where it is (one function for both
        /// networks, so one instantiation meets a prior and a table failing)
        fn fail(_: &str, at: &str) -> Result<f64, String> {
            Err(format!("no value at {at}"))
        }
        let err = read(MISTAKE).unwrap().work_out(fail).unwrap_err();
        assert_eq!(
            err,
            "no value at bayesian_network.nodes.move_1.cpt.cooperator.cooperate"
        );
        let root = read("nodes:\n  a: {states: [x, y], prior: ['=q', 0.5]}\n").unwrap();
        let err = root.work_out(fail).unwrap_err();
        assert_eq!(err, "no value at bayesian_network.nodes.a.prior.x");
    }

    #[test]
    fn worked_out_probabilities_must_be_probabilities() {
        let network = read(MISTAKE).unwrap();
        let worked = network
            .work_out(|formula, _| -> Result<f64, String> {
                Ok(if formula.contains('p') { 0.6 } else { 0.5 })
            })
            .unwrap();
        assert_eq!(
            worked.posteriors().unwrap_err(),
            "Node 'partner': prior probabilities must sum to 1.0, got 1.2"
        );
        let worked = network
            .work_out(|formula, _| -> Result<f64, String> {
                Ok(match formula {
                    "=p" => 1.5,
                    "=1 - p" => -0.5,
                    _ => 0.5,
                })
            })
            .unwrap();
        assert_eq!(
            worked.posteriors().unwrap_err(),
            "Node 'partner': in its prior probabilities, state 'cooperator' has -0.5; a \
             probability is from 0 to 1"
        );
    }

    #[test]
    fn a_query_is_checked_when_it_is_read() {
        let base = "nodes:
  partner: {states: [cooperator, defector], prior: [0.9, 0.1]}
  move: {states: [cooperate, defect], parents: [partner], cpt: {cooperator: [0.9, 0.1], defector: [0.1, 0.9]}}
  mood: {states: [very good, bad], prior: [0.5, 0.5]}
queries:
";
        for (query, want) in [
            (
                "  one: {node: partnr}",
                "bayesian_network.queries.one: 'partnr' is not a node of the network (mood, \
                 move, partner)",
            ),
            (
                "  one: {node: partner, evidence: {mov: defect}}",
                "bayesian_network.queries.one: the evidence names 'mov', which is not a node of \
                 the network (mood, move, partner)",
            ),
            (
                "  one: {node: partner, evidence: {move: defects}}",
                "bayesian_network.queries.one: move has no state 'defects' (cooperate, defect)",
            ),
            (
                "  one: {node: partner, evidence: {partner: defector}}",
                "bayesian_network.queries.one: partner is the node asked about, so it cannot be \
                 evidence too",
            ),
            (
                "  one: {node: mood}",
                "bayesian_network.queries.one: state 'very good' of mood cannot be part of a \
                 result's name (bayesian_network.one.<state>): a state a query asks about is \
                 named with letters, digits and _, starting with a letter",
            ),
            (
                "  one.two: {node: partner}",
                "bayesian_network.queries: \"one.two\" is not a name for a query: letters, \
                 digits and _, starting with a letter",
            ),
            (
                "  one: {node: partner, given: {move: defect}}",
                "bayesian_network config error: unknown field `given`, expected `node` or \
                 `evidence`",
            ),
        ] {
            let err = read(&format!("{base}{query}\n")).unwrap_err();
            assert!(err.starts_with(want), "{query}: {err}");
        }
        // Evidence on a node whose states are not names is fine, and so is
        // a Unicode name
        let fine = read(&format!(
            "{base}  état: {{node: partner, evidence: {{mood: very good}}}}\n  _q: {{node: move}}\n"
        ))
        .unwrap();
        assert_eq!(
            fine.result_names(),
            [
                "bayesian_network._q.cooperate",
                "bayesian_network._q.defect",
                "bayesian_network.état.cooperator",
                "bayesian_network.état.defector"
            ]
        );
        let err = read(&format!("{base}  1st: {{node: move}}\n")).unwrap_err();
        assert!(err.contains("\"1st\" is not a name for a query"), "{err}");
    }

    #[test]
    fn a_probability_written_as_text_must_be_a_formula() {
        let err = read("nodes:\n  a: {states: [x, y], prior: ['0.5', 0.5]}\n").unwrap_err();
        assert_eq!(
            err,
            "bayesian_network.nodes.a.prior.x is the text \"0.5\": a probability is a number, \
             or a formula that starts with ="
        );
        let err = read("nodes:\n  a: {states: [x], prior: [1]}\n  b: {states: [y], parents: [a], cpt: {x: [yes]}}\n")
            .unwrap_err();
        assert!(
            err.starts_with("bayesian_network.nodes.b.cpt.x.y is the text \"yes\""),
            "{err}"
        );
        // The shape is checked first, with the old messages
        let err = read("nodes:\n  a: {states: [x, y], prior: ['=p']}\n").unwrap_err();
        assert_eq!(err, "Node 'a': prior length (1) must match states (2)");
    }

    /// A node with two parents is checked with formulas as with numbers: a
    /// parent state with a comma, and a row that names no combination
    #[test]
    fn a_table_of_two_parents_is_checked_when_it_is_read() {
        let net = |rain: &str, key: &str| {
            format!(
                "nodes:
  rain: {{states: [{rain}], prior: [=r, =1 - r]}}
  sprinkler: {{states: [off, on], prior: [0.6, 0.4]}}
  grass:
    states: [dry, wet]
    parents: [rain, sprinkler]
    cpt: {{'no, off': [1, 0], 'yes, off': [=0.2, =0.8], 'no, on': [0.1, 0.9], '{key}': [0.01, 0.99]}}
queries:
  wet: {{node: rain, evidence: {{grass: wet}}}}
"
            )
        };
        let network = read(&net("no, yes", "yes, on")).unwrap();
        assert_eq!(network.formulas(), ["=0.2", "=0.8", "=r", "=1 - r"]);
        let err = read(&net("no, 'yes,really'", "yes, on")).unwrap_err();
        assert!(err.contains("contains a comma"), "{err}");
        let err = read(&net("no, yes", "on, yes")).unwrap_err();
        assert!(
            err.contains("'on, yes' must name one state of each parent (rain, sprinkler)"),
            "{err}"
        );
    }

    #[test]
    fn a_name_that_is_not_a_result_says_what_the_network_gives() {
        let network = read(MISTAKE).unwrap();
        assert_eq!(
            network.why_not("bayesian_network.one.defectr"),
            "'bayesian_network.one.defectr' is not a result of the network: query 'one' gives \
             bayesian_network.one.cooperator, bayesian_network.one.defector"
        );
        for name in ["bayesian_network.three.defector", "bayesian_network.one"] {
            assert_eq!(
                network.why_not(name),
                format!("'{name}' is not a result of the network: its queries are none, one, two")
            );
        }
        let none = read("nodes:\n  a: {states: [x], prior: [1]}\n").unwrap();
        assert_eq!(
            none.why_not("bayesian_network.q.x"),
            "'bayesian_network.q.x' is not a result of the network: it has no queries, and each \
             query under bayesian_network.queries gives bayesian_network.<query>.<state>"
        );
        assert_eq!(none.result_names(), Vec::<String>::new());
    }

    #[test]
    fn evidence_that_cannot_happen_has_no_posterior() {
        // With no defectors, a partner who never errs cannot defect
        let network = read(MISTAKE).unwrap();
        let worked = network
            .work_out(|formula, _| -> Result<f64, String> {
                Ok(if formula.starts_with("=1") { 1.0 } else { 0.0 })
            })
            .unwrap();
        assert_eq!(
            worked.posteriors().unwrap_err(),
            "bayesian_network.queries.one: the evidence cannot happen in this network (its \
             probability is 0), so the query has no posterior"
        );
        // A query is checked by the engine too, when the network is not read
        let mut one = BayesianConfig::new("n")
            .with_node("a", BayesianNode::discrete(vec!["x"]).with_prior(vec![1.0]));
        one.queries.insert(
            "q".to_string(),
            Query {
                node: "a".to_string(),
                evidence: BTreeMap::from([("ghost".to_string(), "x".to_string())]),
            },
        );
        assert_eq!(
            one.posteriors().unwrap_err(),
            "bayesian_network.queries.q: the evidence names 'ghost', which is not a node of the \
             network (a)"
        );
    }
}
