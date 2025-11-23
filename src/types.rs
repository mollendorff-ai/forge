use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a value with an optional formula
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormulaValue {
    pub value: Option<f64>,
    pub formula: Option<String>,
}

/// A variable in the YAML file
#[derive(Debug, Clone)]
pub struct Variable {
    pub path: String,
    pub value: Option<f64>,
    pub formula: Option<String>,
}

/// Context for formula evaluation
pub struct EvalContext {
    pub variables: HashMap<String, f64>,
}

impl EvalContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn set(&mut self, name: String, value: f64) {
        self.variables.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<f64> {
        self.variables.get(name).copied()
    }
}
