use thiserror::Error;

pub type ForgeResult<T> = Result<T, ForgeError>;

#[derive(Error, Debug)]
pub enum ForgeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Formula evaluation error: {0}")]
    Eval(String),

    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("Invalid formula syntax: {0}")]
    InvalidFormula(String),

    #[error("Validation error: {0}")]
    Validation(String),
}
