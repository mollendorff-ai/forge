//! Belief Propagation Inference
//!
//! Implements variable elimination for exact inference in Bayesian networks.
//! A unit test checks a small network against a hand calculation of a pgmpy example.

use super::config::{BayesianConfig, BayesianNode};
use std::collections::HashMap;

/// Factor (potential function) for inference
#[derive(Debug, Clone)]
pub struct Factor {
    /// Variables in this factor
    pub variables: Vec<String>,
    /// Cardinalities of each variable
    pub cardinalities: Vec<usize>,
    /// Probability values (flattened)
    pub values: Vec<f64>,
}

impl Factor {
    /// Create a factor from a node's CPT
    #[must_use]
    pub fn from_node(name: &str, node: &BayesianNode, config: &BayesianConfig) -> Self {
        let mut variables = vec![name.to_string()];
        let mut cardinalities = vec![node.states.len()];

        // Add parent variables
        for parent in &node.parents {
            if let Some(parent_node) = config.nodes.get(parent) {
                variables.push(parent.clone());
                cardinalities.push(parent_node.states.len());
            }
        }

        // Build values array
        let total_size: usize = cardinalities.iter().product();
        let mut values = vec![0.0; total_size];

        if node.is_root() {
            // Root node: just prior
            values.clone_from(&node.prior);
        } else {
            // Child node: one CPT row per combination of parent states. The
            // flat index runs over [node, parent 1, parent 2, ...], the last
            // variable fastest.
            let parent_nodes: Vec<&BayesianNode> = node
                .parents
                .iter()
                .filter_map(|p| config.nodes.get(p))
                .collect();
            for (i, val) in values.iter_mut().enumerate() {
                let indices = Self::decode_index(i, &cardinalities);
                let parent_states: Vec<&str> = parent_nodes
                    .iter()
                    .zip(&indices[1..])
                    .map(|(p, &idx)| p.states[idx].as_str())
                    .collect();
                if let Some(row) = node.cpt_row(&parent_states) {
                    *val = row.get(indices[0]).copied().unwrap_or(0.0);
                }
            }
        }

        Self {
            variables,
            cardinalities,
            values,
        }
    }

    /// Multiply two factors
    #[must_use]
    pub fn multiply(&self, other: &Self) -> Self {
        // Find common and unique variables
        let mut new_variables = self.variables.clone();
        let mut new_cardinalities = self.cardinalities.clone();

        let mut other_indices: Vec<Option<usize>> = vec![None; other.variables.len()];

        for (i, var) in other.variables.iter().enumerate() {
            if let Some(pos) = self.variables.iter().position(|v| v == var) {
                other_indices[i] = Some(pos);
            } else {
                new_variables.push(var.clone());
                new_cardinalities.push(other.cardinalities[i]);
                other_indices[i] = Some(new_variables.len() - 1);
            }
        }

        let total_size: usize = new_cardinalities.iter().product();
        let mut new_values = vec![0.0; total_size];

        // Compute product
        for (i, val) in new_values.iter_mut().enumerate() {
            let indices = Self::decode_index(i, &new_cardinalities);

            // Get index into self
            let self_idx =
                Self::encode_index(&indices[..self.variables.len()], &self.cardinalities);

            // Get index into other
            let other_idx_vec: Vec<usize> = other_indices
                .iter()
                .filter_map(|&idx| idx.map(|j| indices[j]))
                .collect();
            let other_idx = Self::encode_index(&other_idx_vec, &other.cardinalities);

            let self_val = self.values.get(self_idx).copied().unwrap_or(0.0);
            let other_val = other.values.get(other_idx).copied().unwrap_or(0.0);

            *val = self_val * other_val;
        }

        Self {
            variables: new_variables,
            cardinalities: new_cardinalities,
            values: new_values,
        }
    }

    /// Marginalize (sum out) a variable
    #[must_use]
    pub fn marginalize(&self, var: &str) -> Self {
        let Some(var_idx) = self.variables.iter().position(|v| v == var) else {
            return self.clone();
        };

        let new_variables: Vec<String> = self
            .variables
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != var_idx)
            .map(|(_, v)| v.clone())
            .collect();

        let new_cardinalities: Vec<usize> = self
            .cardinalities
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != var_idx)
            .map(|(_, c)| *c)
            .collect();

        if new_variables.is_empty() {
            // Marginalizing the last variable
            return Self {
                variables: vec![],
                cardinalities: vec![],
                values: vec![self.values.iter().sum()],
            };
        }

        let total_size: usize = new_cardinalities.iter().product();
        let mut new_values = vec![0.0; total_size];

        for i in 0..self.values.len() {
            let indices = Self::decode_index(i, &self.cardinalities);

            // Get new index (without marginalized variable)
            let new_idx_vec: Vec<usize> = indices
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != var_idx)
                .map(|(_, idx)| *idx)
                .collect();

            new_values[Self::encode_index(&new_idx_vec, &new_cardinalities)] += self.values[i];
        }

        Self {
            variables: new_variables,
            cardinalities: new_cardinalities,
            values: new_values,
        }
    }

    /// Normalize the factor
    pub fn normalize(&mut self) {
        let sum: f64 = self.values.iter().sum();
        if sum > 0.0 {
            for v in &mut self.values {
                *v /= sum;
            }
        }
    }

    /// Decode a flat index to multi-dimensional indices
    fn decode_index(mut idx: usize, cardinalities: &[usize]) -> Vec<usize> {
        let mut indices = vec![0; cardinalities.len()];
        for i in (0..cardinalities.len()).rev() {
            indices[i] = idx % cardinalities[i];
            idx /= cardinalities[i];
        }
        indices
    }

    /// Encode multi-dimensional indices to a flat index
    fn encode_index(indices: &[usize], cardinalities: &[usize]) -> usize {
        let mut idx = 0;
        let mut multiplier = 1;
        for i in (0..indices.len()).rev() {
            idx += indices[i] * multiplier;
            multiplier *= cardinalities.get(i).copied().unwrap_or(1);
        }
        idx
    }

    /// Get probability for a specific assignment
    #[must_use]
    pub fn get_probability(&self, assignment: &HashMap<String, usize>) -> f64 {
        let indices: Vec<usize> = self
            .variables
            .iter()
            .map(|v| assignment.get(v).copied().unwrap_or(0))
            .collect();
        let idx = Self::encode_index(&indices, &self.cardinalities);
        self.values.get(idx).copied().unwrap_or(0.0)
    }
}

/// Belief Propagation (Variable Elimination) for exact inference
pub struct BeliefPropagation {
    config: BayesianConfig,
    factors: Vec<Factor>,
}

impl BeliefPropagation {
    /// Create a new belief propagation engine
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid.
    pub fn new(config: BayesianConfig) -> Result<Self, String> {
        config.validate()?;

        // Build initial factors from nodes
        let mut factors = Vec::new();
        for (name, node) in &config.nodes {
            // validate() refuses any other type of node
            factors.push(Factor::from_node(name, node, &config));
        }

        Ok(Self { config, factors })
    }

    /// Query the marginal probability of a variable
    ///
    /// # Errors
    ///
    /// Returns an error if the target variable is not found or no factors remain.
    pub fn query(&self, target: &str) -> Result<Vec<f64>, String> {
        self.query_with_evidence(target, &HashMap::new())
    }

    /// Query with evidence (observed values)
    ///
    /// # Errors
    ///
    /// Returns an error if the target variable is not found or no factors remain.
    pub fn query_with_evidence(
        &self,
        target: &str,
        evidence: &HashMap<String, usize>,
    ) -> Result<Vec<f64>, String> {
        if !self.config.nodes.contains_key(target) {
            return Err(format!("Variable '{target}' not found in network"));
        }

        // Apply evidence to factors
        let mut factors: Vec<Factor> = self
            .factors
            .iter()
            .map(|f| Self::apply_evidence(f, evidence))
            .collect();

        // Variable elimination (excluding evidence variables)
        for var in self.get_elimination_order(target) {
            if evidence.contains_key(&var) {
                continue;
            }

            // Multiply the factors containing this variable, then sum it out
            let (containing, remaining): (Vec<_>, Vec<_>) = factors
                .into_iter()
                .partition(|f| f.variables.contains(&var));
            factors = remaining;
            if let Some(product) = containing.into_iter().reduce(|a, b| a.multiply(&b)) {
                factors.push(product.marginalize(&var));
            }
        }

        // Multiply what remains, and sum out all but the target
        let mut result = factors
            .into_iter()
            .reduce(|a, b| a.multiply(&b))
            .ok_or_else(|| "No factors remaining".to_string())?;
        for var in result.variables.clone() {
            if var != target {
                result = result.marginalize(&var);
            }
        }
        result.normalize();
        Ok(result.values)
    }

    /// Apply evidence to a factor
    fn apply_evidence(factor: &Factor, evidence: &HashMap<String, usize>) -> Factor {
        let mut new_values = factor.values.clone();

        for (i, val) in new_values.iter_mut().enumerate() {
            let indices = Factor::decode_index(i, &factor.cardinalities);

            for (var_idx, var) in factor.variables.iter().enumerate() {
                if let Some(&ev_val) = evidence.get(var) {
                    if indices[var_idx] != ev_val {
                        *val = 0.0;
                        break;
                    }
                }
            }
        }

        Factor {
            variables: factor.variables.clone(),
            cardinalities: factor.cardinalities.clone(),
            values: new_values,
        }
    }

    /// Get elimination order (simple reverse topological)
    fn get_elimination_order(&self, exclude: &str) -> Vec<String> {
        let mut order = self.config.topological_order();
        order.reverse();
        order.retain(|v| v != exclude);
        order
    }

    /// Get the configuration
    #[must_use]
    pub const fn config(&self) -> &BayesianConfig {
        &self.config
    }
}

#[cfg(test)]
mod inference_tests {
    use super::*;

    fn create_simple_network() -> BayesianConfig {
        // Rain -> Sprinkler
        //    \-> Wet Grass <- Sprinkler
        BayesianConfig::new("Sprinkler")
            .with_node(
                "rain",
                BayesianNode::discrete(vec!["no", "yes"]).with_prior(vec![0.8, 0.2]),
            )
            .with_node(
                "sprinkler",
                BayesianNode::discrete(vec!["off", "on"])
                    .with_parents(vec!["rain"])
                    .with_cpt_entry("no", vec![0.6, 0.4])
                    .with_cpt_entry("yes", vec![0.99, 0.01]),
            )
    }

    #[test]
    fn test_prior_query() {
        let config = create_simple_network();
        let bp = BeliefPropagation::new(config).unwrap();

        let rain_probs = bp.query("rain").unwrap();
        assert!(
            (rain_probs[0] - 0.8).abs() < 0.01,
            "P(rain=no) should be 0.8"
        );
        assert!(
            (rain_probs[1] - 0.2).abs() < 0.01,
            "P(rain=yes) should be 0.2"
        );
    }

    #[test]
    fn test_marginal_query() {
        let config = create_simple_network();
        let bp = BeliefPropagation::new(config).unwrap();

        let sprinkler_probs = bp.query("sprinkler").unwrap();

        // P(sprinkler=on) = P(sprinkler=on|rain=no)*P(rain=no) + P(sprinkler=on|rain=yes)*P(rain=yes)
        //                 = 0.4 * 0.8 + 0.01 * 0.2 = 0.32 + 0.002 = 0.322
        let expected_on = 0.4f64.mul_add(0.8, 0.01 * 0.2);
        let on = sprinkler_probs[1];
        assert!(
            (on - expected_on).abs() < 0.01,
            "P(sprinkler=on) should be {expected_on}, got {on}"
        );
    }

    #[test]
    fn test_evidence_query() {
        let config = create_simple_network();
        let bp = BeliefPropagation::new(config).unwrap();

        // Query P(sprinkler | rain=yes)
        let mut evidence = HashMap::new();
        evidence.insert("rain".to_string(), 1); // yes

        let probs = bp.query_with_evidence("sprinkler", &evidence).unwrap();

        // P(sprinkler=on | rain=yes) = 0.01
        let on = probs[1];
        assert!(
            (on - 0.01).abs() < 0.01,
            "P(sprinkler=on | rain=yes) should be 0.01, got {on}"
        );
    }

    fn factor(variables: &[&str], cardinalities: &[usize], values: &[f64]) -> Factor {
        Factor {
            variables: variables.iter().map(ToString::to_string).collect(),
            cardinalities: cardinalities.to_vec(),
            values: values.to_vec(),
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn an_unknown_variable_is_refused() {
        let bp = BeliefPropagation::new(create_simple_network()).unwrap();
        let message = "Variable 'ghost' not found in network";
        assert_eq!(bp.query("ghost").unwrap_err(), message);
        assert_eq!(
            bp.query_with_evidence("ghost", &HashMap::new())
                .unwrap_err(),
            message
        );
    }

    #[test]
    fn an_engine_without_factors_has_no_answer() {
        let bp = BeliefPropagation {
            config: create_simple_network(),
            factors: vec![],
        };
        assert_eq!(bp.query("rain").unwrap_err(), "No factors remaining");
        assert_eq!(bp.config().name, "Sprinkler");
    }

    #[test]
    fn evidence_on_the_target_pins_it_and_impossible_evidence_gives_zeros() {
        let config = create_simple_network();
        let bp = BeliefPropagation::new(config.clone()).unwrap();
        let pinned = bp
            .query_with_evidence("rain", &HashMap::from([("rain".to_string(), 1)]))
            .unwrap();
        assert!(close(pinned[0], 0.0) && close(pinned[1], 1.0), "{pinned:?}");

        // P(sprinkler=on | rain=yes) is 0, so that evidence cannot happen
        let impossible = config.with_node(
            "sprinkler",
            BayesianNode::discrete(vec!["off", "on"])
                .with_parents(vec!["rain"])
                .with_cpt_entry("no", vec![0.6, 0.4])
                .with_cpt_entry("yes", vec![1.0, 0.0]),
        );
        let bp = BeliefPropagation::new(impossible).unwrap();
        let evidence = HashMap::from([("rain".to_string(), 1), ("sprinkler".to_string(), 1)]);
        assert_eq!(
            bp.query_with_evidence("rain", &evidence).unwrap(),
            [0.0, 0.0]
        );
    }

    #[test]
    fn a_factor_from_a_child_reads_its_cpt_by_parent_state() {
        let config = create_simple_network();
        let node = &config.nodes["sprinkler"];
        let f = Factor::from_node("sprinkler", node, &config);
        assert_eq!(f.variables, ["sprinkler", "rain"]);
        assert_eq!(f.cardinalities, [2, 2]);
        // [sprinkler, rain], rain fastest: off|no, off|yes, on|no, on|yes
        assert_eq!(f.values, [0.6, 0.99, 0.4, 0.01]);
        let assignment = HashMap::from([("sprinkler".to_string(), 1), ("rain".to_string(), 0)]);
        assert!(close(f.get_probability(&assignment), 0.4));
        // A missing variable counts as state 0; a missing cell as 0
        assert!(close(f.get_probability(&HashMap::new()), 0.6));
    }

    #[test]
    fn a_parent_missing_from_the_network_and_a_missing_cpt_row_give_zeros() {
        let config = create_simple_network();
        let orphan = BayesianNode::discrete(vec!["a", "b"])
            .with_parents(vec!["ghost"])
            .with_cpt_entry("x", vec![0.5, 0.5]);
        let f = Factor::from_node("orphan", &orphan, &config);
        assert_eq!(f.variables, ["orphan"]);
        assert_eq!(f.values, [0.0, 0.0]);

        let node = BayesianNode::discrete(vec!["off", "on"])
            .with_parents(vec!["rain"])
            .with_cpt_entry("no", vec![0.6, 0.4]);
        let f = Factor::from_node("sprinkler", &node, &config);
        assert_eq!(f.values, [0.6, 0.0, 0.4, 0.0]);
    }

    #[test]
    fn multiply_joins_factors_on_their_shared_variable() {
        let a = factor(&["x"], &[2], &[0.3, 0.7]);
        let b = factor(&["y", "x"], &[2, 2], &[1.0, 2.0, 3.0, 4.0]);
        let p = a.multiply(&b);
        assert_eq!(p.variables, ["x", "y"]);
        // x fastest-last: (x0,y0) (x0,y1) (x1,y0) (x1,y1)
        let want = [0.3, 0.9, 1.4, 2.8];
        assert!(
            p.values.iter().zip(want).all(|(g, w)| close(*g, w)),
            "{:?}",
            p.values
        );
        // A factor with too few values reads the missing cells as 0
        let short = factor(&["x"], &[2], &[0.5]);
        let q = short.multiply(&a);
        assert!(
            close(q.values[0], 0.15) && close(q.values[1], 0.0),
            "{:?}",
            q.values
        );
    }

    #[test]
    fn marginalize_sums_a_variable_out() {
        let f = factor(&["x", "y"], &[2, 2], &[0.1, 0.2, 0.3, 0.4]);
        let no_y = f.marginalize("y");
        assert_eq!(no_y.variables, ["x"]);
        assert!(close(no_y.values[0], 0.3) && close(no_y.values[1], 0.7));
        let no_x = f.marginalize("x");
        assert!(close(no_x.values[0], 0.4) && close(no_x.values[1], 0.6));
        // The last variable leaves one number, and an absent variable nothing
        let total = no_y.marginalize("x");
        assert!(total.variables.is_empty() && close(total.values[0], 1.0));
        assert_eq!(f.marginalize("z").values, f.values);
    }

    #[test]
    fn normalize_scales_to_one_and_leaves_zeros_alone() {
        let mut f = factor(&["x"], &[2], &[1.0, 3.0]);
        f.normalize();
        assert!(close(f.values[0], 0.25) && close(f.values[1], 0.75));
        let mut zero = factor(&["x"], &[2], &[0.0, 0.0]);
        zero.normalize();
        assert_eq!(zero.values, [0.0, 0.0]);
    }
}
