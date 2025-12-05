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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formula_error_context_new() {
        let ctx = FormulaErrorContext::new("=SUM(a)", "test.col", "undefined reference");
        assert_eq!(ctx.formula, "=SUM(a)");
        assert_eq!(ctx.location, "test.col");
        assert_eq!(ctx.error, "undefined reference");
        assert!(ctx.suggestion.is_none());
        assert!(ctx.available_columns.is_empty());
    }

    #[test]
    fn test_formula_error_context_with_suggestion() {
        let ctx =
            FormulaErrorContext::new("=SUM(a)", "test.col", "error").with_suggestion("use SUM(b)");
        assert_eq!(ctx.suggestion, Some("use SUM(b)".to_string()));
    }

    #[test]
    fn test_formula_error_context_with_available_columns() {
        let ctx = FormulaErrorContext::new("=SUM(a)", "test.col", "error")
            .with_available_columns(vec!["col1".to_string(), "col2".to_string()]);
        assert_eq!(ctx.available_columns, vec!["col1", "col2"]);
    }

    #[test]
    fn test_formula_error_context_find_similar_exact() {
        let ctx = FormulaErrorContext::new("=SUM(a)", "test.col", "error")
            .with_available_columns(vec!["Revenue".to_string(), "Costs".to_string()]);

        // Case-insensitive exact match
        assert_eq!(ctx.find_similar("revenue"), Some("Revenue".to_string()));
        assert_eq!(ctx.find_similar("COSTS"), Some("Costs".to_string()));
    }

    #[test]
    fn test_formula_error_context_find_similar_prefix() {
        let ctx = FormulaErrorContext::new("=SUM(a)", "test.col", "error")
            .with_available_columns(vec!["revenue_total".to_string()]);

        // Prefix match
        assert_eq!(
            ctx.find_similar("revenue"),
            Some("revenue_total".to_string())
        );
    }

    #[test]
    fn test_formula_error_context_find_similar_contains() {
        let ctx = FormulaErrorContext::new("=SUM(a)", "test.col", "error")
            .with_available_columns(vec!["total_revenue_ytd".to_string()]);

        // Contains match
        assert_eq!(
            ctx.find_similar("revenue"),
            Some("total_revenue_ytd".to_string())
        );
    }

    #[test]
    fn test_formula_error_context_find_similar_none() {
        let ctx = FormulaErrorContext::new("=SUM(a)", "test.col", "error")
            .with_available_columns(vec!["col1".to_string(), "col2".to_string()]);

        // No match
        assert_eq!(ctx.find_similar("xyz"), None);
    }

    #[test]
    fn test_formula_error_context_format_error_basic() {
        let ctx = FormulaErrorContext::new("=SUM(a)", "test.col", "undefined reference");
        let msg = ctx.format_error();

        assert!(msg.contains("test.col"));
        assert!(msg.contains("=SUM(a)"));
        assert!(msg.contains("undefined reference"));
        assert!(!msg.contains("Suggestion"));
        assert!(!msg.contains("Available"));
    }

    #[test]
    fn test_formula_error_context_format_error_with_suggestion() {
        let ctx = FormulaErrorContext::new("=SUM(a)", "test.col", "error")
            .with_suggestion("Try using SUM(b)");
        let msg = ctx.format_error();

        assert!(msg.contains("Suggestion: Try using SUM(b)"));
    }

    #[test]
    fn test_formula_error_context_format_error_with_columns() {
        let ctx = FormulaErrorContext::new("=SUM(a)", "test.col", "error")
            .with_available_columns(vec!["col1".to_string(), "col2".to_string()]);
        let msg = ctx.format_error();

        assert!(msg.contains("Available columns: col1, col2"));
    }

    #[test]
    fn test_formula_error_context_format_error_many_columns() {
        // More than 10 columns should not be shown
        let ctx = FormulaErrorContext::new("=SUM(a)", "test.col", "error")
            .with_available_columns((0..15).map(|i| format!("col{}", i)).collect::<Vec<_>>());
        let msg = ctx.format_error();

        // Should NOT show columns when there are too many
        assert!(!msg.contains("Available columns"));
    }

    #[test]
    fn test_formula_error_helper() {
        let err = formula_error("=SUM(a)", "test.col", "error", Some("fix it"));
        if let ForgeError::Formula(ctx) = err {
            assert_eq!(ctx.formula, "=SUM(a)");
            assert_eq!(ctx.suggestion, Some("fix it".to_string()));
        } else {
            panic!("Expected ForgeError::Formula");
        }
    }

    #[test]
    fn test_formula_error_helper_no_suggestion() {
        let err = formula_error("=SUM(a)", "test.col", "error", None);
        if let ForgeError::Formula(ctx) = err {
            assert!(ctx.suggestion.is_none());
        } else {
            panic!("Expected ForgeError::Formula");
        }
    }

    #[test]
    fn test_forge_error_display() {
        // Test Display implementation for each variant
        let io_err = ForgeError::IO("file not found".to_string());
        assert!(io_err.to_string().contains("file not found"));

        let parse_err = ForgeError::Parse("invalid syntax".to_string());
        assert!(parse_err.to_string().contains("invalid syntax"));

        let eval_err = ForgeError::Eval("division by zero".to_string());
        assert!(eval_err.to_string().contains("division by zero"));

        let circular_err = ForgeError::CircularDependency("A -> B -> A".to_string());
        assert!(circular_err.to_string().contains("A -> B -> A"));

        let validation_err = ForgeError::Validation("schema mismatch".to_string());
        assert!(validation_err.to_string().contains("schema mismatch"));

        let export_err = ForgeError::Export("xlsx write failed".to_string());
        assert!(export_err.to_string().contains("xlsx write failed"));

        let import_err = ForgeError::Import("xlsx read failed".to_string());
        assert!(import_err.to_string().contains("xlsx read failed"));
    }

    #[test]
    fn test_forge_error_formula_display() {
        let ctx = FormulaErrorContext::new("=SUM(a)", "test.col", "undefined reference")
            .with_suggestion("use SUM(b)");
        let err = ForgeError::Formula(ctx);
        let msg = err.to_string();

        assert!(msg.contains("test.col"));
        assert!(msg.contains("=SUM(a)"));
        assert!(msg.contains("undefined reference"));
        assert!(msg.contains("use SUM(b)"));
    }

    #[test]
    fn test_forge_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let forge_err: ForgeError = io_err.into();
        assert!(matches!(forge_err, ForgeError::Io(_)));
    }

    #[test]
    fn test_forge_error_from_yaml_error() {
        // Create an invalid YAML to trigger a parse error
        let invalid_yaml = ":\n  : invalid";
        let yaml_result: Result<serde_yaml::Value, _> = serde_yaml::from_str(invalid_yaml);
        assert!(yaml_result.is_err());

        let yaml_err = yaml_result.unwrap_err();
        let forge_err: ForgeError = yaml_err.into();
        assert!(matches!(forge_err, ForgeError::Yaml(_)));
        assert!(forge_err.to_string().contains("YAML parsing error"));
    }
}
