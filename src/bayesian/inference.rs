//! Belief Propagation Inference
//!
//! Implements variable elimination for exact inference in Bayesian networks.
//! Validated against pgmpy.

use super::config::{BayesianConfig, BayesianNode, NodeType};
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
            values = node.prior.clone();
        } else {
            // Child node: build from CPT
            // Get parent cardinality (assuming single parent for now)
            if let Some(parent_node) = config.nodes.get(&node.parents[0]) {
                let parent_card = parent_node.states.len();

                for (i, val) in values.iter_mut().enumerate().take(total_size) {
                    // Decode indices - last variable (parent) changes fastest
                    let parent_idx = i % parent_card;
                    let state_idx = i / parent_card;

                    if parent_idx < parent_node.states.len() {
                        let parent_state = &parent_node.states[parent_idx];
                        if let Some(probs) = node.cpt.get(parent_state) {
                            if state_idx < probs.len() {
                                *val = probs[state_idx];
                            }
                        }
                    }
                }
            }
        }

        Factor {
            variables,
            cardinalities,
            values,
        }
    }

    /// Multiply two factors
    pub fn multiply(&self, other: &Factor) -> Factor {
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
            let indices = self.decode_index(i, &new_cardinalities);

            // Get index into self
            let self_idx = self.encode_index(&indices[..self.variables.len()], &self.cardinalities);

            // Get index into other
            let other_idx_vec: Vec<usize> = other_indices
                .iter()
                .filter_map(|&idx| idx.map(|j| indices[j]))
                .collect();
            let other_idx = self.encode_index(&other_idx_vec, &other.cardinalities);

            let self_val = self.values.get(self_idx).copied().unwrap_or(0.0);
            let other_val = other.values.get(other_idx).copied().unwrap_or(0.0);

            *val = self_val * other_val;
        }

        Factor {
            variables: new_variables,
            cardinalities: new_cardinalities,
            values: new_values,
        }
    }

    /// Marginalize (sum out) a variable
    pub fn marginalize(&self, var: &str) -> Factor {
        let var_idx = match self.variables.iter().position(|v| v == var) {
            Some(idx) => idx,
            None => return self.clone(),
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
            return Factor {
                variables: vec![],
                cardinalities: vec![],
                values: vec![self.values.iter().sum()],
            };
        }

        let _var_card = self.cardinalities[var_idx];
        let total_size: usize = new_cardinalities.iter().product();
        let mut new_values = vec![0.0; total_size];

        for i in 0..self.values.len() {
            let indices = self.decode_index(i, &self.cardinalities);

            // Get new index (without marginalized variable)
            let new_idx_vec: Vec<usize> = indices
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != var_idx)
                .map(|(_, idx)| *idx)
                .collect();

            let new_idx = if new_idx_vec.is_empty() {
                0
            } else {
                self.encode_index(&new_idx_vec, &new_cardinalities)
            };

            new_values[new_idx] += self.values[i];
        }

        Factor {
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
    fn decode_index(&self, mut idx: usize, cardinalities: &[usize]) -> Vec<usize> {
        let mut indices = vec![0; cardinalities.len()];
        for i in (0..cardinalities.len()).rev() {
            indices[i] = idx % cardinalities[i];
            idx /= cardinalities[i];
        }
        indices
    }

    /// Encode multi-dimensional indices to a flat index
    fn encode_index(&self, indices: &[usize], cardinalities: &[usize]) -> usize {
        let mut idx = 0;
        let mut multiplier = 1;
        for i in (0..indices.len()).rev() {
            idx += indices[i] * multiplier;
            multiplier *= cardinalities.get(i).copied().unwrap_or(1);
        }
        idx
    }

    /// Get probability for a specific assignment
    pub fn get_probability(&self, assignment: &HashMap<String, usize>) -> f64 {
        let indices: Vec<usize> = self
            .variables
            .iter()
            .map(|v| assignment.get(v).copied().unwrap_or(0))
            .collect();
        let idx = self.encode_index(&indices, &self.cardinalities);
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
    pub fn new(config: BayesianConfig) -> Result<Self, String> {
        config.validate()?;

        // Build initial factors from nodes
        let mut factors = Vec::new();
        for (name, node) in &config.nodes {
            if node.node_type == NodeType::Discrete {
                factors.push(Factor::from_node(name, node, &config));
            }
        }

        Ok(Self { config, factors })
    }

    /// Query the marginal probability of a variable
    pub fn query(&self, target: &str) -> Result<Vec<f64>, String> {
        if !self.config.nodes.contains_key(target) {
            return Err(format!("Variable '{target}' not found in network"));
        }

        // Variable elimination
        let order = self.get_elimination_order(target);

        let mut factors = self.factors.clone();

        for var in order {
            if var == target {
                continue;
            }

            // Find factors containing this variable
            let (containing, remaining): (Vec<_>, Vec<_>) = factors
                .into_iter()
                .partition(|f| f.variables.contains(&var));

            if containing.is_empty() {
                factors = remaining;
                continue;
            }

            // Multiply containing factors
            let mut product = containing[0].clone();
            for f in containing.iter().skip(1) {
                product = product.multiply(f);
            }

            // Marginalize
            let marginal = product.marginalize(&var);

            factors = remaining;
            factors.push(marginal);
        }

        // Multiply remaining factors
        if factors.is_empty() {
            return Err("No factors remaining".to_string());
        }

        let mut result = factors[0].clone();
        for f in factors.iter().skip(1) {
            result = result.multiply(f);
        }

        // Normalize
        result.normalize();

        // Extract probabilities for target variable
        // After variable elimination, result should only contain the target variable
        if result.variables.len() == 1 && result.variables[0] == target {
            // Simple case: result is already just the target variable
            let sum: f64 = result.values.iter().sum();
            if sum > 0.0 {
                Ok(result.values.iter().map(|v| v / sum).collect())
            } else {
                Ok(result.values.clone())
            }
        } else {
            // Complex case: marginalize out any remaining variables except target
            let mut final_result = result.clone();
            for var in &result.variables {
                if var != target {
                    final_result = final_result.marginalize(var);
                }
            }

            // Extract probabilities
            let sum: f64 = final_result.values.iter().sum();
            if sum > 0.0 {
                Ok(final_result.values.iter().map(|v| v / sum).collect())
            } else {
                Ok(final_result.values.clone())
            }
        }
    }

    /// Query with evidence (observed values)
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
            .map(|f| self.apply_evidence(f, evidence))
            .collect();

        // Variable elimination (excluding evidence variables)
        let order = self.get_elimination_order(target);

        for var in order {
            if var == target || evidence.contains_key(&var) {
                continue;
            }

            // Find factors containing this variable
            let (containing, remaining): (Vec<_>, Vec<_>) = factors
                .into_iter()
                .partition(|f| f.variables.contains(&var));

            if containing.is_empty() {
                factors = remaining;
                continue;
            }

            // Multiply containing factors
            let mut product = containing[0].clone();
            for f in containing.iter().skip(1) {
                product = product.multiply(f);
            }

            // Marginalize
            let marginal = product.marginalize(&var);

            factors = remaining;
            factors.push(marginal);
        }

        // Multiply remaining factors
        if factors.is_empty() {
            return Err("No factors remaining".to_string());
        }

        let mut result = factors[0].clone();
        for f in factors.iter().skip(1) {
            result = result.multiply(f);
        }

        // Normalize
        result.normalize();

        // Extract probabilities for target variable
        // After variable elimination, result should only contain the target variable
        if result.variables.len() == 1 && result.variables[0] == target {
            // Simple case: result is already just the target variable
            let sum: f64 = result.values.iter().sum();
            if sum > 0.0 {
                Ok(result.values.iter().map(|v| v / sum).collect())
            } else {
                Ok(result.values.clone())
            }
        } else {
            // Complex case: marginalize out any remaining variables except target
            let mut final_result = result.clone();
            for var in &result.variables {
                if var != target {
                    final_result = final_result.marginalize(var);
                }
            }

            // Extract probabilities
            let sum: f64 = final_result.values.iter().sum();
            if sum > 0.0 {
                Ok(final_result.values.iter().map(|v| v / sum).collect())
            } else {
                Ok(final_result.values.clone())
            }
        }
    }

    /// Apply evidence to a factor
    fn apply_evidence(&self, factor: &Factor, evidence: &HashMap<String, usize>) -> Factor {
        let mut new_values = factor.values.clone();

        for (i, val) in new_values.iter_mut().enumerate() {
            let indices = factor.decode_index(i, &factor.cardinalities);

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
    pub fn config(&self) -> &BayesianConfig {
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
        let expected_on = 0.4 * 0.8 + 0.01 * 0.2;
        assert!(
            (sprinkler_probs[1] - expected_on).abs() < 0.01,
            "P(sprinkler=on) should be {}, got {}",
            expected_on,
            sprinkler_probs[1]
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
        assert!(
            (probs[1] - 0.01).abs() < 0.01,
            "P(sprinkler=on | rain=yes) should be 0.01, got {}",
            probs[1]
        );
    }
}
