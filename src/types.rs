use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents an included YAML file with an alias
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Include {
    pub file: String,
    pub r#as: String,
}

/// A variable in the YAML file
#[derive(Debug, Clone)]
pub struct Variable {
    pub path: String,
    pub value: Option<f64>,
    pub formula: Option<String>,
    /// The alias of the file this variable came from (None for main file)
    pub alias: Option<String>,
}

/// Parsed YAML data with includes
#[derive(Debug)]
pub struct ParsedYaml {
    pub includes: Vec<Include>,
    pub variables: HashMap<String, Variable>,
}

/// Context for formula evaluation
pub struct EvalContext {
    pub variables: HashMap<String, f64>,
}

impl Default for EvalContext {
    fn default() -> Self {
        Self::new()
    }
}

impl EvalContext {
    #[must_use]
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn set(&mut self, name: String, value: f64) {
        self.variables.insert(name, value);
    }
}
