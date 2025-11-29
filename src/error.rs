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

    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Excel export error: {0}")]
    Export(String),

    #[error("Excel import error: {0}")]
    Import(String),

    #[error("IO error: {0}")]
    IO(String),

    /// Rich formula error with context (v4.1.0)
    #[error("{}", .0.format_error())]
    Formula(FormulaErrorContext),
}

/// Rich error context for formula evaluation failures (v4.1.0)
#[derive(Debug, Clone)]
pub struct FormulaErrorContext {
    /// The original formula that failed
    pub formula: String,
    /// Location: table.column or scalar name
    pub location: String,
    /// What went wrong
    pub error: String,
    /// Suggestion for fixing (optional)
    pub suggestion: Option<String>,
    /// Available columns in context (for "did you mean?" suggestions)
    pub available_columns: Vec<String>,
}

impl FormulaErrorContext {
    pub fn new(formula: &str, location: &str, error: &str) -> Self {
        Self {
            formula: formula.to_string(),
            location: location.to_string(),
            error: error.to_string(),
            suggestion: None,
            available_columns: Vec::new(),
        }
    }

    pub fn with_suggestion(mut self, suggestion: &str) -> Self {
        self.suggestion = Some(suggestion.to_string());
        self
    }

    pub fn with_available_columns(mut self, columns: Vec<String>) -> Self {
        self.available_columns = columns;
        self
    }

    /// Find similar column names for "did you mean?" suggestions
    pub fn find_similar(&self, target: &str) -> Option<String> {
        let target_lower = target.to_lowercase();

        // Exact match (case-insensitive)
        for col in &self.available_columns {
            if col.to_lowercase() == target_lower {
                return Some(col.clone());
            }
        }

        // Prefix match
        for col in &self.available_columns {
            if col.to_lowercase().starts_with(&target_lower)
                || target_lower.starts_with(&col.to_lowercase())
            {
                return Some(col.clone());
            }
        }

        // Contains match
        for col in &self.available_columns {
            if col.to_lowercase().contains(&target_lower)
                || target_lower.contains(&col.to_lowercase())
            {
                return Some(col.clone());
            }
        }

        None
    }

    /// Format the error message with context
    pub fn format_error(&self) -> String {
        let mut msg = format!(
            "Formula error in '{}':\n  Formula: {}\n  Error: {}",
            self.location, self.formula, self.error
        );

        if let Some(ref suggestion) = self.suggestion {
            msg.push_str(&format!("\n  Suggestion: {}", suggestion));
        }

        if !self.available_columns.is_empty() && self.available_columns.len() <= 10 {
            msg.push_str(&format!(
                "\n  Available columns: {}",
                self.available_columns.join(", ")
            ));
        }

        msg
    }
}

/// Helper to create formula errors with context
pub fn formula_error(
    formula: &str,
    location: &str,
    error: &str,
    suggestion: Option<&str>,
) -> ForgeError {
    let mut ctx = FormulaErrorContext::new(formula, location, error);
    if let Some(s) = suggestion {
        ctx = ctx.with_suggestion(s);
    }
    ForgeError::Formula(ctx)
}
