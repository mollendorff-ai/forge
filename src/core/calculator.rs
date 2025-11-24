use crate::error::{ForgeError, ForgeResult};
use crate::types::{EvalContext, Variable};
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use std::collections::HashMap;

/// Formula calculator with dependency resolution
pub struct Calculator {
    variables: HashMap<String, Variable>,
    context: EvalContext,
}

impl Calculator {
    #[must_use]
    pub fn new(variables: HashMap<String, Variable>) -> Self {
        Self {
            variables,
            context: EvalContext::new(),
        }
    }

    /// Find a variable by name, handling various naming conventions
    /// Supports: exact match, suffix match, underscore<->dot conversion
    /// Prefers shorter paths (more specific matches) to avoid ambiguity
    fn find_variable_name(&self, search_name: &str) -> Option<String> {
        // Try exact match first
        if self.variables.contains_key(search_name) {
            return Some(search_name.to_string());
        }

        // Try converting FIRST underscore to dot (most common case)
        // ltv_weighted_average → ltv.weighted_average
        if let Some(first_underscore) = search_name.find('_') {
            let with_first_dot = format!(
                "{}.{}",
                &search_name[..first_underscore],
                &search_name[first_underscore + 1..]
            );
            if self.variables.contains_key(&with_first_dot) {
                return Some(with_first_dot);
            }
        }

        // Try converting ALL underscores to dots
        // platform_take_rate → platform.take.rate
        let search_as_path = search_name.replace('_', ".");
        if self.variables.contains_key(&search_as_path) {
            return Some(search_as_path);
        }

        // Extract last component (after last underscore or dot)
        let last_component = search_name.rsplit(['_', '.']).next().unwrap_or(search_name);

        // Collect all potential matches, prefer shorter paths (more specific)
        let mut candidates: Vec<String> = Vec::new();

        for var_name in self.variables.keys() {
            // Match full name as suffix (highest priority)
            if var_name.ends_with(&format!(".{search_name}"))
                || var_name.ends_with(&format!(".{search_as_path}"))
            {
                candidates.push(var_name.clone());
            }
        }

        // If we found matches with full search name, use the shortest
        if !candidates.is_empty() {
            candidates.sort_by_key(std::string::String::len);
            return Some(candidates[0].clone());
        }

        // Try matching first and last components (for abbreviated refs like platform_take_rate)
        // This handles: platform_take_rate → platform_economics.take_rate
        if search_name.contains('_') {
            let parts: Vec<&str> = search_name.split('_').collect();
            if parts.len() >= 2 {
                let first = parts[0];
                let last = parts[parts.len() - 1];

                for var_name in self.variables.keys() {
                    let var_parts: Vec<&str> = var_name.split('.').collect();
                    if var_parts.len() >= 2 {
                        let var_last = var_parts[var_parts.len() - 1];
                        // Check if first component CONTAINS the search first part (platform in platform_economics)
                        // AND last component matches or ends with the search last part (rate matches take_rate)
                        if var_parts[0].contains(first)
                            && (var_last == last || var_last.ends_with(&format!("_{last}")))
                        {
                            candidates.push(var_name.clone());
                        }
                    }
                }
            }
        }

        if !candidates.is_empty() {
            candidates.sort_by_key(std::string::String::len);
            return Some(candidates[0].clone());
        }

        // Fall back to matching just last component only if nothing else matched
        for var_name in self.variables.keys() {
            if var_name.ends_with(&format!(".{last_component}")) {
                candidates.push(var_name.clone());
            }
        }

        if !candidates.is_empty() {
            candidates.sort_by_key(std::string::String::len);
            return Some(candidates[0].clone());
        }

        None
    }

    /// Same logic but for context (which stores calculated values)
    fn find_value_in_context(&self, search_name: &str) -> Option<f64> {
        // Try exact match first
        if let Some(&val) = self.context.variables.get(search_name) {
            return Some(val);
        }

        // Try converting FIRST underscore to dot (most common case)
        // ltv_weighted_average → ltv.weighted_average
        if let Some(first_underscore) = search_name.find('_') {
            let with_first_dot = format!(
                "{}.{}",
                &search_name[..first_underscore],
                &search_name[first_underscore + 1..]
            );
            if let Some(&val) = self.context.variables.get(&with_first_dot) {
                return Some(val);
            }
        }

        // Try converting ALL underscores to dots
        let search_as_path = search_name.replace('_', ".");
        if let Some(&val) = self.context.variables.get(&search_as_path) {
            return Some(val);
        }

        // Extract last component (after last underscore or dot)
        let last_component = search_name.rsplit(['_', '.']).next().unwrap_or(search_name);

        // Collect all potential matches, prefer shorter paths (more specific)
        let mut candidates: Vec<(String, f64)> = Vec::new();

        for (var_name, &value) in &self.context.variables {
            // Match full name as suffix (highest priority)
            if var_name.ends_with(&format!(".{search_name}"))
                || var_name.ends_with(&format!(".{search_as_path}"))
            {
                candidates.push((var_name.clone(), value));
            }
        }

        // If we found matches with full search name, use the shortest
        if !candidates.is_empty() {
            candidates.sort_by_key(|(name, _)| name.len());
            return Some(candidates[0].1);
        }

        // Try matching first and last components (for abbreviated refs like platform_take_rate)
        // This handles: platform_take_rate → platform_economics.take_rate
        if search_name.contains('_') {
            let parts: Vec<&str> = search_name.split('_').collect();
            if parts.len() >= 2 {
                let first = parts[0];
                let last = parts[parts.len() - 1];

                for (var_name, &value) in &self.context.variables {
                    let var_parts: Vec<&str> = var_name.split('.').collect();
                    if var_parts.len() >= 2 {
                        let var_last = var_parts[var_parts.len() - 1];
                        // Check if first component CONTAINS the search first part (platform in platform_economics)
                        // AND last component matches or ends with the search last part (rate matches take_rate)
                        if var_parts[0].contains(first)
                            && (var_last == last || var_last.ends_with(&format!("_{last}")))
                        {
                            candidates.push((var_name.clone(), value));
                        }
                    }
                }
            }
        }

        if !candidates.is_empty() {
            candidates.sort_by_key(|(name, _)| name.len());
            return Some(candidates[0].1);
        }

        // Fall back to matching just last component only if nothing else matched
        for (var_name, &value) in &self.context.variables {
            if var_name.ends_with(&format!(".{last_component}")) {
                candidates.push((var_name.clone(), value));
            }
        }

        if !candidates.is_empty() {
            candidates.sort_by_key(|(name, _)| name.len());
            return Some(candidates[0].1);
        }

        None
    }

    /// Calculate all formulas in dependency order
    pub fn calculate_all(&mut self) -> ForgeResult<HashMap<String, f64>> {
        // Build dependency graph
        let graph = self.build_dependency_graph()?;

        // Topological sort to get calculation order
        let order = toposort(&graph, None).map_err(|_| {
            ForgeError::CircularDependency("Circular dependency detected in formulas".to_string())
        })?;

        let mut results = HashMap::new();

        // Calculate in dependency order
        for node_idx in order {
            if let Some(var_name) = graph.node_weight(node_idx) {
                if let Some(var) = self.variables.get(var_name) {
                    if let Some(formula) = &var.formula {
                        let value = self.evaluate_formula(formula)?;
                        self.context.set(var_name.clone(), value);
                        results.insert(var_name.clone(), value);
                    } else if let Some(value) = var.value {
                        self.context.set(var_name.clone(), value);
                        results.insert(var_name.clone(), value);
                    }
                }
            }
        }

        Ok(results)
    }

    /// Build dependency graph from formula references
    fn build_dependency_graph(&self) -> ForgeResult<DiGraph<String, ()>> {
        let mut graph = DiGraph::new();
        let mut node_indices = HashMap::new();

        // Create nodes for all variables
        for var_name in self.variables.keys() {
            let idx = graph.add_node(var_name.clone());
            node_indices.insert(var_name.clone(), idx);
        }

        // Add edges for dependencies
        for (var_name, var) in &self.variables {
            if let Some(formula) = &var.formula {
                let deps = self.extract_dependencies(formula)?;
                for dep in deps {
                    if let (Some(&from_idx), Some(&to_idx)) =
                        (node_indices.get(&dep), node_indices.get(var_name))
                    {
                        graph.add_edge(from_idx, to_idx, ());
                    }
                }
            }
        }

        Ok(graph)
    }

    /// Extract variable dependencies from formula string
    fn extract_dependencies(&self, formula: &str) -> ForgeResult<Vec<String>> {
        // Simple regex-based extraction (can be improved)
        let formula = formula.trim_start_matches('=');
        let mut deps = Vec::new();

        // Extract all words (variable names), including @ for cross-file refs
        for word in
            formula.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.' && c != '@')
        {
            if !word.is_empty() && !word.chars().next().unwrap().is_numeric() {
                if let Some(var_name) = self.find_variable_name(word) {
                    if !deps.contains(&var_name) {
                        deps.push(var_name);
                    }
                }
            }
        }

        Ok(deps)
    }

    /// Evaluate a formula expression
    fn evaluate_formula(&self, formula: &str) -> ForgeResult<f64> {
        let formula = formula.trim_start_matches('=').trim();

        // Replace variable names with values
        let mut expr = formula.to_string();

        // Extract all potential variable names from the formula, including @ for cross-file refs
        let words: Vec<&str> = formula
            .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.' && c != '@')
            .filter(|w| !w.is_empty() && !w.chars().next().unwrap().is_numeric())
            .collect();

        // Replace each variable name with its value
        for word in words {
            if let Some(value) = self.find_value_in_context(word) {
                expr = expr.replace(word, &value.to_string());
            }
        }

        // Evaluate using meval
        meval::eval_str(&expr)
            .map_err(|e| ForgeError::Eval(format!("Failed to evaluate '{expr}': {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_calculation() {
        let mut vars = HashMap::new();

        vars.insert(
            "base".to_string(),
            Variable {
                path: "base".to_string(),
                value: Some(100.0),
                formula: None,
                alias: None,
            },
        );

        vars.insert(
            "result".to_string(),
            Variable {
                path: "result".to_string(),
                value: None,
                formula: Some("=base * 2".to_string()),
                alias: None,
            },
        );

        let mut calc = Calculator::new(vars);
        let results = calc.calculate_all().unwrap();

        assert_eq!(results.get("result"), Some(&200.0));
    }
}
