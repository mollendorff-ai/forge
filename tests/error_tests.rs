//! Error handling tests
//! ADR-004: 100% coverage required

use royalbit_forge::error::{formula_error, ForgeError, FormulaErrorContext};

#[test]
fn test_formula_error_context_new() {
    let ctx = FormulaErrorContext::new("=A + B", "table.profit", "Variable not found");
    assert_eq!(ctx.formula, "=A + B");
    assert_eq!(ctx.location, "table.profit");
    assert_eq!(ctx.error, "Variable not found");
    assert!(ctx.suggestion.is_none());
    assert!(ctx.available_columns.is_empty());
}

#[test]
fn test_formula_error_context_with_suggestion() {
    let ctx = FormulaErrorContext::new("=A + B", "table.profit", "Error")
        .with_suggestion("Check variable names");
    assert_eq!(ctx.suggestion, Some("Check variable names".to_string()));
}

#[test]
fn test_formula_error_context_with_available_columns() {
    let ctx = FormulaErrorContext::new("=A + B", "table.profit", "Error")
        .with_available_columns(vec!["revenue".to_string(), "costs".to_string()]);
    assert_eq!(ctx.available_columns, vec!["revenue", "costs"]);
}

#[test]
fn test_formula_error_context_find_similar_exact_match() {
    let ctx = FormulaErrorContext::new("=A + B", "loc", "err")
        .with_available_columns(vec!["Revenue".to_string(), "Costs".to_string()]);

    // Case-insensitive exact match
    assert_eq!(ctx.find_similar("revenue"), Some("Revenue".to_string()));
    assert_eq!(ctx.find_similar("COSTS"), Some("Costs".to_string()));
}

#[test]
fn test_formula_error_context_find_similar_prefix_match() {
    let ctx = FormulaErrorContext::new("=A + B", "loc", "err")
        .with_available_columns(vec!["total_revenue".to_string(), "net_profit".to_string()]);

    // Prefix match
    assert_eq!(ctx.find_similar("total"), Some("total_revenue".to_string()));
    assert_eq!(ctx.find_similar("net"), Some("net_profit".to_string()));
}

#[test]
fn test_formula_error_context_find_similar_contains_match() {
    let ctx = FormulaErrorContext::new("=A + B", "loc", "err")
        .with_available_columns(vec!["gross_revenue_2025".to_string()]);

    // Contains match
    assert_eq!(
        ctx.find_similar("revenue"),
        Some("gross_revenue_2025".to_string())
    );
}

#[test]
fn test_formula_error_context_find_similar_no_match() {
    let ctx = FormulaErrorContext::new("=A + B", "loc", "err")
        .with_available_columns(vec!["revenue".to_string(), "costs".to_string()]);

    // No match
    assert!(ctx.find_similar("xyz").is_none());
}

#[test]
fn test_formula_error_context_format_error_basic() {
    let ctx = FormulaErrorContext::new("=A + B", "table.profit", "Variable not found");
    let formatted = ctx.format_error();

    assert!(formatted.contains("table.profit"));
    assert!(formatted.contains("=A + B"));
    assert!(formatted.contains("Variable not found"));
}

#[test]
fn test_formula_error_context_format_error_with_suggestion() {
    let ctx = FormulaErrorContext::new("=A + B", "table.profit", "Error")
        .with_suggestion("Use revenue instead of A");
    let formatted = ctx.format_error();

    assert!(formatted.contains("Suggestion: Use revenue instead of A"));
}

#[test]
fn test_formula_error_context_format_error_with_columns() {
    let ctx = FormulaErrorContext::new("=A + B", "table.profit", "Error")
        .with_available_columns(vec!["revenue".to_string(), "costs".to_string()]);
    let formatted = ctx.format_error();

    assert!(formatted.contains("Available columns: revenue, costs"));
}

#[test]
fn test_formula_error_context_format_error_many_columns() {
    // When there are more than 10 columns, they shouldn't be shown
    let ctx = FormulaErrorContext::new("=A + B", "table.profit", "Error")
        .with_available_columns((0..15).map(|i| format!("col{}", i)).collect());
    let formatted = ctx.format_error();

    // Should not show available columns when more than 10
    assert!(!formatted.contains("Available columns:"));
}

#[test]
fn test_formula_error_helper_function() {
    let err = formula_error("=X + Y", "location", "error msg", None);
    match err {
        ForgeError::Formula(ctx) => {
            assert_eq!(ctx.formula, "=X + Y");
            assert_eq!(ctx.location, "location");
            assert_eq!(ctx.error, "error msg");
            assert!(ctx.suggestion.is_none());
        }
        _ => panic!("Expected ForgeError::Formula"),
    }
}

#[test]
fn test_formula_error_helper_with_suggestion() {
    let err = formula_error("=X + Y", "location", "error msg", Some("try this"));
    match err {
        ForgeError::Formula(ctx) => {
            assert_eq!(ctx.suggestion, Some("try this".to_string()));
        }
        _ => panic!("Expected ForgeError::Formula"),
    }
}

#[test]
fn test_forge_error_parse() {
    let err = ForgeError::Parse("Invalid syntax".to_string());
    assert_eq!(format!("{}", err), "Parse error: Invalid syntax");
}

#[test]
fn test_forge_error_eval() {
    let err = ForgeError::Eval("Division by zero".to_string());
    assert_eq!(
        format!("{}", err),
        "Formula evaluation error: Division by zero"
    );
}

#[test]
fn test_forge_error_circular_dependency() {
    let err = ForgeError::CircularDependency("A -> B -> A".to_string());
    assert_eq!(
        format!("{}", err),
        "Circular dependency detected: A -> B -> A"
    );
}

#[test]
fn test_forge_error_validation() {
    let err = ForgeError::Validation("Value out of range".to_string());
    assert_eq!(format!("{}", err), "Validation error: Value out of range");
}

#[test]
fn test_forge_error_export() {
    let err = ForgeError::Export("Failed to write cell".to_string());
    assert_eq!(
        format!("{}", err),
        "Excel export error: Failed to write cell"
    );
}

#[test]
fn test_forge_error_import() {
    let err = ForgeError::Import("Invalid file format".to_string());
    assert_eq!(
        format!("{}", err),
        "Excel import error: Invalid file format"
    );
}

#[test]
fn test_forge_error_io_string() {
    let err = ForgeError::IO("File not found".to_string());
    assert_eq!(format!("{}", err), "IO error: File not found");
}

#[test]
fn test_forge_error_formula_display() {
    let ctx = FormulaErrorContext::new("=BAD", "loc", "err");
    let err = ForgeError::Formula(ctx);
    let display = format!("{}", err);
    assert!(display.contains("Formula error"));
    assert!(display.contains("=BAD"));
}
